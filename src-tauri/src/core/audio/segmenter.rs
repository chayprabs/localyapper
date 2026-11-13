//! Segmenter: one per recording, on its own thread.
//!
//! Pops native-rate mono audio from the ring buffer, resamples it to
//! 16 kHz with sherpa-onnx's band-limited `LinearResampler`, feeds the VAD
//! in 512-sample windows and cuts each detected segment, padded with
//! pre- and post-roll, out of a rolling buffer. Segments are handed to the
//! STT queue as soon as they close, so decoding overlaps the user's speech.

use super::level::LevelMeter;
use super::rolling::RollingBuffer;
use crate::core::session::{Event, SessionId};
use sherpa_onnx::{LinearResampler, SileroVadModelConfig, VadModelConfig, VoiceActivityDetector};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const RATE: u64 = 16_000;
/// Samples per VAD window (32 ms at 16 kHz, what Silero expects).
pub const WINDOW: usize = 512;
/// Pre-roll added before each segment (200 ms).
pub const PRE: u64 = RATE / 5;
/// Post-roll added after each segment (150 ms).
pub const POST: u64 = RATE * 15 / 100;
/// The no-speech safety net sends at most this much audio (15 s).
pub const SAFETY_NET_MAX: u64 = RATE * 15;

/// VAD parameters (plan P1.6).
pub const VAD_THRESHOLD: f32 = 0.4;
pub const VAD_MIN_SILENCE_S: f32 = 0.35;
pub const VAD_MIN_SPEECH_S: f32 = 0.15;
pub const VAD_MAX_SPEECH_S: f32 = 15.0;

/// A speech segment found by a detector, at its absolute 16 kHz index.
#[derive(Clone, Debug, PartialEq)]
pub struct DetectedSegment {
    pub start: u64,
    pub samples: Vec<f32>,
}

/// Voice activity detection over 16 kHz audio fed in `WINDOW` chunks.
pub trait SpeechDetector: Send {
    fn accept(&mut self, window: &[f32]);
    /// Close any open segment at the end of the recording.
    fn flush(&mut self);
    /// Next closed segment, oldest first.
    fn pop(&mut self) -> Option<DetectedSegment>;
}

/// Silero VAD through sherpa-onnx.
pub struct SileroDetector {
    vad: VoiceActivityDetector,
}

impl SileroDetector {
    pub fn new(model: &Path) -> Result<Self, String> {
        if !model.is_file() {
            return Err(format!("VAD model missing: {}", model.display()));
        }
        let cfg = VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(model.to_string_lossy().into_owned()),
                threshold: VAD_THRESHOLD,
                min_silence_duration: VAD_MIN_SILENCE_S,
                min_speech_duration: VAD_MIN_SPEECH_S,
                window_size: WINDOW as i32,
                max_speech_duration: VAD_MAX_SPEECH_S,
            },
            sample_rate: RATE as i32,
            num_threads: 1,
            provider: Some("cpu".into()),
            ..Default::default()
        };
        let vad = VoiceActivityDetector::create(&cfg, 30.0).ok_or("could not load Silero VAD")?;
        Ok(Self { vad })
    }
}

impl SpeechDetector for SileroDetector {
    fn accept(&mut self, window: &[f32]) {
        self.vad.accept_waveform(window);
    }

    fn flush(&mut self) {
        self.vad.flush();
    }

    fn pop(&mut self) -> Option<DetectedSegment> {
        if self.vad.is_empty() {
            return None;
        }
        let seg = self.vad.front().map(|s| DetectedSegment {
            start: s.start().max(0) as u64,
            samples: s.samples().to_vec(),
        });
        self.vad.pop();
        seg
    }
}

/// Energy VAD, used when the Silero model cannot be loaded.
pub struct EnergyDetector {
    threshold: f32,
    pos: u64,
    in_speech: bool,
    start: u64,
    speech: Vec<f32>,
    silence_run: usize,
    voiced: usize,
    ready: std::collections::VecDeque<DetectedSegment>,
}

impl Default for EnergyDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl EnergyDetector {
    pub fn new() -> Self {
        Self {
            threshold: 0.015,
            pos: 0,
            in_speech: false,
            start: 0,
            speech: Vec::new(),
            silence_run: 0,
            voiced: 0,
            ready: Default::default(),
        }
    }

    fn windows(seconds: f32) -> usize {
        ((seconds * RATE as f32) / WINDOW as f32).ceil() as usize
    }

    fn close(&mut self) {
        if !self.in_speech {
            return;
        }
        self.in_speech = false;
        // Trim the trailing silence that closed the segment.
        let keep = self.speech.len().saturating_sub(self.silence_run * WINDOW);
        self.speech.truncate(keep);
        if self.voiced >= Self::windows(VAD_MIN_SPEECH_S) {
            self.ready.push_back(DetectedSegment {
                start: self.start,
                samples: std::mem::take(&mut self.speech),
            });
        }
        self.speech.clear();
        self.silence_run = 0;
        self.voiced = 0;
    }
}

impl SpeechDetector for EnergyDetector {
    fn accept(&mut self, window: &[f32]) {
        let loud = super::level::rms(window) >= self.threshold;
        if loud && !self.in_speech {
            self.in_speech = true;
            self.start = self.pos;
        }
        if self.in_speech {
            self.speech.extend_from_slice(window);
            if loud {
                self.voiced += 1;
                self.silence_run = 0;
            } else {
                self.silence_run += 1;
            }
            let too_long = self.speech.len() as f32 >= VAD_MAX_SPEECH_S * RATE as f32;
            if self.silence_run >= Self::windows(VAD_MIN_SILENCE_S) || too_long {
                self.close();
            }
        }
        self.pos += window.len() as u64;
    }

    fn flush(&mut self) {
        self.close();
    }

    fn pop(&mut self) -> Option<DetectedSegment> {
        self.ready.pop_front()
    }
}

/// Where native-rate mono samples come from (the ring-buffer consumer in
/// the app, a vector in tests).
pub trait SampleSource: Send {
    /// Copy up to `out.len()` samples into `out` and return how many.
    fn pop_into(&mut self, out: &mut [f32]) -> usize;
}

impl SampleSource for ringbuf::HeapCons<f32> {
    fn pop_into(&mut self, out: &mut [f32]) -> usize {
        use ringbuf::traits::Consumer;
        self.pop_slice(out)
    }
}

/// A source that yields a fixed buffer, for tests and the bench.
pub struct VecSource {
    data: Vec<f32>,
    pos: usize,
}

impl VecSource {
    pub fn new(data: Vec<f32>) -> Self {
        Self { data, pos: 0 }
    }
}

impl SampleSource for VecSource {
    fn pop_into(&mut self, out: &mut [f32]) -> usize {
        let n = out.len().min(self.data.len() - self.pos);
        out[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        n
    }
}

/// Receives closed segments for decoding.
pub trait SegmentSink {
    fn push(&self, id: SessionId, index: usize, samples: Vec<f32>);
}

/// Signals shared with the controller.
pub struct Flags<'a> {
    /// Set once the stream has stopped: drain what is left, then finish.
    pub stop: &'a AtomicBool,
    /// Set by `DiscardCapture`: return at once, send nothing more.
    pub discard: &'a AtomicBool,
}

/// Summary returned when the segmenter finishes normally.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub segments: usize,
    pub audio_ms: u64,
    pub safety_net: bool,
}

struct Cutter<'a> {
    id: SessionId,
    sink: &'a dyn SegmentSink,
    events: &'a dyn Fn(Event),
    prev_end: u64,
    index: usize,
}

impl Cutter<'_> {
    fn emit_ready(&mut self, vad: &mut dyn SpeechDetector, rolling: &RollingBuffer) {
        while let Some(seg) = vad.pop() {
            let len = seg.samples.len() as u64;
            let audio = rolling.slice_padded(seg.start, len, PRE, POST, self.prev_end);
            // If the rolling buffer no longer holds it, use the VAD's copy.
            let audio = if audio.is_empty() { seg.samples } else { audio };
            self.prev_end = seg.start + len;
            // Queued is always sent before the decoder can answer.
            (self.events)(Event::SegmentQueued {
                id: self.id,
                index: self.index,
            });
            self.sink.push(self.id, self.index, audio);
            self.index += 1;
        }
    }
}

/// Run one recording's segmenter until `flags.stop` is set and the source
/// is drained (then `CaptureEnded` is sent) or `flags.discard` is set.
#[allow(clippy::too_many_arguments)]
pub fn run_segmenter(
    id: SessionId,
    native_rate: u32,
    src: &mut dyn SampleSource,
    flags: Flags<'_>,
    vad: &mut dyn SpeechDetector,
    sink: &dyn SegmentSink,
    events: &dyn Fn(Event),
    levels: &mut dyn FnMut(f32),
) -> Result<Summary, String> {
    let resampler = if u64::from(native_rate) == RATE {
        None
    } else {
        Some(
            LinearResampler::create(native_rate as i32, RATE as i32)
                .ok_or("could not create resampler")?,
        )
    };

    let mut rolling = RollingBuffer::new();
    let mut meter = LevelMeter::new();
    let mut pending: Vec<f32> = Vec::with_capacity(WINDOW * 4);
    let mut native = vec![0.0f32; 4096];
    let mut fed: u64 = 0;
    let mut total_16k: u64 = 0;
    let mut sum_sq: f64 = 0.0;
    let mut cut = Cutter {
        id,
        sink,
        events,
        prev_end: 0,
        index: 0,
    };

    loop {
        if flags.discard.load(Ordering::Acquire) {
            return Ok(Summary::default());
        }
        let stopping = flags.stop.load(Ordering::Acquire);
        let n = src.pop_into(&mut native);
        if n == 0 && !stopping {
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }
        let last = stopping && n == 0;

        let chunk: Vec<f32> = match &resampler {
            Some(r) => r.resample(&native[..n], last),
            None => native[..n].to_vec(),
        };
        meter.push(&chunk, &mut *levels);
        total_16k += chunk.len() as u64;
        sum_sq += chunk
            .iter()
            .map(|s| f64::from(*s) * f64::from(*s))
            .sum::<f64>();
        rolling.push(&chunk);
        pending.extend_from_slice(&chunk);

        let mut off = 0;
        while pending.len() - off >= WINDOW {
            vad.accept(&pending[off..off + WINDOW]);
            off += WINDOW;
            fed += WINDOW as u64;
            cut.emit_ready(vad, &rolling);
        }
        pending.drain(..off);
        // Keep only what a future segment, or the safety net, could need.
        rolling.drop_before(cut.prev_end.max(fed.saturating_sub(RATE * 20)));

        if last {
            if !pending.is_empty() {
                pending.resize(WINDOW, 0.0);
                vad.accept(&pending);
            }
            vad.flush();
            cut.emit_ready(vad, &rolling);
            let mut safety_net = false;
            // P1.7: the VAD found nothing in audible audio. Send the tail.
            let overall_rms = if total_16k > 0 {
                (sum_sq / total_16k as f64).sqrt()
            } else {
                0.0
            };
            if cut.index == 0 && total_16k > RATE * 4 / 10 && overall_rms > 0.01 {
                let take = (rolling.len() as u64).min(SAFETY_NET_MAX);
                let tail = rolling.slice_padded(rolling.end() - take, take, 0, 0, 0);
                if !tail.is_empty() {
                    events(Event::SegmentQueued { id, index: 0 });
                    sink.push(id, 0, tail);
                    cut.index = 1;
                    safety_net = true;
                }
            }
            let audio_ms = total_16k * 1000 / RATE;
            events(Event::CaptureEnded {
                id,
                segments: cut.index,
                audio_ms,
            });
            return Ok(Summary {
                segments: cut.index,
                audio_ms,
                safety_net,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Collect(Mutex<Vec<(usize, Vec<f32>)>>);
    impl SegmentSink for Collect {
        fn push(&self, _id: SessionId, index: usize, samples: Vec<f32>) {
            if let Ok(mut v) = self.0.lock() {
                v.push((index, samples));
            }
        }
    }

    fn tone(seconds: f32, rate: u32, amp: f32) -> Vec<f32> {
        let n = (seconds * rate as f32) as usize;
        (0..n)
            .map(|i| amp * (i as f32 * 2.0 * std::f32::consts::PI * 300.0 / rate as f32).sin())
            .collect()
    }

    fn silence(seconds: f32, rate: u32) -> Vec<f32> {
        vec![0.0; (seconds * rate as f32) as usize]
    }

    type Segs = Vec<(usize, Vec<f32>)>;

    fn run(audio: Vec<f32>, rate: u32) -> (Summary, Segs, Vec<Event>, usize) {
        let stop = AtomicBool::new(true);
        let discard = AtomicBool::new(false);
        let sink = Collect::default();
        let events = Mutex::new(Vec::new());
        let mut levels = 0usize;
        let mut vad = EnergyDetector::new();
        let summary = run_segmenter(
            7,
            rate,
            &mut VecSource::new(audio),
            Flags {
                stop: &stop,
                discard: &discard,
            },
            &mut vad,
            &sink,
            &|e| {
                if let Ok(mut v) = events.lock() {
                    v.push(e);
                }
            },
            &mut |_| levels += 1,
        )
        .expect("segmenter");
        let segs = sink.0.into_inner().unwrap_or_default();
        (
            summary,
            segs,
            events.into_inner().unwrap_or_default(),
            levels,
        )
    }

    #[test]
    fn two_utterances_make_two_padded_segments() {
        let mut audio = silence(0.5, 16_000);
        audio.extend(tone(1.0, 16_000, 0.3));
        audio.extend(silence(1.0, 16_000));
        audio.extend(tone(0.8, 16_000, 0.3));
        audio.extend(silence(0.5, 16_000));
        let (summary, segs, events, levels) = run(audio, 16_000);
        assert_eq!(summary.segments, 2);
        assert!(!summary.safety_net);
        assert_eq!(summary.audio_ms, 3800);
        assert_eq!(segs.len(), 2);
        // Padded: longer than the tone, by at most pre + post roll plus a window.
        let first = segs[0].1.len() as u64;
        assert!(
            first > RATE && first <= RATE + PRE + POST + WINDOW as u64,
            "{first}"
        );
        assert_eq!(
            events.first(),
            Some(&Event::SegmentQueued { id: 7, index: 0 })
        );
        assert_eq!(
            events.last(),
            Some(&Event::CaptureEnded {
                id: 7,
                segments: 2,
                audio_ms: 3800
            })
        );
        assert!(levels >= 100, "levels {levels}");
    }

    #[test]
    fn resamples_48k_input() {
        let mut audio = silence(0.3, 48_000);
        audio.extend(tone(1.0, 48_000, 0.3));
        audio.extend(silence(0.6, 48_000));
        let (summary, segs, _, _) = run(audio, 48_000);
        assert_eq!(summary.segments, 1);
        // 1.9 s of 48 kHz input is about 1.9 s at 16 kHz.
        assert!(
            (summary.audio_ms as i64 - 1900).abs() < 30,
            "{}",
            summary.audio_ms
        );
        assert!(segs[0].1.len() > 15_000);
    }

    #[test]
    fn silence_produces_no_segment() {
        let (summary, segs, _, _) = run(silence(2.0, 16_000), 16_000);
        assert_eq!(summary.segments, 0);
        assert!(segs.is_empty());
    }

    #[test]
    fn safety_net_sends_quiet_but_audible_audio() {
        // Steady low-level audio just under the energy threshold per window
        // but audible overall: the detector finds nothing, the net fires.
        let audio = tone(1.0, 16_000, 0.018);
        let (summary, segs, _, _) = run(audio, 16_000);
        assert!(summary.safety_net);
        assert_eq!(summary.segments, 1);
        assert_eq!(segs[0].1.len(), 16_000);
    }

    #[test]
    fn short_recordings_skip_the_safety_net() {
        let (summary, _, _, _) = run(tone(0.3, 16_000, 0.018), 16_000);
        assert_eq!(summary.segments, 0);
    }

    #[test]
    fn discard_returns_without_events() {
        let stop = AtomicBool::new(false);
        let discard = AtomicBool::new(true);
        let sink = Collect::default();
        let sent = Mutex::new(0usize);
        let mut vad = EnergyDetector::new();
        let s = run_segmenter(
            1,
            16_000,
            &mut VecSource::new(tone(1.0, 16_000, 0.3)),
            Flags {
                stop: &stop,
                discard: &discard,
            },
            &mut vad,
            &sink,
            &|_| {
                if let Ok(mut n) = sent.lock() {
                    *n += 1;
                }
            },
            &mut |_| {},
        )
        .expect("segmenter");
        assert_eq!(s, Summary::default());
        assert_eq!(sent.into_inner().unwrap_or(1), 0);
    }

    #[test]
    fn energy_detector_splits_long_speech() {
        let mut d = EnergyDetector::new();
        let loud = tone(16.0, 16_000, 0.3);
        for w in loud.as_chunks::<WINDOW>().0 {
            d.accept(w);
        }
        d.flush();
        let a = d.pop().expect("first");
        let b = d.pop().expect("second");
        assert!(a.samples.len() as f32 <= VAD_MAX_SPEECH_S * 16_000.0 + WINDOW as f32);
        assert_eq!(b.start, a.samples.len() as u64);
        assert!(d.pop().is_none());
    }

    fn sine_at(freq: f32, rate: u32, seconds: f32) -> Vec<f32> {
        let n = (seconds * rate as f32) as usize;
        (0..n)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin())
            .collect()
    }

    fn resample_48k(input: &[f32]) -> Vec<f32> {
        let r = LinearResampler::create(48_000, 16_000).expect("resampler");
        // Feed in device-sized chunks, as the segmenter does.
        let mut out = Vec::new();
        for c in input.chunks(480) {
            out.extend(r.resample(c, false));
        }
        out.extend(r.resample(&[], true));
        out
    }

    #[test]
    fn resampler_keeps_1khz_frequency_and_level() {
        let input = sine_at(1_000.0, 48_000, 2.0);
        let out = resample_48k(&input);
        // Skip the filter's start-up and tail, measure over one second.
        let mid = &out[4_000..20_000];
        let crossings = mid.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
        assert!(
            (crossings as f32 - 1_000.0).abs() <= 1.0,
            "frequency {crossings} Hz"
        );
        let db = 20.0 * (super::super::level::rms(mid) / super::super::level::rms(&input)).log10();
        assert!(db.abs() <= 0.5, "level change {db} dB");
    }

    #[test]
    fn resampler_rejects_10khz() {
        let input = sine_at(10_000.0, 48_000, 2.0);
        let out = resample_48k(&input);
        let mid = &out[4_000..20_000];
        let db = 20.0 * (super::super::level::rms(mid) / super::super::level::rms(&input)).log10();
        assert!(db <= -40.0, "10 kHz only {db} dB down");
    }
}
