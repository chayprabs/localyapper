//! Rolling 16 kHz buffer addressed by absolute sample index.
//!
//! The VAD reports each speech segment as (absolute start, length). This
//! buffer keeps recent audio so a segment can be cut with pre- and
//! post-roll padding, and forgets audio that can no longer be needed, so
//! memory stays flat however long the recording runs.

use std::collections::VecDeque;

pub struct RollingBuffer {
    buf: VecDeque<f32>,
    /// Absolute index of `buf[0]`.
    base: u64,
}

impl RollingBuffer {
    pub fn new() -> Self {
        Self { buf: VecDeque::new(), base: 0 }
    }

    /// Absolute index one past the newest sample.
    pub fn end(&self) -> u64 {
        self.base + self.buf.len() as u64
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn push(&mut self, samples: &[f32]) {
        self.buf.extend(samples.iter().copied());
    }

    /// Copy `[start - pre, start + len + post)`, clamped to what is held.
    /// `floor` is the lowest index allowed, used to stop the pre-roll from
    /// reaching back into the previous segment.
    pub fn slice_padded(&self, start: u64, len: u64, pre: u64, post: u64, floor: u64) -> Vec<f32> {
        let lo = start.saturating_sub(pre).max(floor).max(self.base);
        let hi = (start + len + post).min(self.end());
        if hi <= lo {
            return Vec::new();
        }
        let a = (lo - self.base) as usize;
        let b = (hi - self.base) as usize;
        self.buf.range(a..b).copied().collect()
    }

    /// Forget everything before absolute index `keep_from`.
    pub fn drop_before(&mut self, keep_from: u64) {
        let n = keep_from.saturating_sub(self.base).min(self.buf.len() as u64) as usize;
        self.buf.drain(..n);
        self.base += n as u64;
    }
}

impl Default for RollingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(from: u32, to: u32) -> Vec<f32> {
        (from..to).map(|v| v as f32).collect()
    }

    #[test]
    fn slices_with_padding_and_clamps() {
        let mut r = RollingBuffer::new();
        r.push(&ramp(0, 100));
        assert_eq!(r.slice_padded(10, 5, 3, 2, 0), ramp(7, 17));
        // Pre-roll clamps at the start, post-roll at the end.
        assert_eq!(r.slice_padded(1, 2, 10, 0, 0), ramp(0, 3));
        assert_eq!(r.slice_padded(95, 5, 0, 50, 0), ramp(95, 100));
        // Floor stops pre-roll overlapping the previous segment.
        assert_eq!(r.slice_padded(10, 5, 8, 0, 6), ramp(6, 15));
    }

    #[test]
    fn drop_before_keeps_absolute_indexing() {
        let mut r = RollingBuffer::new();
        r.push(&ramp(0, 100));
        r.drop_before(40);
        assert_eq!(r.len(), 60);
        assert_eq!(r.end(), 100);
        r.push(&ramp(100, 120));
        assert_eq!(r.slice_padded(50, 3, 0, 0, 0), ramp(50, 53));
        // Asking for forgotten audio returns what is left.
        assert_eq!(r.slice_padded(30, 15, 0, 0, 0), ramp(40, 45));
        assert!(r.slice_padded(0, 10, 0, 0, 0).is_empty());
        // Dropping past the end or backwards is safe.
        r.drop_before(10);
        assert_eq!(r.len(), 80);
        r.drop_before(10_000);
        assert!(r.is_empty());
        assert_eq!(r.end(), 120);
    }
}
