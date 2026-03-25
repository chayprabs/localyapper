// Audio capture module -- cpal-based recording with resampling
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::error::LocalYapperError;

/// Target sample rate for capture (16kHz for Whisper).
pub const SAMPLE_RATE: u32 = 16_000;
/// Mono channel.
pub const CHANNELS: u16 = 1;
/// Pre-roll buffer size: 0.5 seconds at 16kHz.
pub const PRE_ROLL_SAMPLES: usize = 8_000;
/// Maximum recording: 120 seconds at 16kHz.
pub const MAX_RECORDING_SAMPLES: usize = 1_920_000;

const STATE_IDLE: u8 = 0;
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

/// Fixed-capacity ring buffer for pre-roll audio.
pub struct RingBuffer {
    data: Vec<f32>,
    capacity: usize,
    write_pos: usize,
    is_full: bool,
}

impl RingBuffer {
    /// Create a new ring buffer with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0.0; capacity],
            capacity,
            write_pos: 0,
            is_full: false,
        }
    }

    /// Push a slice of samples into the ring buffer.
    pub fn push_slice(&mut self, samples: &[f32]) {
        for &sample in samples {
            self.data[self.write_pos] = sample;
            self.write_pos += 1;
            if self.write_pos >= self.capacity {
                self.write_pos = 0;
                self.is_full = true;
            }
        }
    }

    /// Drain all samples in chronological order and reset the buffer.
    pub fn drain_ordered(&mut self) -> Vec<f32> {
        let result = if self.is_full {
            let mut out = Vec::with_capacity(self.capacity);
            out.extend_from_slice(&self.data[self.write_pos..]);
            out.extend_from_slice(&self.data[..self.write_pos]);
            out
        } else {
            self.data[..self.write_pos].to_vec()
        };
        self.write_pos = 0;
        self.is_full = false;
        result
    }
}

/// Audio recorder that captures microphone input via cpal.
pub struct AudioRecorder {
    state: Arc<AtomicU8>,
    buffer: Arc<Mutex<Vec<f32>>>,
    pre_roll: Arc<Mutex<RingBuffer>>,
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
            pre_roll: Arc::new(Mutex::new(RingBuffer::new(PRE_ROLL_SAMPLES))),
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
        if let Ok(mut pr) = self.pre_roll.lock() {
            *pr = RingBuffer::new(PRE_ROLL_SAMPLES);
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
        log::info!("Audio: native {}Hz {}ch, target {}Hz 1ch", native_rate, native_channels, SAMPLE_RATE);

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
            buf.clear();
        }
        if let Ok(mut t) = self.started_at.lock() {
            *t = None;
        }

        self.state.store(STATE_IDLE, Ordering::SeqCst);
        log::info!("Recording cancelled");
        Ok(())
    }

    /// Check if a recording is currently in progress.
    pub fn is_recording(&self) -> bool {
        self.state.load(Ordering::SeqCst) == STATE_RECORDING
    }

    /// Get elapsed recording time in seconds, if recording.
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
    fn ring_buffer_empty_drain() {
        let mut rb = RingBuffer::new(10);
        let result = rb.drain_ordered();
        assert!(result.is_empty());
    }

    #[test]
    fn ring_buffer_partial_fill() {
        let mut rb = RingBuffer::new(10);
        rb.push_slice(&[1.0, 2.0, 3.0]);
        let result = rb.drain_ordered();
        assert_eq!(result, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn ring_buffer_exact_fill() {
        let mut rb = RingBuffer::new(4);
        rb.push_slice(&[1.0, 2.0, 3.0, 4.0]);
        let result = rb.drain_ordered();
        assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn ring_buffer_wrap_around() {
        let mut rb = RingBuffer::new(4);
        rb.push_slice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let result = rb.drain_ordered();
        // Should contain last 4 samples in order
        assert_eq!(result, vec![3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn ring_buffer_drain_resets() {
        let mut rb = RingBuffer::new(4);
        rb.push_slice(&[1.0, 2.0, 3.0]);
        let _ = rb.drain_ordered();
        let result = rb.drain_ordered();
        assert!(result.is_empty());
    }

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
