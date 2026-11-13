//! Filter for X11 key auto-repeat.
//!
//! On X11 a held key arrives as Release/Press pairs a few milliseconds
//! apart. The `global-hotkey` crate forwards both, so hold-to-talk would
//! stop after the keyboard repeat delay. This filter holds each Release
//! back for a short window and drops it, together with the Press that
//! follows, when they form a repeat pair.
//!
//! Call `on_press` / `on_release` from the hotkey callback, and
//! `poll(now_ms)` from a timer (10 ms) or on the next event. All three
//! return the events to forward.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Down { at_ms: u64 },
    Up { at_ms: u64 },
}

pub struct RepeatFilter {
    window_ms: u64,
    pending_up: Option<u64>,
    down: bool,
}

impl RepeatFilter {
    /// 30 ms is far above the gap inside a repeat pair and far below a
    /// deliberate double-tap.
    pub fn new(window_ms: u64) -> Self {
        Self { window_ms, pending_up: None, down: false }
    }

    pub fn on_press(&mut self, now_ms: u64) -> Vec<Key> {
        let mut out = self.poll(now_ms);
        if self.pending_up.take().is_some() {
            // Release then Press inside the window: auto-repeat. Drop both.
            return out;
        }
        if !self.down {
            self.down = true;
            out.push(Key::Down { at_ms: now_ms });
        }
        out
    }

    pub fn on_release(&mut self, now_ms: u64) -> Vec<Key> {
        let out = self.poll(now_ms);
        if self.down {
            self.pending_up = Some(now_ms);
        }
        out
    }

    pub fn poll(&mut self, now_ms: u64) -> Vec<Key> {
        match self.pending_up {
            Some(at) if now_ms.saturating_sub(at) >= self.window_ms => {
                self.pending_up = None;
                self.down = false;
                vec![Key::Up { at_ms: at }]
            }
            _ => Vec::new(),
        }
    }

    /// Milliseconds until `poll` should run again, if a release is held.
    pub fn next_deadline(&self) -> Option<u64> {
        self.pending_up.map(|at| at + self.window_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_press_and_release_pass_through() {
        let mut f = RepeatFilter::new(30);
        assert_eq!(f.on_press(0), vec![Key::Down { at_ms: 0 }]);
        assert!(f.on_release(500).is_empty());
        assert!(f.poll(520).is_empty());
        // The Up keeps its true time, so tap-length maths stays right.
        assert_eq!(f.poll(530), vec![Key::Up { at_ms: 500 }]);
    }

    #[test]
    fn auto_repeat_pairs_are_swallowed() {
        let mut f = RepeatFilter::new(30);
        assert_eq!(f.on_press(0), vec![Key::Down { at_ms: 0 }]);
        for t in [500u64, 533, 566, 600] {
            assert!(f.on_release(t).is_empty());
            assert!(f.on_press(t + 1).is_empty());
        }
        assert!(f.on_release(700).is_empty());
        assert_eq!(f.poll(731), vec![Key::Up { at_ms: 700 }]);
    }

    #[test]
    fn double_tap_survives() {
        let mut f = RepeatFilter::new(30);
        assert_eq!(f.on_press(0), vec![Key::Down { at_ms: 0 }]);
        assert!(f.on_release(100).is_empty());
        // The second press arrives 150 ms later: flush the Up, then Down.
        assert_eq!(
            f.on_press(250),
            vec![Key::Up { at_ms: 100 }, Key::Down { at_ms: 250 }]
        );
    }

    #[test]
    fn stray_release_is_ignored() {
        let mut f = RepeatFilter::new(30);
        assert!(f.on_release(0).is_empty());
        assert!(f.poll(100).is_empty());
        assert_eq!(f.next_deadline(), None);
    }
}
