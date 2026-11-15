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
    /// A short message that is not part of a dictation (the app has
    /// started and is waiting in the tray).
    Notice {
        message: String,
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

/// Where the pill sits (setting `overlay_position`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum OverlayPosition {
    #[default]
    BottomCenter,
    TopCenter,
    /// Centre of the pill as a fraction of the work area.
    Custom {
        fx: f64,
        fy: f64,
    },
}

impl OverlayPosition {
    /// Parse `bottom_center`, `top_center` or `custom` with its
    /// `overlay_custom` value ("0.5000,0.8500").
    pub fn parse(position: &str, custom: Option<&str>) -> Option<Self> {
        match position {
            "bottom_center" => Some(Self::BottomCenter),
            "top_center" => Some(Self::TopCenter),
            "custom" => {
                let (x, y) = custom?.split_once(',')?;
                Some(Self::Custom {
                    fx: x.trim().parse().ok()?,
                    fy: y.trim().parse().ok()?,
                })
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayConfig {
    pub enabled: bool,
    pub position: OverlayPosition,
    /// Keep `Done` with its text on screen for 1.2 s.
    pub preview: bool,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: OverlayPosition::BottomCenter,
            preview: true,
        }
    }
}

/// The overlay's on-screen rectangle in physical pixels and its scale.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct OverlayRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

pub trait OverlayBackend: Send + Sync {
    fn show(&self, view: &View);
    /// Microphone level in `[0, 1]`, about 30 times a second.
    fn level(&self, v: f32);
    fn hide(&self);
    /// Show a short message for a few seconds.
    fn notice(&self, _message: &str) {}
    fn configure(&self, _config: OverlayConfig) {}
    /// Where the overlay is on screen, if shown.
    fn rect(&self) -> Option<OverlayRect> {
        None
    }
    /// Destroy the window to free its WebView after a long idle period
    /// (plan P6.3).
    fn release(&self) {}
    /// Recreate the window in the background if it was released. Never
    /// blocks: a recording starts without waiting for it.
    fn ensure(&self) {}
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
    fn positions_parse() {
        assert_eq!(
            OverlayPosition::parse("top_center", None),
            Some(OverlayPosition::TopCenter)
        );
        assert_eq!(
            OverlayPosition::parse("custom", Some("0.25, 0.75")),
            Some(OverlayPosition::Custom { fx: 0.25, fy: 0.75 })
        );
        assert_eq!(OverlayPosition::parse("custom", None), None);
        assert_eq!(OverlayPosition::parse("left", None), None);
    }

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
