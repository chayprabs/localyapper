//! Dictation state machine. Pure: no I/O, no clocks, no tauri.
//! The controller feeds `Event`s (with a monotonic millisecond timestamp
//! where needed) and executes the returned `Effect`s in order.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationMode {
    /// Record while the key is held.
    Hold,
    /// Press to start, press again to stop.
    Toggle,
    /// Hold to record; a quick double-tap latches hands-free.
    Hybrid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub mode: ActivationMode,
    pub tap_threshold_ms: u64,
    pub double_tap_window_ms: u64,
    pub max_recording_ms: u64,
    pub warning_before_end_ms: u64,
    pub cancel_key_enabled: bool,
    pub cancel_key_in_hands_free: bool,
    pub history_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: ActivationMode::Hybrid,
            tap_threshold_ms: 250,
            double_tap_window_ms: 350,
            max_recording_ms: 600_000,
            warning_before_end_ms: 15_000,
            cancel_key_enabled: true,
            cancel_key_in_hands_free: false,
            history_enabled: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecKind {
    Hold,
    TapPending,
    HandsFree,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerKind {
    TapWindow,
    Warning,
    MaxDuration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum View {
    Hidden,
    Listening,
    HandsFree,
    StoppingSoon { remaining_ms: u64 },
    Processing,
    Done { text: String },
    NoSpeech,
    Error { message: String, text: Option<String> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tray {
    Idle,
    Recording,
    Processing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Start,
    Stop,
}

/// Session ids make every late message from an older recording harmless.
pub type SessionId = u64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    RecordDown { at_ms: u64 },
    RecordUp { at_ms: u64 },
    /// Cancel hotkey or the `cancel_recording` IPC command.
    Cancel,
    Timer { kind: TimerKind, id: SessionId },
    /// The segmenter closed a speech segment and queued it for decoding.
    SegmentQueued { id: SessionId, index: usize },
    SegmentText { id: SessionId, index: usize, text: String },
    /// Capture stopped and flushed; `segments` is the final segment count.
    CaptureEnded { id: SessionId, segments: usize, audio_ms: u64 },
    CaptureFailed { id: SessionId, message: String },
    ModelFailed { message: String },
    InjectDone { id: SessionId, result: Result<(), String> },
    SetPaused(bool),
    ConfigChanged(Config),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    StartCapture { id: SessionId },
    /// Stop the stream, flush the segmenter, then send `CaptureEnded`.
    StopCapture { id: SessionId },
    /// Stop the stream and drop everything queued for this session.
    DiscardCapture { id: SessionId },
    EnsureModelLoaded,
    ScheduleEvict,
    Overlay(View),
    Tray(Tray),
    Cue(Cue),
    RegisterCancelKey,
    UnregisterCancelKey,
    StartTimer { kind: TimerKind, id: SessionId, after_ms: u64 },
    Inject { id: SessionId, text: String },
    SaveHistory { text: String, audio_ms: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum State {
    Idle,
    Recording {
        id: SessionId,
        kind: RecKind,
        pressed_at_ms: u64,
        texts: Vec<Option<String>>,
        cancel_registered: bool,
    },
    Finishing {
        id: SessionId,
        texts: Vec<Option<String>>,
        total: Option<usize>,
        audio_ms: u64,
    },
    Injecting {
        id: SessionId,
        text: String,
    },
}

pub struct Session {
    cfg: Config,
    state: State,
    next_id: SessionId,
    paused: bool,
}

impl Session {
    pub fn new(cfg: Config) -> Self {
        Self { cfg, state: State::Idle, next_id: 1, paused: false }
    }

    pub fn is_idle(&self) -> bool {
        self.state == State::Idle
    }

    pub fn is_recording(&self) -> bool {
        matches!(self.state, State::Recording { .. })
    }

    pub fn handle(&mut self, event: Event) -> Vec<Effect> {
        let mut fx = Vec::new();
        match event {
            Event::ConfigChanged(cfg) => self.cfg = cfg,
            Event::SetPaused(p) => {
                self.paused = p;
                if p {
                    self.abort(&mut fx, View::Hidden);
                }
            }
            Event::RecordDown { at_ms } => self.on_down(at_ms, &mut fx),
            Event::RecordUp { at_ms } => self.on_up(at_ms, &mut fx),
            Event::Cancel => self.abort(&mut fx, View::Hidden),
            Event::Timer { kind, id } => self.on_timer(kind, id, &mut fx),
            Event::SegmentQueued { id, index } => {
                if let Some(texts) = self.texts_for(id) {
                    if texts.len() <= index {
                        texts.resize(index + 1, None);
                    }
                }
            }
            Event::SegmentText { id, index, text } => {
                if let Some(texts) = self.texts_for(id) {
                    if texts.len() <= index {
                        texts.resize(index + 1, None);
                    }
                    texts[index] = Some(text);
                }
                self.try_complete(&mut fx);
            }
            Event::CaptureEnded { id, segments, audio_ms } => {
                if let State::Finishing { id: cur, texts, total, audio_ms: a } = &mut self.state {
                    if *cur == id {
                        texts.resize(segments, None);
                        *total = Some(segments);
                        *a = audio_ms;
                    }
                }
                self.try_complete(&mut fx);
            }
            Event::CaptureFailed { id, message } => {
                if self.current_id() == Some(id) && !matches!(self.state, State::Injecting { .. }) {
                    self.abort(&mut fx, View::Error { message, text: None });
                }
            }
            Event::ModelFailed { message } => {
                if matches!(self.state, State::Recording { .. } | State::Finishing { .. }) {
                    self.abort(&mut fx, View::Error { message, text: None });
                }
            }
            Event::InjectDone { id, result } => {
                if let State::Injecting { id: cur, text } = &self.state {
                    if *cur == id {
                        let text = text.clone();
                        self.state = State::Idle;
                        fx.push(Effect::Tray(Tray::Idle));
                        fx.push(Effect::Overlay(match result {
                            Ok(()) => View::Done { text },
                            Err(message) => View::Error { message, text: Some(text) },
                        }));
                        fx.push(Effect::ScheduleEvict);
                    }
                }
            }
        }
        fx
    }

    fn current_id(&self) -> Option<SessionId> {
        match &self.state {
            State::Idle => None,
            State::Recording { id, .. }
            | State::Finishing { id, .. }
            | State::Injecting { id, .. } => Some(*id),
        }
    }

    fn texts_for(&mut self, id: SessionId) -> Option<&mut Vec<Option<String>>> {
        match &mut self.state {
            State::Recording { id: cur, texts, .. } | State::Finishing { id: cur, texts, .. }
                if *cur == id =>
            {
                Some(texts)
            }
            _ => None,
        }
    }

    fn wants_cancel_key(&self, kind: RecKind) -> bool {
        self.cfg.cancel_key_enabled
            && (kind != RecKind::HandsFree || self.cfg.cancel_key_in_hands_free)
    }

    fn on_down(&mut self, at_ms: u64, fx: &mut Vec<Effect>) {
        match &mut self.state {
            State::Idle => {
                if self.paused {
                    fx.push(Effect::Overlay(View::Error {
                        message: "Dictation is paused. Resume from the tray menu.".into(),
                        text: None,
                    }));
                    return;
                }
                let id = self.next_id;
                self.next_id += 1;
                let kind = match self.cfg.mode {
                    ActivationMode::Toggle => RecKind::HandsFree,
                    ActivationMode::Hold | ActivationMode::Hybrid => RecKind::Hold,
                };
                let cancel = self.wants_cancel_key(kind);
                self.state = State::Recording {
                    id,
                    kind,
                    pressed_at_ms: at_ms,
                    texts: Vec::new(),
                    cancel_registered: cancel,
                };
                // Capture first: nothing may delay the microphone.
                fx.push(Effect::StartCapture { id });
                fx.push(Effect::EnsureModelLoaded);
                fx.push(Effect::Tray(Tray::Recording));
                fx.push(Effect::Overlay(View::Listening));
                fx.push(Effect::Cue(Cue::Start));
                if cancel {
                    fx.push(Effect::RegisterCancelKey);
                }
                let max = self.cfg.max_recording_ms;
                let warn = max.saturating_sub(self.cfg.warning_before_end_ms);
                if warn > 0 && warn < max {
                    fx.push(Effect::StartTimer { kind: TimerKind::Warning, id, after_ms: warn });
                }
                fx.push(Effect::StartTimer { kind: TimerKind::MaxDuration, id, after_ms: max });
            }
            State::Recording { kind, cancel_registered, .. } => match *kind {
                // Key auto-repeat while held.
                RecKind::Hold => {}
                RecKind::TapPending => {
                    *kind = RecKind::HandsFree;
                    let keep = self.cfg.cancel_key_enabled && self.cfg.cancel_key_in_hands_free;
                    if *cancel_registered && !keep {
                        *cancel_registered = false;
                        fx.push(Effect::UnregisterCancelKey);
                    }
                    fx.push(Effect::Overlay(View::HandsFree));
                }
                RecKind::HandsFree => self.finish(fx),
            },
            // A press while the tail is decoding or pasting is dropped.
            State::Finishing { .. } | State::Injecting { .. } => {}
        }
    }

    fn on_up(&mut self, at_ms: u64, fx: &mut Vec<Effect>) {
        let State::Recording { id, kind, pressed_at_ms, .. } = &mut self.state else {
            return;
        };
        if *kind != RecKind::Hold {
            return;
        }
        let held = at_ms.saturating_sub(*pressed_at_ms);
        if self.cfg.mode == ActivationMode::Hybrid && held <= self.cfg.tap_threshold_ms {
            *kind = RecKind::TapPending;
            fx.push(Effect::StartTimer {
                kind: TimerKind::TapWindow,
                id: *id,
                after_ms: self.cfg.double_tap_window_ms,
            });
        } else {
            self.finish(fx);
        }
    }

    fn on_timer(&mut self, timer: TimerKind, timer_id: SessionId, fx: &mut Vec<Effect>) {
        let State::Recording { id, kind, .. } = &self.state else {
            return;
        };
        if *id != timer_id {
            return;
        }
        match timer {
            TimerKind::TapWindow => {
                if *kind == RecKind::TapPending {
                    self.abort(fx, View::Hidden);
                }
            }
            TimerKind::Warning => fx.push(Effect::Overlay(View::StoppingSoon {
                remaining_ms: self.cfg.warning_before_end_ms,
            })),
            TimerKind::MaxDuration => self.finish(fx),
        }
    }

    fn finish(&mut self, fx: &mut Vec<Effect>) {
        let State::Recording { id, texts, cancel_registered, .. } = &mut self.state else {
            return;
        };
        let id = *id;
        if *cancel_registered {
            fx.push(Effect::UnregisterCancelKey);
        }
        let texts = std::mem::take(texts);
        self.state = State::Finishing { id, texts, total: None, audio_ms: 0 };
        fx.push(Effect::StopCapture { id });
        fx.push(Effect::Cue(Cue::Stop));
        fx.push(Effect::Tray(Tray::Processing));
        fx.push(Effect::Overlay(View::Processing));
    }

    /// Leave any non-idle, non-injecting state without producing text.
    fn abort(&mut self, fx: &mut Vec<Effect>, view: View) {
        match &self.state {
            State::Idle | State::Injecting { .. } => return,
            State::Recording { id, cancel_registered, .. } => {
                if *cancel_registered {
                    fx.push(Effect::UnregisterCancelKey);
                }
                fx.push(Effect::DiscardCapture { id: *id });
            }
            State::Finishing { id, .. } => fx.push(Effect::DiscardCapture { id: *id }),
        }
        self.state = State::Idle;
        fx.push(Effect::Tray(Tray::Idle));
        fx.push(Effect::Overlay(view));
        fx.push(Effect::ScheduleEvict);
    }

    fn try_complete(&mut self, fx: &mut Vec<Effect>) {
        let State::Finishing { id, texts, total: Some(total), audio_ms } = &self.state else {
            return;
        };
        if texts.len() < *total || texts.iter().take(*total).any(Option::is_none) {
            return;
        }
        let id = *id;
        let audio_ms = *audio_ms;
        let text = join_segments(texts.iter().flatten().map(String::as_str));
        if text.is_empty() {
            self.state = State::Idle;
            fx.push(Effect::Tray(Tray::Idle));
            fx.push(Effect::Overlay(View::NoSpeech));
            fx.push(Effect::ScheduleEvict);
            return;
        }
        // History first, so a failed paste never loses the words.
        if self.cfg.history_enabled {
            fx.push(Effect::SaveHistory { text: text.clone(), audio_ms });
        }
        fx.push(Effect::Inject { id, text: text.clone() });
        self.state = State::Injecting { id, text };
    }
}

/// Join per-segment transcripts with single spaces, dropping empties.
pub fn join_segments<'a>(parts: impl Iterator<Item = &'a str>) -> String {
    let mut out = String::new();
    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(part);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(mode: ActivationMode) -> Session {
        Session::new(Config { mode, ..Config::default() })
    }

    fn has(fx: &[Effect], want: &Effect) -> bool {
        fx.iter().any(|e| e == want)
    }

    /// Drive a recording that is already in Finishing to completion.
    fn complete(sess: &mut Session, id: SessionId, texts: &[&str]) -> Vec<Effect> {
        let mut all = Vec::new();
        for (i, t) in texts.iter().enumerate() {
            all.extend(sess.handle(Event::SegmentQueued { id, index: i }));
            all.extend(sess.handle(Event::SegmentText { id, index: i, text: (*t).into() }));
        }
        all.extend(sess.handle(Event::CaptureEnded { id, segments: texts.len(), audio_ms: 1000 }));
        all
    }

    #[test]
    fn hold_mode_happy_path() {
        let mut x = s(ActivationMode::Hold);
        let fx = x.handle(Event::RecordDown { at_ms: 0 });
        assert_eq!(fx[0], Effect::StartCapture { id: 1 });
        assert_eq!(fx[1], Effect::EnsureModelLoaded);
        assert!(has(&fx, &Effect::Overlay(View::Listening)));
        assert!(has(&fx, &Effect::RegisterCancelKey));

        // Even a very short hold finishes in Hold mode.
        let fx = x.handle(Event::RecordUp { at_ms: 100 });
        assert!(has(&fx, &Effect::StopCapture { id: 1 }));
        assert!(has(&fx, &Effect::UnregisterCancelKey));

        let fx = complete(&mut x, 1, &["Hello there.", " How are you?"]);
        assert!(has(&fx, &Effect::Inject { id: 1, text: "Hello there. How are you?".into() }));
        assert!(has(
            &fx,
            &Effect::SaveHistory { text: "Hello there. How are you?".into(), audio_ms: 1000 }
        ));

        let fx = x.handle(Event::InjectDone { id: 1, result: Ok(()) });
        assert!(has(&fx, &Effect::Overlay(View::Done { text: "Hello there. How are you?".into() })));
        // Idle immediately: no post-inject lockout.
        assert!(x.is_idle());
        let fx = x.handle(Event::RecordDown { at_ms: 5000 });
        assert_eq!(fx[0], Effect::StartCapture { id: 2 });
    }

    #[test]
    fn auto_repeat_down_is_ignored_while_holding() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        assert!(x.handle(Event::RecordDown { at_ms: 30 }).is_empty());
        assert!(x.is_recording());
    }

    #[test]
    fn up_before_down_is_ignored() {
        let mut x = s(ActivationMode::Hybrid);
        assert!(x.handle(Event::RecordUp { at_ms: 0 }).is_empty());
        assert!(x.is_idle());
    }

    #[test]
    fn hybrid_single_tap_cancels_after_window() {
        let mut x = s(ActivationMode::Hybrid);
        x.handle(Event::RecordDown { at_ms: 0 });
        let fx = x.handle(Event::RecordUp { at_ms: 120 });
        assert_eq!(fx, vec![Effect::StartTimer { kind: TimerKind::TapWindow, id: 1, after_ms: 350 }]);
        let fx = x.handle(Event::Timer { kind: TimerKind::TapWindow, id: 1 });
        assert!(has(&fx, &Effect::DiscardCapture { id: 1 }));
        assert!(has(&fx, &Effect::Overlay(View::Hidden)));
        assert!(x.is_idle());
    }

    #[test]
    fn hybrid_double_tap_latches_then_tap_stops() {
        let mut x = s(ActivationMode::Hybrid);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 100 });
        let fx = x.handle(Event::RecordDown { at_ms: 250 });
        assert!(has(&fx, &Effect::Overlay(View::HandsFree)));
        // Escape goes back to the focused app in hands-free by default.
        assert!(has(&fx, &Effect::UnregisterCancelKey));
        // The stale tap timer and the second release do nothing.
        assert!(x.handle(Event::Timer { kind: TimerKind::TapWindow, id: 1 }).is_empty());
        assert!(x.handle(Event::RecordUp { at_ms: 300 }).is_empty());
        assert!(x.is_recording());
        let fx = x.handle(Event::RecordDown { at_ms: 9000 });
        assert!(has(&fx, &Effect::StopCapture { id: 1 }));
        // The cancel key was already released: not unregistered twice.
        assert!(!has(&fx, &Effect::UnregisterCancelKey));
    }

    #[test]
    fn hybrid_long_hold_finishes_on_release() {
        let mut x = s(ActivationMode::Hybrid);
        x.handle(Event::RecordDown { at_ms: 0 });
        let fx = x.handle(Event::RecordUp { at_ms: 251 });
        assert!(has(&fx, &Effect::StopCapture { id: 1 }));
    }

    #[test]
    fn toggle_mode_press_press() {
        let mut x = s(ActivationMode::Toggle);
        let fx = x.handle(Event::RecordDown { at_ms: 0 });
        assert!(!has(&fx, &Effect::RegisterCancelKey));
        assert!(x.handle(Event::RecordUp { at_ms: 80 }).is_empty());
        assert!(x.is_recording());
        let fx = x.handle(Event::RecordDown { at_ms: 4000 });
        assert!(has(&fx, &Effect::StopCapture { id: 1 }));
    }

    #[test]
    fn cancel_while_recording_discards() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        let fx = x.handle(Event::Cancel);
        assert!(has(&fx, &Effect::DiscardCapture { id: 1 }));
        assert!(has(&fx, &Effect::UnregisterCancelKey));
        assert!(x.is_idle());
        // Late results from the cancelled session are ignored.
        assert!(x.handle(Event::SegmentText { id: 1, index: 0, text: "x".into() }).is_empty());
        assert!(x
            .handle(Event::CaptureEnded { id: 1, segments: 1, audio_ms: 10 })
            .is_empty());
    }

    #[test]
    fn cancel_while_finishing_discards() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 900 });
        let fx = x.handle(Event::Cancel);
        assert!(has(&fx, &Effect::DiscardCapture { id: 1 }));
        assert!(x.is_idle());
    }

    #[test]
    fn text_arriving_before_and_after_release_in_any_order() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::SegmentQueued { id: 1, index: 0 });
        x.handle(Event::SegmentText { id: 1, index: 0, text: "one".into() });
        x.handle(Event::SegmentQueued { id: 1, index: 1 });
        x.handle(Event::RecordUp { at_ms: 5000 });
        // Capture ends before the last two texts arrive, out of order.
        assert!(x
            .handle(Event::CaptureEnded { id: 1, segments: 3, audio_ms: 5000 })
            .is_empty());
        assert!(x.handle(Event::SegmentText { id: 1, index: 2, text: "three".into() }).is_empty());
        let fx = x.handle(Event::SegmentText { id: 1, index: 1, text: "two".into() });
        assert!(has(&fx, &Effect::Inject { id: 1, text: "one two three".into() }));
    }

    #[test]
    fn no_segments_means_no_speech() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 900 });
        let fx = x.handle(Event::CaptureEnded { id: 1, segments: 0, audio_ms: 900 });
        assert!(has(&fx, &Effect::Overlay(View::NoSpeech)));
        assert!(x.is_idle());
    }

    #[test]
    fn empty_transcripts_mean_no_speech() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 900 });
        let fx = complete(&mut x, 1, &["  ", ""]);
        assert!(has(&fx, &Effect::Overlay(View::NoSpeech)));
        assert!(!fx.iter().any(|e| matches!(e, Effect::Inject { .. })));
    }

    #[test]
    fn max_duration_finishes_and_warning_shows() {
        let mut x = s(ActivationMode::Toggle);
        let fx = x.handle(Event::RecordDown { at_ms: 0 });
        assert!(has(&fx, &Effect::StartTimer { kind: TimerKind::Warning, id: 1, after_ms: 585_000 }));
        assert!(has(
            &fx,
            &Effect::StartTimer { kind: TimerKind::MaxDuration, id: 1, after_ms: 600_000 }
        ));
        let fx = x.handle(Event::Timer { kind: TimerKind::Warning, id: 1 });
        assert_eq!(fx, vec![Effect::Overlay(View::StoppingSoon { remaining_ms: 15_000 })]);
        let fx = x.handle(Event::Timer { kind: TimerKind::MaxDuration, id: 1 });
        assert!(has(&fx, &Effect::StopCapture { id: 1 }));
    }

    #[test]
    fn stale_timers_from_an_old_session_are_ignored() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::Cancel);
        x.handle(Event::RecordDown { at_ms: 1000 });
        assert!(x.handle(Event::Timer { kind: TimerKind::MaxDuration, id: 1 }).is_empty());
        assert!(x.is_recording());
    }

    #[test]
    fn capture_and_model_failures_surface_and_reset() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        let fx = x.handle(Event::CaptureFailed { id: 1, message: "No microphone".into() });
        assert!(has(
            &fx,
            &Effect::Overlay(View::Error { message: "No microphone".into(), text: None })
        ));
        assert!(x.is_idle());

        x.handle(Event::RecordDown { at_ms: 10 });
        x.handle(Event::RecordUp { at_ms: 900 });
        let fx = x.handle(Event::ModelFailed { message: "Model missing".into() });
        assert!(has(&fx, &Effect::DiscardCapture { id: 2 }));
        assert!(x.is_idle());
        // A model failure while idle is not shown.
        assert!(x.handle(Event::ModelFailed { message: "x".into() }).is_empty());
    }

    #[test]
    fn inject_failure_keeps_text_for_copy() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 900 });
        complete(&mut x, 1, &["keep me"]);
        let fx = x.handle(Event::InjectDone { id: 1, result: Err("blocked".into()) });
        assert!(has(
            &fx,
            &Effect::Overlay(View::Error { message: "blocked".into(), text: Some("keep me".into()) })
        ));
        assert!(x.is_idle());
    }

    #[test]
    fn presses_during_finishing_and_injecting_are_dropped() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 900 });
        assert!(x.handle(Event::RecordDown { at_ms: 950 }).is_empty());
        complete(&mut x, 1, &["a"]);
        assert!(x.handle(Event::RecordDown { at_ms: 960 }).is_empty());
        // Cancel cannot interrupt a paste that is already in flight.
        assert!(x.handle(Event::Cancel).is_empty());
    }

    #[test]
    fn paused_blocks_start_and_pausing_aborts() {
        let mut x = s(ActivationMode::Hold);
        x.handle(Event::RecordDown { at_ms: 0 });
        let fx = x.handle(Event::SetPaused(true));
        assert!(has(&fx, &Effect::DiscardCapture { id: 1 }));
        let fx = x.handle(Event::RecordDown { at_ms: 10 });
        assert!(matches!(fx.as_slice(), [Effect::Overlay(View::Error { .. })]));
        x.handle(Event::SetPaused(false));
        assert_eq!(x.handle(Event::RecordDown { at_ms: 20 })[0], Effect::StartCapture { id: 2 });
    }

    #[test]
    fn history_can_be_disabled() {
        let mut x = Session::new(Config {
            mode: ActivationMode::Hold,
            history_enabled: false,
            ..Config::default()
        });
        x.handle(Event::RecordDown { at_ms: 0 });
        x.handle(Event::RecordUp { at_ms: 900 });
        let fx = complete(&mut x, 1, &["secret"]);
        assert!(!fx.iter().any(|e| matches!(e, Effect::SaveHistory { .. })));
        assert!(has(&fx, &Effect::Inject { id: 1, text: "secret".into() }));
    }

    #[test]
    fn join_trims_and_skips_empty() {
        assert_eq!(join_segments(["  a ", "", "b", "   "].into_iter()), "a b");
        assert_eq!(join_segments(std::iter::empty()), "");
    }
}
