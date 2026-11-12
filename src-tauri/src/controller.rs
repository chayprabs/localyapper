//! The controller: the only owner of dictation state.
//!
//! One tokio task with an unbounded inbox. Every unit (hotkeys, the audio
//! thread, segmenters, the STT worker, timers, IPC) sends it a `Msg`. It
//! feeds session events to `Session::handle` and executes the returned
//! `Effect`s in order (plan section 4.3).

use crate::core::audio::segmenter::{self, EnergyDetector, Flags, SileroDetector, SpeechDetector};
use crate::core::session::{self, Cue, Effect, Event, Session, SessionId, TimerKind};
use crate::core::stt::manager::{LoadOutcome, ModelManager, ModelSpec, SttWorker, WorkerEvent};
use crate::platform::audio_in::{AudioThread, Capture};
use crate::platform::hotkey::shortcut::{self, HotkeyId};
use crate::platform::memory;
use crate::platform::overlay::OverlayBackend;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

/// Everything the controller can be told.
#[derive(Debug)]
pub enum Msg {
    Session(Event),
    Worker(WorkerEvent),
    CaptureStarted {
        id: SessionId,
        device: String,
        used_fallback: bool,
    },
    EvictTimer {
        generation: u64,
    },
    PasteLast,
    SetPaused(bool),
    /// Settings changed in the database: re-read them.
    ReloadSettings,
    /// Read the state for diagnostics and the test channel.
    Query(tokio::sync::oneshot::Sender<Snapshot>),
}

/// Stage timings of one dictation, in milliseconds. Never holds text.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct StageTimings {
    pub capture_start_ms: u64,
    pub model_ready_ms: u64,
    pub segments: usize,
    pub audio_ms: u64,
    pub decode_ms: u64,
    pub tail_decode_ms: u64,
    pub inject_ms: u64,
    pub release_to_text_ms: u64,
}

/// What `Msg::Query` returns.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Snapshot {
    pub idle: bool,
    pub recording: bool,
    pub paused: bool,
    pub model_loaded: Option<String>,
    pub last_view: Option<crate::platform::overlay::ViewPayload>,
    pub last_timings: Option<StageTimings>,
    pub dictations: u64,
}

/// Cheap, cloneable sender used by every producer.
#[derive(Clone, Debug)]
pub struct ControllerHandle {
    tx: UnboundedSender<Msg>,
    epoch: Instant,
}

impl ControllerHandle {
    pub fn send(&self, msg: Msg) {
        let _ = self.tx.send(msg);
    }

    /// Milliseconds since start-up, the clock for `RecordDown`/`RecordUp`.
    pub fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    pub async fn query(&self) -> Option<Snapshot> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.send(Msg::Query(tx));
        tokio::time::timeout(Duration::from_secs(5), rx)
            .await
            .ok()?
            .ok()
    }
}

/// Settings the controller reads. Re-read on `Msg::ReloadSettings`.
#[derive(Clone, Debug, PartialEq)]
pub struct ControllerSettings {
    pub session: session::Config,
    pub input_device: String,
    pub idle_unload_seconds: u64,
    pub trailing_space: bool,
    pub hotkey_cancel: String,
    pub sound_cues: bool,
    pub overlay_enabled: bool,
}

impl Default for ControllerSettings {
    fn default() -> Self {
        Self {
            session: session::Config::default(),
            input_device: crate::platform::audio_in::DEFAULT_DEVICE.into(),
            idle_unload_seconds: 120,
            trailing_space: true,
            hotkey_cancel: "Escape".into(),
            sound_cues: false,
            overlay_enabled: true,
        }
    }
}

impl ControllerSettings {
    /// Build from `key -> value` pairs; unknown or bad values keep defaults.
    pub fn from_map(map: &HashMap<String, String>) -> Self {
        let mut s = Self::default();
        let get = |k: &str| map.get(k).map(String::as_str);
        let flag = |k: &str, d: bool| get(k).map_or(d, |v| v.eq_ignore_ascii_case("true"));
        s.session.mode = match get("activation_mode") {
            Some("hold") => session::ActivationMode::Hold,
            Some("toggle") => session::ActivationMode::Toggle,
            _ => session::ActivationMode::Hybrid,
        };
        let max_s = get("max_recording_seconds")
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| (30..=3600).contains(v))
            .unwrap_or(600);
        s.session.max_recording_ms = max_s * 1000;
        s.session.cancel_key_enabled = flag("cancel_key_enabled", true);
        s.session.cancel_key_in_hands_free = flag("cancel_key_in_hands_free", false);
        s.session.history_enabled = flag("history_enabled", true);
        if let Some(d) = get("input_device").filter(|d| !d.is_empty()) {
            s.input_device = d.to_string();
        }
        s.idle_unload_seconds = get("idle_unload_seconds")
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| [0, 30, 120, 300, 900].contains(v))
            .unwrap_or(120);
        s.trailing_space = flag("trailing_space", true);
        if let Some(c) = get("hotkey_cancel").filter(|c| !c.is_empty()) {
            s.hotkey_cancel = c.to_string();
        }
        s.sound_cues = flag("sound_cues", false);
        s.overlay_enabled = flag("overlay_enabled", true);
        s
    }
}

/// Flags for one recording's capture.
struct CaptureFlags {
    stopped: Arc<AtomicBool>,
    discard: Arc<AtomicBool>,
}

/// Timestamps for the stage-timing log line.
#[derive(Default)]
struct Timing {
    pressed: Option<Instant>,
    capture_started: Option<Instant>,
    model_ready: Option<Instant>,
    released: Option<Instant>,
    decode_ms: u64,
    tail_decode_ms: u64,
    segments: usize,
    audio_ms: u64,
    inject_started: Option<Instant>,
}

fn ms_between(a: Option<Instant>, b: Option<Instant>) -> u64 {
    match (a, b) {
        (Some(a), Some(b)) => b.saturating_duration_since(a).as_millis() as u64,
        _ => 0,
    }
}

/// Resolves the configured speech model to files on disk.
pub type ModelResolver = Box<dyn Fn(&AppHandle) -> Result<ModelSpec, String> + Send>;

pub struct Controller {
    app: AppHandle,
    handle: ControllerHandle,
    session: Session,
    settings: ControllerSettings,
    audio: Arc<AudioThread>,
    worker: Arc<SttWorker>,
    overlay: Arc<dyn OverlayBackend>,
    resolve_model: ModelResolver,
    vad_model: Option<PathBuf>,
    captures: HashMap<SessionId, CaptureFlags>,
    current_id: Option<SessionId>,
    timing: Timing,
    target_app: String,
    /// The cancel combo registered now, if any. Never re-read from
    /// settings while registered, so unregister always matches.
    cancel_registered: Option<String>,
    evict_generation: u64,
    rss_before_evict: Option<u64>,
    paused: Arc<AtomicBool>,
    last_injection: Arc<Mutex<Option<String>>>,
    last_view: Option<crate::platform::overlay::ViewPayload>,
    last_timings: Option<StageTimings>,
    dictations: u64,
}

/// Shared pieces the rest of the app needs.
pub struct Started {
    pub handle: ControllerHandle,
    pub manager: Arc<ModelManager>,
    pub paused: Arc<AtomicBool>,
    pub last_injection: Arc<Mutex<Option<String>>>,
}

pub struct ControllerParts {
    pub app: AppHandle,
    pub settings: ControllerSettings,
    pub overlay: Arc<dyn OverlayBackend>,
    pub resolve_model: ModelResolver,
    pub vad_model: Option<PathBuf>,
}

/// Create the controller and start its task.
pub fn start(parts: ControllerParts) -> Result<Started, String> {
    let (tx, rx) = unbounded_channel();
    let handle = ControllerHandle {
        tx,
        epoch: Instant::now(),
    };
    let manager = Arc::new(ModelManager::new(memory::stt_threads()));
    let worker_handle = handle.clone();
    let worker = Arc::new(SttWorker::spawn(
        manager.clone(),
        Arc::new(move |ev| worker_handle.send(Msg::Worker(ev))),
    )?);
    let audio = Arc::new(AudioThread::spawn()?);
    let paused = Arc::new(AtomicBool::new(false));
    let last_injection = Arc::new(Mutex::new(None));
    let ctl = Controller {
        app: parts.app,
        handle: handle.clone(),
        session: Session::new(parts.settings.session.clone()),
        settings: parts.settings,
        audio,
        worker,
        overlay: parts.overlay,
        resolve_model: parts.resolve_model,
        vad_model: parts.vad_model,
        captures: HashMap::new(),
        current_id: None,
        timing: Timing::default(),
        target_app: String::new(),
        cancel_registered: None,
        evict_generation: 0,
        rss_before_evict: None,
        paused: paused.clone(),
        last_injection: last_injection.clone(),
        last_view: None,
        last_timings: None,
        dictations: 0,
    };
    tauri::async_runtime::spawn(ctl.run(rx));
    log::info!("controller: started, {} STT threads", manager.threads());
    Ok(Started {
        handle,
        manager,
        paused,
        last_injection,
    })
}

impl Controller {
    async fn run(mut self, mut rx: UnboundedReceiver<Msg>) {
        while let Some(msg) = rx.recv().await {
            self.on_msg(msg);
        }
    }

    fn on_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Session(Event::InjectDone { id, result }) => self.on_inject_done(id, result),
            Msg::Session(ev) => self.feed(ev),
            Msg::Worker(ev) => self.on_worker(ev),
            Msg::CaptureStarted {
                id,
                device,
                used_fallback,
            } => {
                if self.current_id == Some(id) {
                    self.timing.capture_started = Some(Instant::now());
                }
                if used_fallback {
                    let _ = self.app.emit(
                        "audio-device-warning",
                        format!("The selected microphone was not found. Using {device}."),
                    );
                }
            }
            Msg::EvictTimer { generation } => {
                if generation == self.evict_generation && self.session.is_idle() {
                    self.rss_before_evict = memory::resident_bytes();
                    self.worker.evict();
                }
            }
            Msg::PasteLast => self.paste_last(),
            Msg::SetPaused(p) => {
                self.paused.store(p, Ordering::SeqCst);
                self.feed(Event::SetPaused(p));
                let _ = self.app.emit("paused-state-changed", p);
            }
            Msg::ReloadSettings => self.reload_settings(),
            Msg::Query(reply) => {
                let _ = reply.send(Snapshot {
                    idle: self.session.is_idle(),
                    recording: self.session.is_recording(),
                    paused: self.paused.load(Ordering::SeqCst),
                    model_loaded: self.worker.manager().loaded_id(),
                    last_view: self.last_view.clone(),
                    last_timings: self.last_timings.clone(),
                    dictations: self.dictations,
                });
            }
        }
    }

    fn reload_settings(&mut self) {
        let map = self
            .app
            .state::<crate::state::AppState>()
            .db
            .lock()
            .ok()
            .and_then(|c| crate::db::queries::get_all_settings(&c).ok())
            .unwrap_or_default();
        let next = ControllerSettings::from_map(&map);
        if next.session != self.settings.session {
            self.feed(Event::ConfigChanged(next.session.clone()));
        }
        self.settings = next;
    }

    /// Feed one event to the session and execute its effects in order.
    fn feed(&mut self, ev: Event) {
        match &ev {
            Event::RecordDown { .. } if self.session.is_idle() => {
                self.timing = Timing {
                    pressed: Some(Instant::now()),
                    ..Timing::default()
                };
            }
            Event::CaptureEnded {
                id,
                segments,
                audio_ms,
            } if self.current_id == Some(*id) => {
                self.timing.segments = *segments;
                self.timing.audio_ms = *audio_ms;
                self.captures.remove(id);
            }
            _ => {}
        }
        let effects = self.session.handle(ev);
        for fx in effects {
            self.execute(fx);
        }
    }

    fn execute(&mut self, fx: Effect) {
        match fx {
            Effect::StartCapture { id } => self.start_capture(id),
            Effect::EnsureModelLoaded => self.ensure_model_loaded(),
            Effect::StopCapture { id } => {
                if self.current_id == Some(id) {
                    self.timing.released = Some(Instant::now());
                }
                self.audio.stop();
            }
            Effect::DiscardCapture { id } => {
                if let Some(f) = self.captures.remove(&id) {
                    f.discard.store(true, Ordering::Release);
                }
                self.worker.discard(id);
                self.audio.stop();
            }
            Effect::Overlay(view) => {
                self.last_view = Some((&view).into());
                if self.settings.overlay_enabled || view == session::View::Hidden {
                    self.overlay.show(&view);
                }
            }
            Effect::Tray(t) => crate::tray::set_state(&self.app, t),
            Effect::Cue(c) => self.cue(c),
            Effect::RegisterCancelKey => {
                let combo = self.settings.hotkey_cancel.clone();
                match shortcut::register(&self.app, HotkeyId::Cancel, &combo, &self.handle) {
                    Ok(()) => self.cancel_registered = Some(combo),
                    Err(e) => log::warn!("controller: {e}"),
                }
            }
            Effect::UnregisterCancelKey => {
                if let Some(combo) = self.cancel_registered.take() {
                    shortcut::unregister(&self.app, &combo);
                }
            }
            Effect::StartTimer { kind, id, after_ms } => self.start_timer(kind, id, after_ms),
            Effect::Inject { id, text } => self.inject(id, text),
            Effect::SaveHistory { text, audio_ms } => self.save_history(&text, audio_ms),
            Effect::ScheduleEvict => self.schedule_evict(),
        }
    }

    fn start_capture(&mut self, id: SessionId) {
        // The target app is read before any overlay call.
        self.target_app = crate::context::detector::get_focused_window_name();
        self.current_id = Some(id);
        let flags = CaptureFlags {
            stopped: Arc::new(AtomicBool::new(false)),
            discard: Arc::new(AtomicBool::new(false)),
        };
        let stopped = flags.stopped.clone();
        let discard = flags.discard.clone();
        self.captures.insert(id, flags);

        let audio = self.audio.clone();
        let worker = self.worker.clone();
        let overlay = self.overlay.clone();
        let handle = self.handle.clone();
        let device = self.settings.input_device.clone();
        let vad_model = self.vad_model.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let err_handle = handle.clone();
            let on_error = Box::new(move |message: String| {
                err_handle.send(Msg::Session(Event::CaptureFailed { id, message }));
            });
            let cap = match audio.start(&device, stopped.clone(), on_error) {
                Ok(cap) => cap,
                Err(message) => {
                    handle.send(Msg::Session(Event::CaptureFailed { id, message }));
                    return;
                }
            };
            if discard.load(Ordering::Acquire) {
                audio.stop();
                return;
            }
            handle.send(Msg::CaptureStarted {
                id,
                device: cap.device_name.clone(),
                used_fallback: cap.used_fallback,
            });
            spawn_segmenter(
                id, cap, stopped, discard, vad_model, worker, overlay, handle,
            );
        });
    }

    fn ensure_model_loaded(&mut self) {
        // A pending eviction must not drop the model we are about to use.
        self.evict_generation += 1;
        match (self.resolve_model)(&self.app) {
            Ok(spec) => {
                if self.worker.manager().loaded_id().as_deref() != Some(spec.id.as_str()) {
                    emit_model_state(&self.app, "loading");
                }
                self.worker.load(spec);
            }
            Err(message) => self.feed(Event::ModelFailed { message }),
        }
    }

    fn on_worker(&mut self, ev: WorkerEvent) {
        match ev {
            WorkerEvent::Loaded { model, outcome } => {
                if self.timing.model_ready.is_none() {
                    self.timing.model_ready = Some(Instant::now());
                }
                if let LoadOutcome::Loaded { load_ms, warm_ms } = outcome {
                    log::info!(
                        "stt: loaded {model} in {load_ms} ms, warm-up {warm_ms} ms, rss {} MB",
                        memory::resident_mb()
                    );
                }
                emit_model_state(&self.app, "loaded");
            }
            WorkerEvent::LoadFailed { message } => {
                log::error!("stt: load failed: {message}");
                emit_model_state(&self.app, "unloaded");
                self.feed(Event::ModelFailed { message });
            }
            WorkerEvent::Text {
                id,
                index,
                text,
                decode_ms,
            } => {
                self.note_decode(id, decode_ms);
                self.feed(Event::SegmentText { id, index, text });
            }
            WorkerEvent::DecodeFailed { id, index, message } => {
                log::warn!("stt: segment {index} of session {id} not decoded: {message}");
                self.feed(Event::SegmentText {
                    id,
                    index,
                    text: String::new(),
                });
            }
            WorkerEvent::Evicted { was_loaded } => {
                if was_loaded {
                    let before = self.rss_before_evict.take();
                    memory::trim();
                    let after = memory::resident_bytes();
                    let mb = |b: Option<u64>| {
                        b.map_or("n/a".into(), |b| format!("{:.1}", b as f64 / 1_048_576.0))
                    };
                    log::info!(
                        "stt: model evicted, rss {} MB -> {} MB",
                        mb(before),
                        mb(after)
                    );
                    emit_model_state(&self.app, "unloaded");
                }
            }
        }
    }

    fn note_decode(&mut self, id: SessionId, decode_ms: u64) {
        if self.current_id != Some(id) {
            return;
        }
        self.timing.decode_ms += decode_ms;
        if self.timing.released.is_some() {
            self.timing.tail_decode_ms += decode_ms;
        }
    }

    fn start_timer(&self, kind: TimerKind, id: SessionId, after_ms: u64) {
        let handle = self.handle.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_millis(after_ms)).await;
            handle.send(Msg::Session(Event::Timer { kind, id }));
        });
    }

    fn schedule_evict(&mut self) {
        self.evict_generation += 1;
        let secs = self.settings.idle_unload_seconds;
        if secs == 0 {
            return;
        }
        let generation = self.evict_generation;
        let handle = self.handle.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(secs)).await;
            handle.send(Msg::EvictTimer { generation });
        });
    }

    fn cue(&self, cue: Cue) {
        if self.settings.sound_cues {
            log::debug!("controller: cue {cue:?}");
        }
    }

    fn inject(&mut self, id: SessionId, text: String) {
        self.timing.inject_started = Some(Instant::now());
        if let Ok(mut last) = self.last_injection.lock() {
            *last = Some(text.clone());
        }
        let payload = if self.settings.trailing_space {
            format!("{text} ")
        } else {
            text
        };
        let handle = self.handle.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let result = crate::injection::injector::inject(&payload, false);
            handle.send(Msg::Session(Event::InjectDone { id, result }));
        });
    }

    fn paste_last(&self) {
        let text = self.last_injection.lock().ok().and_then(|g| g.clone());
        let Some(text) = text.filter(|t| !t.is_empty()) else {
            return;
        };
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(e) = crate::injection::injector::inject(&text, false) {
                log::warn!("paste last: {e}");
            }
        });
    }

    fn save_history(&self, text: &str, audio_ms: u64) {
        let entry = crate::models::HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            raw_text: text.to_string(),
            final_text: text.to_string(),
            app_name: Some(self.target_app.clone()),
            duration_ms: Some(audio_ms as i64),
            word_count: Some(text.split_whitespace().count() as i64),
            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        };
        let state = self.app.state::<crate::state::AppState>();
        let result = match state.db.lock() {
            Ok(conn) => {
                crate::db::queries::insert_history(&conn, &entry).map_err(|e| e.to_string())
            }
            Err(e) => Err(e.to_string()),
        };
        if let Err(e) = result {
            log::warn!("history: could not save: {e}");
        }
        let _ = self.app.emit("history-updated", ());
    }

    /// Closes the stage timings when the paste of the current session ends.
    fn finish_timings(&mut self) {
        let now = Some(Instant::now());
        let t = &self.timing;
        let timings = StageTimings {
            capture_start_ms: ms_between(t.pressed, t.capture_started),
            model_ready_ms: ms_between(t.pressed, t.model_ready),
            segments: t.segments,
            audio_ms: t.audio_ms,
            decode_ms: t.decode_ms,
            tail_decode_ms: t.tail_decode_ms,
            inject_ms: ms_between(t.inject_started, now),
            release_to_text_ms: ms_between(t.released, now),
        };
        log::info!(
            "dictation: capture_start_ms={} model_ready_ms={} segments={} audio_ms={} decode_ms={} tail_decode_ms={} inject_ms={} release_to_text_ms={}",
            timings.capture_start_ms,
            timings.model_ready_ms,
            timings.segments,
            timings.audio_ms,
            timings.decode_ms,
            timings.tail_decode_ms,
            timings.inject_ms,
            timings.release_to_text_ms
        );
        let _ = self.app.emit("dictation-timings", &timings);
        self.last_timings = Some(timings);
        self.dictations += 1;
    }

    /// Wraps `feed` for `InjectDone` so timings are closed exactly once.
    fn on_inject_done(&mut self, id: SessionId, result: Result<(), String>) {
        if self.current_id == Some(id) {
            self.finish_timings();
        }
        self.feed(Event::InjectDone { id, result });
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_segmenter(
    id: SessionId,
    cap: Capture,
    stopped: Arc<AtomicBool>,
    discard: Arc<AtomicBool>,
    vad_model: Option<PathBuf>,
    worker: Arc<SttWorker>,
    overlay: Arc<dyn OverlayBackend>,
    handle: ControllerHandle,
) {
    let spawned = std::thread::Builder::new()
        .name(format!("segmenter-{id}"))
        .spawn(move || {
            let Capture {
                mut consumer,
                native_rate,
                ..
            } = cap;
            let mut vad: Box<dyn SpeechDetector> =
                match vad_model.as_deref().map(SileroDetector::new) {
                    Some(Ok(v)) => Box::new(v),
                    Some(Err(e)) => {
                        log::warn!("vad: {e}; using the energy detector");
                        Box::new(EnergyDetector::new())
                    }
                    None => Box::new(EnergyDetector::new()),
                };
            let events = |ev: Event| handle.send(Msg::Session(ev));
            let result = segmenter::run_segmenter(
                id,
                native_rate,
                &mut consumer,
                Flags {
                    stop: &stopped,
                    discard: &discard,
                },
                vad.as_mut(),
                worker.as_ref(),
                &events,
                &mut |v| overlay.level(v),
            );
            if let Err(message) = result {
                handle.send(Msg::Session(Event::CaptureFailed { id, message }));
            }
        });
    if let Err(e) = spawned {
        log::error!("controller: could not start the segmenter: {e}");
    }
}

fn emit_model_state(app: &AppHandle, state: &str) {
    #[derive(Serialize, Clone)]
    struct Payload<'a> {
        loaded: bool,
        state: &'a str,
    }
    let _ = app.emit(
        "model-state",
        Payload {
            loaded: state == "loaded",
            state,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_parse_with_defaults_and_ranges() {
        let s = ControllerSettings::from_map(&HashMap::new());
        assert_eq!(s, ControllerSettings::default());
        let map: HashMap<String, String> = [
            ("activation_mode", "toggle"),
            ("max_recording_seconds", "45"),
            ("idle_unload_seconds", "0"),
            ("cancel_key_in_hands_free", "true"),
            ("history_enabled", "false"),
            ("input_device", "USB Mic"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        let s = ControllerSettings::from_map(&map);
        assert_eq!(s.session.mode, session::ActivationMode::Toggle);
        assert_eq!(s.session.max_recording_ms, 45_000);
        assert_eq!(s.idle_unload_seconds, 0);
        assert!(s.session.cancel_key_in_hands_free);
        assert!(!s.session.history_enabled);
        assert_eq!(s.input_device, "USB Mic");
        let bad: HashMap<String, String> =
            [("max_recording_seconds", "5"), ("idle_unload_seconds", "7")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        let s = ControllerSettings::from_map(&bad);
        assert_eq!(s.session.max_recording_ms, 600_000);
        assert_eq!(s.idle_unload_seconds, 120);
    }

    #[test]
    fn ms_between_handles_missing_marks() {
        let a = Instant::now();
        assert_eq!(ms_between(None, Some(a)), 0);
        assert_eq!(ms_between(Some(a), None), 0);
        assert_eq!(ms_between(Some(a), Some(a)), 0);
    }
}
