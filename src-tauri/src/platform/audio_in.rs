//! Audio input thread.
//!
//! One OS thread, created at start-up, owns every `cpal::Stream`, so a
//! stream never crosses threads and needs no `unsafe impl Send`. The
//! stream only exists while recording. The callback converts the device
//! format to `f32`, downmixes to mono into a stack buffer, and pushes into
//! a lock-free ring buffer. It never allocates, locks or logs.

use crate::core::audio::convert::{downmix_into, ToF32};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::traits::{Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

/// Device name meaning "the system default input".
pub const DEFAULT_DEVICE: &str = "default";
/// Ring buffer capacity in seconds of native-rate audio.
const RING_SECONDS: usize = 2;

/// What a successful start hands back to the controller.
pub struct Capture {
    pub consumer: HeapCons<f32>,
    pub native_rate: u32,
    pub device_name: String,
    /// The named device was missing and the default was used instead.
    pub used_fallback: bool,
}

/// Called from the stream's error callback (for example the device was
/// unplugged).
pub type ErrorFn = Box<dyn Fn(String) + Send + 'static>;

enum Cmd {
    Start {
        device: String,
        /// Set by the audio thread once the stream is gone, so the
        /// segmenter drains only after the last callback.
        stopped: Arc<AtomicBool>,
        on_error: ErrorFn,
        reply: mpsc::Sender<Result<Capture, String>>,
    },
    Stop,
    Shutdown,
}

pub struct AudioThread {
    tx: mpsc::Sender<Cmd>,
    join: Option<JoinHandle<()>>,
}

impl AudioThread {
    pub fn spawn() -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let join = std::thread::Builder::new()
            .name("audio-in".into())
            .spawn(move || run(rx))
            .map_err(|e| format!("could not start the audio thread: {e}"))?;
        Ok(Self {
            tx,
            join: Some(join),
        })
    }

    /// Open `device` and start streaming. Blocks until the stream runs or
    /// fails (at most 5 s).
    pub fn start(
        &self,
        device: &str,
        stopped: Arc<AtomicBool>,
        on_error: ErrorFn,
    ) -> Result<Capture, String> {
        let (reply, rx) = mpsc::channel();
        self.tx
            .send(Cmd::Start {
                device: device.to_string(),
                stopped,
                on_error,
                reply,
            })
            .map_err(|_| "the audio thread has stopped".to_string())?;
        rx.recv_timeout(Duration::from_secs(5))
            .map_err(|_| "the microphone did not start in time".to_string())?
    }

    /// Stop the current stream. The `stopped` flag given to `start` is set
    /// once the stream has been dropped.
    pub fn stop(&self) {
        let _ = self.tx.send(Cmd::Stop);
    }
}

impl Drop for AudioThread {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Shutdown);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// A running capture: a real stream, or the fake source in test builds.
/// Held only so that dropping it stops the capture.
#[allow(dead_code)]
enum Running {
    Stream(cpal::Stream),
    #[cfg(feature = "test-hooks")]
    Fake(fake::FakeFeed),
}

struct Active {
    _running: Running,
    stopped: Arc<AtomicBool>,
    overflow: Arc<AtomicU64>,
}

impl Active {
    fn finish(self) {
        let overflow = self.overflow.load(Ordering::Relaxed);
        let stopped = self.stopped.clone();
        // Dropping the stream joins the callback, so after this line no
        // more samples arrive.
        drop(self);
        stopped.store(true, Ordering::Release);
        if overflow > 0 {
            log::warn!("audio: ring buffer overflowed, {overflow} samples dropped");
        }
    }
}

fn run(rx: mpsc::Receiver<Cmd>) {
    let mut active: Option<Active> = None;
    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Start {
                device,
                stopped,
                on_error,
                reply,
            } => {
                if let Some(a) = active.take() {
                    a.finish();
                }
                match open(&device, stopped, on_error) {
                    Ok((a, capture)) => {
                        active = Some(a);
                        let _ = reply.send(Ok(capture));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            Cmd::Stop => {
                if let Some(a) = active.take() {
                    a.finish();
                }
            }
            Cmd::Shutdown => break,
        }
    }
    if let Some(a) = active.take() {
        a.finish();
    }
}

fn ring(rate: u32) -> (HeapProd<f32>, HeapCons<f32>) {
    HeapRb::<f32>::new(rate as usize * RING_SECONDS).split()
}

fn open(
    device: &str,
    stopped: Arc<AtomicBool>,
    on_error: ErrorFn,
) -> Result<(Active, Capture), String> {
    #[cfg(feature = "test-hooks")]
    if let Some(path) = std::env::var_os("LOCALYAPPER_FAKE_AUDIO") {
        let (feed, rate) = fake::FakeFeed::start(std::path::Path::new(&path))?;
        let (prod, cons) = ring(rate);
        feed.attach(prod);
        return Ok((
            Active {
                _running: Running::Fake(feed),
                stopped,
                overflow: Arc::new(AtomicU64::new(0)),
            },
            Capture {
                consumer: cons,
                native_rate: rate,
                device_name: "fake".into(),
                used_fallback: false,
            },
        ));
    }

    let host = cpal::default_host();
    let (dev, used_fallback) = find_device(&host, device)?;
    let device_name = dev.name().unwrap_or_else(|_| "unknown".into());
    let supported = dev
        .default_input_config()
        .map_err(|e| describe_error(&format!("could not read the microphone format: {e}")))?;
    let rate = supported.sample_rate().0;
    let channels = usize::from(supported.channels());
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let (prod, cons) = ring(rate);
    let overflow = Arc::new(AtomicU64::new(0));

    use cpal::SampleFormat as F;
    let stream = match format {
        F::F32 => build::<f32>(&dev, &config, channels, prod, overflow.clone(), on_error),
        F::I16 => build::<i16>(&dev, &config, channels, prod, overflow.clone(), on_error),
        F::U16 => build::<u16>(&dev, &config, channels, prod, overflow.clone(), on_error),
        F::I32 => build::<i32>(&dev, &config, channels, prod, overflow.clone(), on_error),
        F::U8 => build::<u8>(&dev, &config, channels, prod, overflow.clone(), on_error),
        F::I8 => build::<i8>(&dev, &config, channels, prod, overflow.clone(), on_error),
        other => return Err(format!("unsupported microphone sample format {other:?}")),
    }
    .map_err(|e| describe_error(&format!("could not open the microphone: {e}")))?;
    stream
        .play()
        .map_err(|e| describe_error(&format!("could not start the microphone: {e}")))?;
    log::info!("audio: capturing from '{device_name}' at {rate} Hz, {channels} ch, {format:?}");
    Ok((
        Active {
            _running: Running::Stream(stream),
            stopped,
            overflow,
        },
        Capture {
            consumer: cons,
            native_rate: rate,
            device_name,
            used_fallback,
        },
    ))
}

/// Where the OS lets the user grant microphone access.
#[cfg(windows)]
const PRIVACY_HINT: &str =
    "Allow microphone access in Settings > Privacy > Microphone (ms-settings:privacy-microphone).";
#[cfg(target_os = "macos")]
const PRIVACY_HINT: &str =
    "Allow microphone access in System Settings > Privacy & Security > Microphone.";
#[cfg(not(any(windows, target_os = "macos")))]
const PRIVACY_HINT: &str = "Check that no other app holds the microphone.";

/// Add a hint for the OS privacy setting when access was refused.
pub fn describe_error(msg: &str) -> String {
    let lower = msg.to_lowercase();
    if lower.contains("denied") || lower.contains("permission") || lower.contains("access") {
        format!("{msg}. {PRIVACY_HINT}")
    } else {
        msg.to_string()
    }
}

fn find_device(host: &cpal::Host, name: &str) -> Result<(cpal::Device, bool), String> {
    if name != DEFAULT_DEVICE && !name.is_empty() {
        if let Ok(mut devices) = host.input_devices() {
            if let Some(d) = devices.find(|d| d.name().map(|n| n == name).unwrap_or(false)) {
                return Ok((d, false));
            }
        }
        log::warn!("audio: input device '{name}' not found, using the default");
        let d = host.default_input_device().ok_or("no microphone found")?;
        return Ok((d, true));
    }
    host.default_input_device()
        .map(|d| (d, false))
        .ok_or_else(|| "no microphone found".to_string())
}

/// Frames converted per inner step of the callback.
const CHUNK: usize = 1024;

fn build<T>(
    dev: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    mut prod: HeapProd<f32>,
    overflow: Arc<AtomicU64>,
    on_error: ErrorFn,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample + ToF32,
{
    dev.build_input_stream(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            let mut buf = [0.0f32; CHUNK];
            for chunk in data.chunks(CHUNK * channels) {
                let n = downmix_into(chunk, channels, &mut buf);
                let pushed = prod.push_slice(&buf[..n]);
                if pushed < n {
                    overflow.fetch_add((n - pushed) as u64, Ordering::Relaxed);
                }
            }
        },
        move |err| on_error(err.to_string()),
        None,
    )
}

/// Names of the available input devices, default first.
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    let default = host.default_input_device().and_then(|d| d.name().ok());
    let mut names: Vec<String> = host
        .input_devices()
        .map(|it| it.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default();
    names.sort();
    names.dedup();
    if let Some(def) = default {
        names.retain(|n| *n != def);
        names.insert(0, def);
    }
    names
}

#[cfg(feature = "test-hooks")]
mod fake {
    //! `LOCALYAPPER_FAKE_AUDIO=/path/to.wav` plays a file into the ring
    //! buffer at real-time speed instead of opening a device, then
    //! silence. Test builds only.
    use super::*;
    use crate::core::audio::wav;
    use std::sync::Mutex;

    pub struct FakeFeed {
        stop: Arc<AtomicBool>,
        slot: Arc<Mutex<Option<HeapProd<f32>>>>,
        join: Option<JoinHandle<()>>,
    }

    impl FakeFeed {
        pub fn start(path: &std::path::Path) -> Result<(Self, u32), String> {
            let data = wav::read_file(path)?;
            let rate = data.sample_rate;
            let mono = data.to_mono();
            let stop = Arc::new(AtomicBool::new(false));
            let slot: Arc<Mutex<Option<HeapProd<f32>>>> = Arc::new(Mutex::new(None));
            let (s2, slot2) = (stop.clone(), slot.clone());
            let join = std::thread::Builder::new()
                .name("fake-audio".into())
                .spawn(move || {
                    let step = (rate / 100) as usize; // 10 ms
                    let start = std::time::Instant::now();
                    let mut sent = 0usize;
                    let silence = vec![0.0f32; step];
                    while !s2.load(Ordering::Acquire) {
                        let due = start.elapsed().as_millis() as usize * rate as usize / 1000;
                        while sent + step <= due {
                            let block = if sent + step <= mono.len() {
                                &mono[sent..sent + step]
                            } else {
                                &silence[..]
                            };
                            if let Ok(mut g) = slot2.lock() {
                                if let Some(p) = g.as_mut() {
                                    p.push_slice(block);
                                }
                            }
                            sent += step;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                })
                .map_err(|e| e.to_string())?;
            Ok((
                Self {
                    stop,
                    slot,
                    join: Some(join),
                },
                rate,
            ))
        }

        pub fn attach(&self, prod: HeapProd<f32>) {
            if let Ok(mut g) = self.slot.lock() {
                *g = Some(prod);
            }
        }
    }

    impl Drop for FakeFeed {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            if let Some(j) = self.join.take() {
                let _ = j.join();
            }
        }
    }
}
