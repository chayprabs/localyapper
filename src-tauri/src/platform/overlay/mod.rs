//! Overlay back ends. Rust owns the window: it positions, shows and hides
//! it. The page only renders the view it is sent.

pub mod webview;

use crate::core::session::View;
use serde::Serialize;

/// The serialised form of `session::View` sent to the page as
/// `overlay-view`.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ViewPayload {
    Hidden,
    Listening,
    HandsFree,
    StoppingSoon {
        remaining_ms: u64,
    },
    Processing,
    Done {
        text: String,
    },
    NoSpeech,
    Error {
        message: String,
        text: Option<String>,
    },
}

impl From<&View> for ViewPayload {
    fn from(v: &View) -> Self {
        match v {
            View::Hidden => Self::Hidden,
            View::Listening => Self::Listening,
            View::HandsFree => Self::HandsFree,
            View::StoppingSoon { remaining_ms } => Self::StoppingSoon {
                remaining_ms: *remaining_ms,
            },
            View::Processing => Self::Processing,
            View::Done { text } => Self::Done { text: text.clone() },
            View::NoSpeech => Self::NoSpeech,
            View::Error { message, text } => Self::Error {
                message: message.clone(),
                text: text.clone(),
            },
        }
    }
}

pub trait OverlayBackend: Send + Sync {
    fn show(&self, view: &View);
    /// Microphone level in `[0, 1]`, about 30 times a second.
    fn level(&self, v: f32);
    fn hide(&self);
}

/// An overlay that draws nothing (overlay disabled, headless tests).
pub struct NoOverlay;

impl OverlayBackend for NoOverlay {
    fn show(&self, _view: &View) {}
    fn level(&self, _v: f32) {}
    fn hide(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_is_tagged() {
        let json = serde_json::to_string(&ViewPayload::from(&View::StoppingSoon {
            remaining_ms: 15_000,
        }))
        .unwrap_or_default();
        assert_eq!(json, r#"{"kind":"stopping_soon","remaining_ms":15000}"#);
        let json = serde_json::to_string(&ViewPayload::from(&View::Error {
            message: "m".into(),
            text: None,
        }))
        .unwrap_or_default();
        assert_eq!(json, r#"{"kind":"error","message":"m","text":null}"#);
    }
}
