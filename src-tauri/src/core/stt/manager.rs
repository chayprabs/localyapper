//! Model lifecycle and the STT worker thread.
//!
//! `ModelManager` loads a recogniser at most once at a time (a
//! `tokio::sync::Mutex` serialises loads), warms it up, and drops it on
//! eviction. `SttWorker` is one OS thread with a queue: load requests,
//! segments to decode, and evictions run strictly in the order sent, so a
//! segment can never race an eviction.

use super::engine::{Family, ModelFiles, SherpaEngine};
use crate::core::audio::segmenter::SegmentSink;
use crate::core::session::SessionId;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Instant;

/// Everything needed to load one model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelSpec {
    pub id: String,
    pub family: Family,
    pub files: ModelFiles,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LoadOutcome {
    AlreadyLoaded,
    Loaded { load_ms: u64, warm_ms: u64 },
}

pub struct ModelManager {
    load_lock: tokio::sync::Mutex<()>,
    current: RwLock<Option<(String, Arc<SherpaEngine>)>>,
    threads: i32,
}

impl ModelManager {
    pub fn new(threads: i32) -> Self {
        Self {
            load_lock: tokio::sync::Mutex::new(()),
            current: RwLock::new(None),
            threads: threads.max(1),
        }
    }

    pub fn threads(&self) -> i32 {
        self.threads
    }

    /// Id of the loaded model, if any.
    pub fn loaded_id(&self) -> Option<String> {
        self.current
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|(id, _)| id.clone()))
    }

    pub fn engine(&self) -> Option<Arc<SherpaEngine>> {
        self.current
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|(_, e)| e.clone()))
    }

    /// Load `spec` unless it is already loaded. Idempotent; concurrent
    /// callers wait for the one load in progress. Blocking: call from a
    /// worker thread or `spawn_blocking`.
    pub fn ensure_loaded_blocking(&self, spec: &ModelSpec) -> Result<LoadOutcome, String> {
        let _guard = self.load_lock.blocking_lock();
        if self.loaded_id().as_deref() == Some(spec.id.as_str()) {
            return Ok(LoadOutcome::AlreadyLoaded);
        }
        // A different model is loaded: drop it before loading the new one
        // so two models never sit in memory together.
        self.evict_locked();
        let t = Instant::now();
        let engine = SherpaEngine::load_files(spec.family, &spec.files, self.threads)?;
        let load_ms = t.elapsed().as_millis() as u64;
        let t = Instant::now();
        engine.warm_up();
        let warm_ms = t.elapsed().as_millis() as u64;
        if let Ok(mut g) = self.current.write() {
            *g = Some((spec.id.clone(), Arc::new(engine)));
        }
        Ok(LoadOutcome::Loaded { load_ms, warm_ms })
    }

    fn evict_locked(&self) -> bool {
        match self.current.write() {
            Ok(mut g) => g.take().is_some(),
            Err(_) => false,
        }
    }

    /// Drop the recogniser. Returns whether one was loaded.
    pub fn evict_blocking(&self) -> bool {
        let _guard = self.load_lock.blocking_lock();
        self.evict_locked()
    }
}

/// What the worker reports back.
#[derive(Clone, Debug, PartialEq)]
pub enum WorkerEvent {
    Loaded {
        model: String,
        outcome: LoadOutcome,
    },
    LoadFailed {
        message: String,
    },
    Text {
        id: SessionId,
        index: usize,
        text: String,
        decode_ms: u64,
    },
    /// A segment could not be decoded (no model, or a decode error). The
    /// text is empty so the session can still complete.
    DecodeFailed {
        id: SessionId,
        index: usize,
        message: String,
    },
    Evicted {
        was_loaded: bool,
    },
}

enum Job {
    Load(Box<ModelSpec>),
    Decode {
        id: SessionId,
        index: usize,
        samples: Vec<f32>,
    },
    Evict,
    Shutdown,
}

pub type WorkerCallback = Arc<dyn Fn(WorkerEvent) + Send + Sync>;

pub struct SttWorker {
    tx: mpsc::Sender<Job>,
    /// Jobs for sessions below this id are dropped unread.
    min_live_id: Arc<AtomicU64>,
    manager: Arc<ModelManager>,
    join: Option<JoinHandle<()>>,
}

impl SttWorker {
    pub fn spawn(manager: Arc<ModelManager>, on_event: WorkerCallback) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<Job>();
        let min_live_id = Arc::new(AtomicU64::new(0));
        let (m, live) = (manager.clone(), min_live_id.clone());
        let join = std::thread::Builder::new()
            .name("stt-worker".into())
            .spawn(move || worker_loop(rx, m, live, on_event))
            .map_err(|e| format!("could not start the speech worker: {e}"))?;
        Ok(Self {
            tx,
            min_live_id,
            manager,
            join: Some(join),
        })
    }

    pub fn manager(&self) -> &Arc<ModelManager> {
        &self.manager
    }

    /// Queue a load. Segments queued after it decode with this model.
    pub fn load(&self, spec: ModelSpec) {
        let _ = self.tx.send(Job::Load(Box::new(spec)));
    }

    /// Drop everything queued for `id`.
    pub fn discard(&self, id: SessionId) {
        self.min_live_id.fetch_max(id + 1, Ordering::AcqRel);
    }

    /// Queue an eviction behind any pending work.
    pub fn evict(&self) {
        let _ = self.tx.send(Job::Evict);
    }
}

impl SegmentSink for SttWorker {
    fn push(&self, id: SessionId, index: usize, samples: Vec<f32>) {
        let _ = self.tx.send(Job::Decode { id, index, samples });
    }
}

impl Drop for SttWorker {
    fn drop(&mut self) {
        let _ = self.tx.send(Job::Shutdown);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

fn worker_loop(
    rx: mpsc::Receiver<Job>,
    manager: Arc<ModelManager>,
    min_live_id: Arc<AtomicU64>,
    on_event: WorkerCallback,
) {
    while let Ok(job) = rx.recv() {
        match job {
            Job::Load(spec) => match manager.ensure_loaded_blocking(&spec) {
                Ok(outcome) => on_event(WorkerEvent::Loaded {
                    model: spec.id,
                    outcome,
                }),
                Err(message) => on_event(WorkerEvent::LoadFailed { message }),
            },
            Job::Decode { id, index, samples } => {
                if id < min_live_id.load(Ordering::Acquire) {
                    continue;
                }
                let Some(engine) = manager.engine() else {
                    on_event(WorkerEvent::DecodeFailed {
                        id,
                        index,
                        message: "no speech model is loaded".into(),
                    });
                    continue;
                };
                let t = Instant::now();
                match engine.transcribe(&samples) {
                    Ok(text) => on_event(WorkerEvent::Text {
                        id,
                        index,
                        text,
                        decode_ms: t.elapsed().as_millis() as u64,
                    }),
                    Err(message) => on_event(WorkerEvent::DecodeFailed { id, index, message }),
                }
            }
            Job::Evict => {
                let was_loaded = manager.evict_blocking();
                on_event(WorkerEvent::Evicted { was_loaded });
            }
            Job::Shutdown => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::Duration;

    fn bogus_spec() -> ModelSpec {
        let dir = std::path::Path::new("/nonexistent-localyapper-model");
        ModelSpec {
            id: "missing".into(),
            family: Family::NemoCtc,
            files: ModelFiles {
                tokens: dir.join("tokens.txt"),
                model: Some(dir.join("model.onnx")),
                ..Default::default()
            },
        }
    }

    fn worker() -> (SttWorker, Arc<Mutex<Vec<WorkerEvent>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let e2 = events.clone();
        let w = SttWorker::spawn(
            Arc::new(ModelManager::new(1)),
            Arc::new(move |ev| {
                if let Ok(mut v) = e2.lock() {
                    v.push(ev);
                }
            }),
        )
        .expect("spawn");
        (w, events)
    }

    fn wait_for(events: &Mutex<Vec<WorkerEvent>>, n: usize) -> Vec<WorkerEvent> {
        for _ in 0..200 {
            if let Ok(v) = events.lock() {
                if v.len() >= n {
                    return v.clone();
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        events.lock().map(|v| v.clone()).unwrap_or_default()
    }

    #[test]
    fn missing_model_reports_failure_and_segments_still_answer() {
        let (w, events) = worker();
        w.load(bogus_spec());
        w.push(3, 0, vec![0.0; 16_000]);
        w.evict();
        let got = wait_for(&events, 3);
        assert!(matches!(got[0], WorkerEvent::LoadFailed { .. }));
        assert!(matches!(
            got[1],
            WorkerEvent::DecodeFailed {
                id: 3,
                index: 0,
                ..
            }
        ));
        assert_eq!(got[2], WorkerEvent::Evicted { was_loaded: false });
        assert!(w.manager().loaded_id().is_none());
    }

    #[test]
    fn discarded_sessions_are_skipped() {
        let (w, events) = worker();
        w.discard(5);
        w.push(5, 0, vec![0.0; 100]);
        w.push(6, 0, vec![0.0; 100]);
        let got = wait_for(&events, 1);
        assert_eq!(got.len(), 1);
        assert!(matches!(got[0], WorkerEvent::DecodeFailed { id: 6, .. }));
    }
}
