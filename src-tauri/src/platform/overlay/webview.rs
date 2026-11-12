//! WebView overlay (plan P3, Appendix F).
//!
//! Rust builds, positions, shows and hides the window; the page renders
//! what it is sent. The window never takes focus: it is built with
//! `focusable(false)`, shown with `SW_SHOWNOACTIVATE` on Windows, turned
//! into a non-activating floating NSPanel on macOS, and kept out of the
//! taskbar and on top on Linux. It ignores the mouse except while an
//! error with a copy button is shown.

use super::{OverlayBackend, OverlayConfig, OverlayPosition, OverlayRect, ViewPayload};
use crate::core::session::View;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "overlay";
/// Window size in logical pixels: the pill plus room for its shadow.
pub const WIDTH: f64 = 340.0;
pub const HEIGHT: f64 = 96.0;
/// Distance from the work-area edge in logical pixels.
pub const MARGIN: f64 = 48.0;

/// How long a view stays before the overlay hides itself.
pub fn hide_after(view: &View, preview: bool) -> Option<Duration> {
    match view {
        View::Done { .. } if preview => Some(Duration::from_millis(1200)),
        View::Done { .. } => Some(Duration::ZERO),
        View::NoSpeech => Some(Duration::from_millis(1500)),
        // An error without text has no button to dismiss it.
        View::Error { text: None, .. } => Some(Duration::from_millis(4000)),
        _ => None,
    }
}

/// Only an error with text has a button, so only then may the pill
/// receive clicks.
pub fn wants_clicks(view: &View) -> bool {
    matches!(view, View::Error { text: Some(_), .. })
}

/// A work area in physical pixels and its scale factor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

/// Top-left corner of the overlay in physical pixels. Computed in logical
/// pixels against the monitor under the cursor, then scaled.
pub fn place(area: WorkArea, position: OverlayPosition) -> (i32, i32) {
    let scale = if area.scale > 0.0 { area.scale } else { 1.0 };
    let (aw, ah) = (area.width as f64 / scale, area.height as f64 / scale);
    let centered_x = (aw - WIDTH) / 2.0;
    let (lx, ly) = match position {
        OverlayPosition::TopCenter => (centered_x, MARGIN),
        OverlayPosition::Custom { fx, fy } => {
            let x = fx * aw - WIDTH / 2.0;
            let y = fy * ah - HEIGHT / 2.0;
            let on_screen = (0.0..=1.0).contains(&fx)
                && (0.0..=1.0).contains(&fy)
                && x >= 0.0
                && y >= 0.0
                && x + WIDTH <= aw
                && y + HEIGHT <= ah;
            if on_screen {
                (x, y)
            } else {
                (centered_x, ah - HEIGHT - MARGIN)
            }
        }
        OverlayPosition::BottomCenter => (centered_x, ah - HEIGHT - MARGIN),
    };
    (
        area.x + (lx * scale).round() as i32,
        area.y + (ly * scale).round() as i32,
    )
}

#[cfg(target_os = "macos")]
tauri_nspanel::tauri_panel! {
    panel!(OverlayPanel {
        config: {
            can_become_key_window: false,
            can_become_main_window: false,
            is_floating_panel: true
        }
    })
}

/// Build the overlay window. Called once at start-up.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    #[allow(unused_mut)]
    let mut builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("overlay.html".into()))
        .title("LocalYapper Overlay")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .focusable(false)
        .focused(false)
        .visible(false)
        .inner_size(WIDTH, HEIGHT);
    #[cfg(feature = "test-hooks")]
    {
        builder = builder.initialization_script("window.__LY_TEST__ = true;");
    }
    #[allow(unused_variables)]
    let window = builder.build()?;
    // Click-through is set each time the window is shown: on Linux tao
    // panics if it is set before GTK has realized the window.
    #[cfg(not(target_os = "linux"))]
    window.set_ignore_cursor_events(true)?;

    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::{CollectionBehavior, PanelLevel, StyleMask, WebviewWindowExt};
        let panel = window
            .to_panel::<OverlayPanel>()
            .map_err(|e| tauri::Error::Anyhow(anyhow::anyhow!("overlay panel: {e}")))?;
        panel.set_level(PanelLevel::Floating.value());
        let _ = panel.set_style_mask(StyleMask::empty().nonactivating_panel().borderless().into());
        panel.set_collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary()
                .stationary()
                .into(),
        );
        panel.set_hides_on_deactivate(false);
    }
    Ok(())
}

pub struct WebviewOverlay {
    app: AppHandle,
    config: Mutex<OverlayConfig>,
    /// Bumped on every show/hide so stale hide timers do nothing.
    generation: Arc<AtomicU64>,
    visible: Arc<AtomicBool>,
}

impl WebviewOverlay {
    pub fn new(app: AppHandle, config: OverlayConfig) -> Self {
        Self {
            app,
            config: Mutex::new(config),
            generation: Arc::new(AtomicU64::new(0)),
            visible: Arc::new(AtomicBool::new(false)),
        }
    }

    fn config(&self) -> OverlayConfig {
        self.config.lock().map(|c| *c).unwrap_or_default()
    }

    fn work_area(&self) -> Option<WorkArea> {
        let monitor = self
            .app
            .cursor_position()
            .ok()
            .and_then(|p| self.app.monitor_from_point(p.x, p.y).ok().flatten())
            .or_else(|| self.app.primary_monitor().ok().flatten())?;
        let wa = monitor.work_area();
        Some(WorkArea {
            x: wa.position.x,
            y: wa.position.y,
            width: wa.size.width,
            height: wa.size.height,
            scale: monitor.scale_factor(),
        })
    }

    fn show_window(&self, clicks: bool) {
        let Some(window) = self.app.get_webview_window(LABEL) else {
            return;
        };
        if let Some(area) = self.work_area() {
            let (x, y) = place(area, self.config().position);
            let _ = window.set_position(PhysicalPosition::new(x, y));
        }
        self.visible.store(true, Ordering::Release);
        show_no_activate(&self.app, &window);
        let _ = window.set_ignore_cursor_events(!clicks);
    }

    fn hide_window(&self) {
        self.visible.store(false, Ordering::Release);
        if let Some(window) = self.app.get_webview_window(LABEL) {
            hide_window(&self.app, &window);
        }
    }

    /// The overlay's on-screen rectangle, if shown.
    pub fn rect(&self) -> Option<OverlayRect> {
        if !self.visible.load(Ordering::Acquire) {
            return None;
        }
        let w = self.app.get_webview_window(LABEL)?;
        let p = w.outer_position().ok()?;
        let s = w.outer_size().ok()?;
        Some(OverlayRect {
            x: p.x,
            y: p.y,
            width: s.width,
            height: s.height,
            scale: w.scale_factor().unwrap_or(1.0),
        })
    }
}

#[cfg(windows)]
fn show_no_activate(_app: &AppHandle, w: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        SWP_SHOWWINDOW, SW_SHOWNOACTIVATE,
    };
    let Ok(h) = w.hwnd() else { return };
    let hwnd = HWND(h.0 as _);
    // SAFETY: plain window-management calls on our own window handle.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
}

#[cfg(target_os = "macos")]
fn show_no_activate(app: &AppHandle, _w: &tauri::WebviewWindow) {
    use tauri_nspanel::ManagerExt;
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Ok(panel) = handle.get_webview_panel(LABEL) {
            // orderFrontRegardless: shown without activating the app.
            panel.show();
        }
    });
}

#[cfg(not(any(windows, target_os = "macos")))]
fn show_no_activate(_app: &AppHandle, w: &tauri::WebviewWindow) {
    let _ = w.show();
}

#[cfg(target_os = "macos")]
fn hide_window(app: &AppHandle, _w: &tauri::WebviewWindow) {
    use tauri_nspanel::ManagerExt;
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Ok(panel) = handle.get_webview_panel(LABEL) {
            panel.hide();
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn hide_window(_app: &AppHandle, w: &tauri::WebviewWindow) {
    let _ = w.hide();
}

impl OverlayBackend for WebviewOverlay {
    fn show(&self, view: &View) {
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let cfg = self.config();
        if *view == View::Hidden || !cfg.enabled {
            self.hide();
            return;
        }
        let _ = self
            .app
            .emit_to(LABEL, "overlay-view", ViewPayload::from(view));
        let hide = hide_after(view, cfg.preview);
        if hide == Some(Duration::ZERO) {
            self.hide();
            return;
        }
        self.show_window(wants_clicks(view));
        if let Some(after) = hide {
            let (gen, app) = (self.generation.clone(), self.app.clone());
            let visible = self.visible.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(after).await;
                if gen.load(Ordering::Acquire) == generation {
                    visible.store(false, Ordering::Release);
                    let _ = app.emit_to(LABEL, "overlay-view", ViewPayload::Hidden);
                    if let Some(w) = app.get_webview_window(LABEL) {
                        hide_window(&app, &w);
                    }
                }
            });
        }
    }

    fn level(&self, v: f32) {
        if self.visible.load(Ordering::Acquire) {
            let _ = self.app.emit_to(LABEL, "overlay-level", v);
        }
    }

    fn hide(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        let _ = self.app.emit_to(LABEL, "overlay-view", ViewPayload::Hidden);
        self.hide_window();
    }

    fn configure(&self, config: OverlayConfig) {
        if let Ok(mut c) = self.config.lock() {
            *c = config;
        }
        if !config.enabled {
            self.hide();
        }
    }

    fn rect(&self) -> Option<OverlayRect> {
        WebviewOverlay::rect(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: WorkArea = WorkArea {
        x: 0,
        y: 0,
        width: 1920,
        height: 1040,
        scale: 1.0,
    };

    #[test]
    fn places_bottom_and_top_center_with_margin() {
        assert_eq!(
            place(AREA, OverlayPosition::BottomCenter),
            (790, 1040 - 96 - 48)
        );
        assert_eq!(place(AREA, OverlayPosition::TopCenter), (790, 48));
    }

    #[test]
    fn scales_and_offsets_for_hidpi_secondary_monitors() {
        let area = WorkArea {
            x: 1920,
            y: -200,
            width: 2880,
            height: 1800,
            scale: 2.0,
        };
        // 1440x900 logical; centred 340-wide pill at x = 550 logical.
        assert_eq!(
            place(area, OverlayPosition::TopCenter),
            (1920 + 1100, -200 + 96)
        );
    }

    #[test]
    fn custom_falls_back_when_off_screen() {
        let inside = place(AREA, OverlayPosition::Custom { fx: 0.5, fy: 0.5 });
        assert_eq!(inside, (960 - 170, 520 - 48));
        let off = place(AREA, OverlayPosition::Custom { fx: 1.5, fy: 0.5 });
        assert_eq!(off, place(AREA, OverlayPosition::BottomCenter));
        let edge = place(AREA, OverlayPosition::Custom { fx: 0.0, fy: 0.0 });
        assert_eq!(edge, place(AREA, OverlayPosition::BottomCenter));
    }

    #[test]
    fn hide_timers_follow_the_plan() {
        let done = View::Done { text: "x".into() };
        assert_eq!(hide_after(&done, true), Some(Duration::from_millis(1200)));
        assert_eq!(hide_after(&done, false), Some(Duration::ZERO));
        assert_eq!(
            hide_after(&View::NoSpeech, true),
            Some(Duration::from_millis(1500))
        );
        let err_text = View::Error {
            message: "m".into(),
            text: Some("t".into()),
        };
        assert_eq!(hide_after(&err_text, true), None);
        assert!(wants_clicks(&err_text));
        assert!(!wants_clicks(&View::Listening));
        assert_eq!(hide_after(&View::Listening, true), None);
    }
}
