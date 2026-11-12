//! Microphone level meter for the overlay.
//!
//! RMS over 20 ms windows of 16 kHz audio, mapped from -60..0 dBFS to
//! `[0, 1]`, smoothed with a fast attack (0.6) and slow release (0.15),
//! and emitted at most 30 times per second of audio.

const RATE: usize = 16_000;
const WINDOW: usize = RATE / 50; // 20 ms
const EMIT_EVERY: usize = RATE / 30; // 30 Hz
const ATTACK: f32 = 0.6;
const RELEASE: f32 = 0.15;

pub struct LevelMeter {
    sum_sq: f32,
    in_window: usize,
    since_emit: usize,
    level: f32,
}

impl Default for LevelMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl LevelMeter {
    pub fn new() -> Self {
        Self {
            sum_sq: 0.0,
            in_window: 0,
            since_emit: 0,
            level: 0.0,
        }
    }

    /// Current smoothed level in `[0, 1]`.
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Feed 16 kHz samples. Calls `emit` with the smoothed level whenever
    /// a 30 Hz tick passes.
    pub fn push(&mut self, samples: &[f32], mut emit: impl FnMut(f32)) {
        for &s in samples {
            self.sum_sq += s * s;
            self.in_window += 1;
            if self.in_window == WINDOW {
                let rms = (self.sum_sq / WINDOW as f32).sqrt();
                let target = rms_to_level(rms);
                let k = if target > self.level { ATTACK } else { RELEASE };
                self.level += k * (target - self.level);
                self.sum_sq = 0.0;
                self.in_window = 0;
            }
            self.since_emit += 1;
            if self.since_emit >= EMIT_EVERY {
                self.since_emit = 0;
                emit(self.level);
            }
        }
    }
}

/// Map RMS amplitude to `[0, 1]` on a -60..0 dBFS scale.
pub fn rms_to_level(rms: f32) -> f32 {
    if rms <= 1e-6 {
        return 0.0;
    }
    ((20.0 * rms.log10() + 60.0) / 60.0).clamp(0.0, 1.0)
}

/// RMS of a slice, 0 for an empty slice.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(amp: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| amp * (i as f32 * 2.0 * std::f32::consts::PI * 440.0 / RATE as f32).sin())
            .collect()
    }

    #[test]
    fn maps_db_scale() {
        assert_eq!(rms_to_level(0.0), 0.0);
        assert_eq!(rms_to_level(1.0), 1.0);
        assert!((rms_to_level(0.001) - 0.0).abs() < 1e-6);
        assert!((rms_to_level(0.031_6) - 0.5).abs() < 0.01);
    }

    #[test]
    fn emits_at_30_hz_and_tracks_loudness() {
        let mut m = LevelMeter::new();
        let mut out = Vec::new();
        m.push(&sine(0.5, RATE), |v| out.push(v));
        assert_eq!(out.len(), 30);
        let loud = *out.last().unwrap_or(&0.0);
        assert!(loud > 0.8, "loud level {loud}");
        out.clear();
        m.push(&vec![0.0; RATE], |v| out.push(v));
        assert!(*out.last().unwrap_or(&1.0) < 0.05);
    }

    #[test]
    fn attack_is_faster_than_release() {
        let mut m = LevelMeter::new();
        m.push(&sine(0.5, WINDOW), |_| {});
        let after_one_loud = m.level();
        m.push(&sine(0.5, WINDOW * 20), |_| {});
        let full = m.level();
        m.push(&vec![0.0; WINDOW], |_| {});
        let drop = full - m.level();
        assert!(after_one_loud > 0.5 * full);
        assert!(drop < 0.2 * full, "release too fast: {drop}");
    }
}
