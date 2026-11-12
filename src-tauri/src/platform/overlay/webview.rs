//! WebView overlay. Sends `overlay-view` and `overlay-level` to the
//! `overlay` window only, and shows or hides the window from Rust.

use super::{OverlayBackend, ViewPayload};
use crate::core::session::View;
use tauri::{AppHandle, Emitter, Manager};

pub const LABEL: &str = "overlay";

pub struct WebviewOverlay {
    app: AppHandle,
}

impl WebviewOverlay {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl OverlayBackend for WebviewOverlay {
    fn show(&self, view: &View) {
        if *view == View::Hidden {
            self.hide();
            return;
        }
        let _ = self
            .app
            .emit_to(LABEL, "overlay-view", ViewPayload::from(view));
        if let Some(w) = self.app.get_webview_window(LABEL) {
            let _ = w.show();
        }
    }

    fn level(&self, v: f32) {
        let _ = self.app.emit_to(LABEL, "overlay-level", v);
    }

    fn hide(&self) {
        let _ = self.app.emit_to(LABEL, "overlay-view", ViewPayload::Hidden);
        if let Some(w) = self.app.get_webview_window(LABEL) {
            let _ = w.hide();
        }
    }
}
