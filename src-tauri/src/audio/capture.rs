// Audio capture module -- cpal-based recording with resampling
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::error::LocalYapperError;

/// Target sample rate for capture (16kHz for the local speech engine).
pub const SAMPLE_RATE: u32 = 16_000;
/// Maximum recording: 120 seconds at 16kHz.
pub const MAX_RECORDING_SAMPLES: usize = 1_920_000;

/// Atomic state flag: recorder is idle and ready.
const STATE_IDLE: u8 = 0;
/// Atomic state flag: recorder is actively capturing audio.
const STATE_RECORDING: u8 = 1;

/// Wrapper to make cpal::Stream usable in Arc<Mutex<>>.
/// cpal::Stream contains a platform marker that is !Send + !Sync,
/// but the actual audio handle is safe to move between threads
/// when protected by a Mutex on all desktop platforms.
struct StreamHandle(Option<cpal::Stream>);

// SAFETY: cpal::Stream's !Send+!Sync is a conservative platform marker.
// We only access the stream through Mutex, ensuring exclusive access.
// On Windows (WASAPI), macOS (CoreAudio), and Linux (ALSA/PulseAudio),
// the underlying stream handles are safe to send between threads.
unsafe impl Send for StreamHandle {}
unsafe impl Sync for StreamHandle {}

/// Audio recorder that captures microphone input via cpal.
pub struct AudioRecorder {
    state: Arc<AtomicU8>,
    buffer: Arc<Mutex<Vec<f32>>>,
    stop_signal: Arc<AtomicBool>,
    stream: Arc<Mutex<StreamHandle>>,
    started_at: Arc<Mutex<Option<Instant>>>,
}

impl AudioRecorder {
    /// Create a new AudioRecorder in idle state.
    pub fn new() -> Self {
        Self {
            state: Arc::new(AtomicU8::new(STATE_IDLE)),
            buffer: Arc::new(Mutex::new(Vec::new())),
            stop_signal: Arc::new(AtomicBool::new(false)),
            stream: Arc::new(Mutex::new(StreamHandle(None))),
            started_at: Arc::new(Mutex::new(None)),
        }
    }

    /// Start recording from the default microphone.
    pub fn start(&self) -> Result<(), LocalYapperError> {
        let current = self.state.load(Ordering::SeqCst);
        if current != STATE_IDLE {
            return Err(LocalYapperError::AudioError(
                "Recording is already in progress".to_string(),
            ));
        }

        self.stop_signal.store(false, Ordering::SeqCst);

        // Clear previous buffer
        if let Ok(mut buf) = self.buffer.lock() {
            buf.clear();
        }

        let host = cpal::default_host();
        let device = host.default_input_device().ok_or_else(|| {
            LocalYapperError::AudioError(
                "No microphone found. Please connect a microphone and try again.".to_string(),
            )
        })?;

        // Use the device's default config (most compatible), then resample to 16kHz mono
        let default_config = device.default_input_config().map_err(|e| {
            LocalYapperError::AudioError(format!("Failed to get default input config: {}", e))
        })?;
        let native_rate = default_config.sample_rate().0;
        let native_channels = default_config.channels();
        let config = cpal::StreamConfig {
            channels: native_channels,
            sample_rate: cpal::SampleRate(native_rate),
            buffer_size: cpal::BufferSize::Default,
        };
        log::info!(
            "Audio: native {}Hz {}ch, target {}Hz 1ch",
            native_rate,
            native_channels,
            SAMPLE_RATE
        );

        let buffer = Arc::clone(&self.buffer);
        let stop_signal = Arc::clone(&self.stop_signal);
        let started_at = Arc::clone(&self.started_at);

        let err_stop_signal = Arc::clone(&self.stop_signal);

        // Resampling state: track fractional position for accurate sample-rate conversion
        let resample_ratio = SAMPLE_RATE as f64 / native_rate as f64;
        let resample_pos = Arc::new(Mutex::new(0.0_f64));

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Check stop signal (lock-free)
                    if stop_signal.load(Ordering::Relaxed) {
                        return;
                    }

                    // Check 120s limit
                    if let Ok(guard) = started_at.lock() {
                        if let Some(start) = *guard {
                            if start.elapsed().as_secs() >= 120 {
                                stop_signal.store(true, Ordering::SeqCst);
                                return;
                            }
                        }
                    }

                    // Convert to mono by averaging channels per frame
                    let ch = native_channels as usize;
                    let mono: Vec<f32> = data
                        .chunks_exact(ch)
                        .map(|frame| frame.iter().sum::<f32>() / ch as f32)
                        .collect();

                    // Resample from native rate to 16kHz using linear interpolation
                    let resampled = if native_rate == SAMPLE_RATE {
                        mono
                    } else {
                        let mut out = Vec::new();
                        let mut pos = resample_pos.lock().unwrap_or_else(|e| e.into_inner());
                        while ((*pos).floor() as usize) < mono.len().saturating_sub(1) {
                            let idx = (*pos).floor() as usize;
                            let frac = (*pos - idx as f64) as f32;
                            let sample = mono[idx] * (1.0 - frac) + mono[idx + 1] * frac;
                            out.push(sample);
                            *pos += 1.0 / resample_ratio;
                        }
                        // Save fractional remainder for next callback
                        *pos -= mono.len() as f64;
                        if *pos < 0.0 {
                            *pos = 0.0;
                        }
                        out
                    };

                    // Append resampled samples to buffer (drop on contention)
                    if let Ok(mut buf) = buffer.try_lock() {
                        let remaining = MAX_RECORDING_SAMPLES.saturating_sub(buf.len());
                        let to_copy = resampled.len().min(remaining);
                        if to_copy > 0 {
                            buf.extend_from_slice(&resampled[..to_copy]);
                        }
                        if remaining == 0 {
                            stop_signal.store(true, Ordering::SeqCst);
                        }
                    }
                },
                move |err| {
                    log::error!("Audio stream error: {}", err);
                    err_stop_signal.store(true, Ordering::SeqCst);
                },
                None,
            )
            .map_err(|e| {
                LocalYapperError::AudioError(format!("Failed to build audio stream: {}", e))
            })?;

        stream.play().map_err(|e| {
            LocalYapperError::AudioError(format!("Failed to start audio stream: {}", e))
        })?;

        // Store stream and mark recording start
        if let Ok(mut s) = self.stream.lock() {
            s.0 = Some(stream);
        }
        if let Ok(mut t) = self.started_at.lock() {
            *t = Some(Instant::now());
        }

        self.state.store(STATE_RECORDING, Ordering::SeqCst);
        log::info!("Recording started");
        Ok(())
    }

    /// Stop recording and return the captured audio samples.
    pub fn stop(&self) -> Result<Vec<f32>, LocalYapperError> {
        let current = self.state.load(Ordering::SeqCst);
        if current != STATE_RECORDING {
            return Err(LocalYapperError::AudioError(
                "No recording in progress".to_string(),
            ));
        }

        // Signal the audio callback to stop
        self.stop_signal.store(true, Ordering::SeqCst);

        // Drop the stream to stop capture
        if let Ok(mut s) = self.stream.lock() {
            s.0 = None;
        }

        // Take the buffer
        let audio = if let Ok(mut buf) = self.buffer.lock() {
            std::mem::take(&mut *buf)
        } else {
            Vec::new()
        };

        // Clear started_at
        if let Ok(mut t) = self.started_at.lock() {
            *t = None;
        }

        self.state.store(STATE_IDLE, Ordering::SeqCst);
        log::info!("Recording stopped. Captured {} samples", audio.len());
        Ok(audio)
    }

    /// Cancel the current recording and discard all captured audio.
    pub fn cancel(&self) -> Result<(), LocalYapperError> {
        let current = self.state.load(Ordering::SeqCst);
        if current != STATE_RECORDING {
            return Err(LocalYapperError::AudioError(
                "No recording in progress".to_string(),
            ));
        }

        self.stop_signal.store(true, Ordering::SeqCst);

        if let Ok(mut s) = self.stream.lock() {
            s.0 = None;
        }
        if let Ok(mut buf) = self.buffer.lock() {
            *buf = Vec::new();
        }
        if let Ok(mut t) = self.started_at.lock() {
            *t = None;
        }

        self.state.store(STATE_IDLE, Ordering::SeqCst);
        log::info!("Recording cancelled");
        Ok(())
    }

    /// Check if a recording is currently in progress.
    #[cfg(test)]
    pub fn is_recording(&self) -> bool {
        self.state.load(Ordering::SeqCst) == STATE_RECORDING
    }

    /// Get elapsed recording time in seconds, if recording.
    #[cfg(test)]
    pub fn elapsed_seconds(&self) -> Option<f64> {
        if !self.is_recording() {
            return None;
        }
        if let Ok(guard) = self.started_at.lock() {
            guard.map(|start| start.elapsed().as_secs_f64())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorder_new_is_idle() {
        let recorder = AudioRecorder::new();
        assert!(!recorder.is_recording());
        assert!(recorder.elapsed_seconds().is_none());
    }

    #[test]
    fn recorder_stop_without_start_errors() {
        let recorder = AudioRecorder::new();
        let result = recorder.stop();
        assert!(result.is_err());
    }

    #[test]
    fn recorder_cancel_without_start_errors() {
        let recorder = AudioRecorder::new();
        let result = recorder.cancel();
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod manual_tests {
    use super::*;
    use crate::audio::vad::{apply_vad, SileroVad};
    use crate::stt::whisper::{
        stt_model_dir_name, WhisperEngine, DEFAULT_STT_MODEL, SILERO_VAD_FILENAME,
    };
    use std::path::PathBuf;
    use std::thread;
    use std::time::Duration;

    fn env_u64(key: &str, default: u64) -> Result<u64, String> {
        match std::env::var(key) {
            Ok(value) => value
                .parse::<u64>()
                .map_err(|_| format!("{key} must be a positive integer, got {value:?}")),
            Err(_) => Ok(default),
        }
    }

    fn default_app_data_dir() -> Result<PathBuf, String> {
        if let Ok(path) = std::env::var("LOCALYAPPER_APP_DATA_DIR") {
            return Ok(PathBuf::from(path));
        }

        #[cfg(target_os = "windows")]
        {
            let app_data = std::env::var("APPDATA")
                .map_err(|_| "APPDATA is not set; set LOCALYAPPER_APP_DATA_DIR".to_string())?;
            Ok(PathBuf::from(app_data).join("com.localyapper.desktop"))
        }

        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME")
                .map_err(|_| "HOME is not set; set LOCALYAPPER_APP_DATA_DIR".to_string())?;
            Ok(PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("com.localyapper.desktop"))
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
                Ok(PathBuf::from(data_home).join("com.localyapper.desktop"))
            } else {
                let home = std::env::var("HOME")
                    .map_err(|_| "HOME is not set; set LOCALYAPPER_APP_DATA_DIR".to_string())?;
                Ok(PathBuf::from(home)
                    .join(".local")
                    .join("share")
                    .join("com.localyapper.desktop"))
            }
        }
    }

    #[test]
    #[ignore = "requires an interactive desktop, a microphone, installed speech model files, and spoken audio"]
    fn manual_microphone_transcription_smoke() -> Result<(), String> {
        let countdown_secs = env_u64("LOCALYAPPER_MIC_SMOKE_COUNTDOWN_SECS", 3)?;
        let record_secs = env_u64("LOCALYAPPER_MIC_SMOKE_RECORD_SECS", 5)?;
        if record_secs == 0 {
            return Err("LOCALYAPPER_MIC_SMOKE_RECORD_SECS must be greater than 0".to_string());
        }

        let app_data_dir = default_app_data_dir()?;
        let models_dir = app_data_dir.join("models");
        let speech_model_dir = models_dir.join(stt_model_dir_name(DEFAULT_STT_MODEL));
        let vad_path = models_dir.join(SILERO_VAD_FILENAME);

        println!("Using speech model: {}", speech_model_dir.display());
        println!("Recording for {record_secs}s after a {countdown_secs}s countdown.");
        println!("Speak a short sentence while recording is active.");

        for remaining in (1..=countdown_secs).rev() {
            println!("Recording starts in {remaining}...");
            thread::sleep(Duration::from_secs(1));
        }

        let recorder = AudioRecorder::new();
        recorder.start().map_err(|e| e.to_string())?;
        println!("Recording now.");
        thread::sleep(Duration::from_secs(record_secs));
        let audio = recorder.stop().map_err(|e| e.to_string())?;
        println!("Captured {} samples at 16 kHz.", audio.len());

        if audio.len() < SAMPLE_RATE as usize {
            return Err(format!(
                "Captured too little audio: {} samples at 16 kHz",
                audio.len()
            ));
        }

        let silero = if vad_path.exists() {
            Some(SileroVad::new(&vad_path).map_err(|e| e.to_string())?)
        } else {
            None
        };
        let vad_result = apply_vad(&audio, silero.as_ref());
        if !vad_result.has_speech {
            return Err(
                "No speech detected; rerun and speak during the recording window".to_string(),
            );
        }

        let engine = WhisperEngine::new(&speech_model_dir).map_err(|e| e.to_string())?;
        let text = engine
            .transcribe(&vad_result.trimmed_audio)
            .map_err(|e| e.to_string())?;

        println!("Transcript: {text}");
        if text.trim().is_empty() {
            return Err("STT returned an empty transcript".to_string());
        }

        Ok(())
    }
}
