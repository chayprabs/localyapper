//! WebView overlay (plan P3, Appendix F).
//!
//! Rust builds, positions, shows and hides the window; the page renders
//! what it is sent. The window never takes focus: it is built with
//! `focusable(false)`, shown with `SW_SHOWNOACTIVATE` on Windows, turned
//! into a non-activating floating NSPanel on macOS, and kept out of the
//! taskbar and on top on Linux. While shown it takes mouse input so the
//! pill can be dragged to a new place and its buttons clicked; clicking it
//! never moves keyboard focus away from the app being dictated into.

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

/// A work area in physical pixels and its scale factor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

/// Centre of a window at `(x, y)` sized `width` x `height` (physical
/// pixels) as fractions of `area`, clamped to `[0, 1]`. The inverse of
/// `place` for `OverlayPosition::Custom`.
pub fn fraction_of(area: WorkArea, x: i32, y: i32, width: u32, height: u32) -> (f64, f64) {
    let frac = |pos: i32, size: u32, origin: i32, extent: u32| {
        if extent == 0 {
            return 0.5;
        }
        let centre = f64::from(pos) + f64::from(size) / 2.0;
        ((centre - f64::from(origin)) / f64::from(extent)).clamp(0.0, 1.0)
    };
    (
        frac(x, width, area.x, area.width),
        frac(y, height, area.y, area.height),
    )
}

/// A drag of the pill in progress: where the cursor grabbed the window
/// and where the window started, in physical pixels.
#[derive(Clone, Copy, Debug)]
struct Drag {
    grab: (f64, f64),
    origin: (i32, i32),
    moved: bool,
}

static DRAG: Mutex<Option<Drag>> = Mutex::new(None);
/// The cursor must travel this far (physical pixels) before a press on
/// the pill counts as a drag rather than a click.
const DRAG_SLOP: f64 = 4.0;

fn dragging() -> bool {
    DRAG.lock().map(|d| d.is_some()).unwrap_or(false)
}

/// The pill was pressed: remember where. Main thread only.
pub fn drag_start(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    let (Ok(cursor), Ok(pos)) = (app.cursor_position(), window.outer_position()) else {
        return;
    };
    if let Ok(mut d) = DRAG.lock() {
        *d = Some(Drag {
            grab: (cursor.x - f64::from(pos.x), cursor.y - f64::from(pos.y)),
            origin: (pos.x, pos.y),
            moved: false,
        });
    }
}

/// The cursor moved with the pill held: the window follows it. Main
/// thread only.
pub fn drag_move(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    let Ok(cursor) = app.cursor_position() else {
        return;
    };
    let Ok(mut guard) = DRAG.lock() else { return };
    let Some(drag) = guard.as_mut() else { return };
    let x = (cursor.x - drag.grab.0).round() as i32;
    let y = (cursor.y - drag.grab.1).round() as i32;
    if !drag.moved {
        let (dx, dy) = (f64::from(x - drag.origin.0), f64::from(y - drag.origin.1));
        if dx.hypot(dy) < DRAG_SLOP {
            return;
        }
        drag.moved = true;
    }
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// The pill was let go. Returns where it now sits as fractions of its
/// monitor's work area, or `None` when it was only clicked. Main thread
/// only.
pub fn drag_end(app: &AppHandle) -> Option<(f64, f64)> {
    let drag = DRAG.lock().ok()?.take()?;
    if !drag.moved {
        return None;
    }
    let window = app.get_webview_window(LABEL)?;
    let pos = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    let centre = (
        f64::from(pos.x) + f64::from(size.width) / 2.0,
        f64::from(pos.y) + f64::from(size.height) / 2.0,
    );
    let monitor = app
        .monitor_from_point(centre.0, centre.1)
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let wa = monitor.work_area();
    let area = WorkArea {
        x: wa.position.x,
        y: wa.position.y,
        width: wa.size.width,
        height: wa.size.height,
        scale: monitor.scale_factor(),
    };
    // Snap back onto the work area if it was dropped past an edge.
    let fractions = fraction_of(area, pos.x, pos.y, size.width, size.height);
    let (x, y) = place_sized(
        area,
        OverlayPosition::Custom {
            fx: fractions.0,
            fy: fractions.1,
        },
        logical_size(&window, area.scale),
    );
    if (x, y) != (pos.x, pos.y) {
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
    Some(fractions)
}

/// The window's real size in logical pixels on a monitor of `scale`, or
/// the size it was built with when the window system does not say.
fn logical_size(window: &tauri::WebviewWindow, scale: f64) -> (f64, f64) {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    match window.outer_size() {
        Ok(s) if s.width > 0 && s.height > 0 => {
            (f64::from(s.width) / scale, f64::from(s.height) / scale)
        }
        _ => (WIDTH, HEIGHT),
    }
}

/// End a drag and remember the new place as the custom position. Main
/// thread only.
pub fn finish_drag(app: &AppHandle) {
    let Some((fx, fy)) = drag_end(app) else {
        return;
    };
    let custom = format!("{fx:.4},{fy:.4}");
    for (key, value) in [
        ("overlay_custom", custom.as_str()),
        ("overlay_position", "custom"),
    ] {
        if let Err(e) = crate::commands::settings::set_setting_now(app, key, value) {
            log::warn!("overlay: could not save the dragged position: {e}");
        }
    }
}

/// A pill hidden while still held would never see the button come up, so
/// the drag ends with the hide.
fn finish_drag_on_hide(app: &AppHandle) {
    if dragging() {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || finish_drag(&handle));
    }
}

/// Top-left corner of the overlay in physical pixels. Computed in logical
/// pixels against the monitor under the cursor, then scaled.
pub fn place(area: WorkArea, position: OverlayPosition) -> (i32, i32) {
    place_sized(area, position, (WIDTH, HEIGHT))
}

/// `place` for a window whose real size (logical pixels) is known. Some
/// window systems give the window more room than was asked for (GTK
/// without a window manager makes it 200 high), and the pill sits in the
/// middle of whatever it got.
pub fn place_sized(area: WorkArea, position: OverlayPosition, size: (f64, f64)) -> (i32, i32) {
    let (w, h) = size;
    let scale = if area.scale > 0.0 { area.scale } else { 1.0 };
    let (aw, ah) = (area.width as f64 / scale, area.height as f64 / scale);
    let centered_x = (aw - w) / 2.0;
    let (lx, ly) = match position {
        OverlayPosition::TopCenter => (centered_x, MARGIN),
        OverlayPosition::Custom { fx, fy } if fx.is_finite() && fy.is_finite() => {
            // Kept whole on the work area: a spot near an edge, or one
            // saved on a larger monitor, slides in instead of being lost.
            let x = (fx * aw - w / 2.0).clamp(0.0, (aw - w).max(0.0));
            let y = (fy * ah - h / 2.0).clamp(0.0, (ah - h).max(0.0));
            (x, y)
        }
        OverlayPosition::Custom { .. } => (centered_x, ah - h - MARGIN),
        OverlayPosition::BottomCenter => (centered_x, ah - h - MARGIN),
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
        // macOS: the first click on a window that is not key reaches the
        // page, so the pill can be dragged and its buttons work at once.
        .accept_first_mouse(true)
        .visible(false)
        .inner_size(WIDTH, HEIGHT);
    #[cfg(feature = "test-hooks")]
    {
        builder = builder.initialization_script("window.__LY_TEST__ = true;");
    }
    #[allow(unused_variables)]
    let window = builder.build()?;
    // Mouse input is turned on each time the window is shown: on Linux
    // tao panics if it is changed before GTK has realized the window.

    #[cfg(target_os = "macos")]
    {
        use tauri_nspanel::{CollectionBehavior, PanelLevel, StyleMask, WebviewWindowExt};
        let panel = window
            .to_panel::<OverlayPanel>()
            .map_err(|e| tauri::Error::Io(std::io::Error::other(format!("overlay panel: {e}"))))?;
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

    fn show_window(&self) {
        self.visible.store(true, Ordering::Release);
        let position = self.config().position;
        // Monitor queries build GDK objects on the calling thread, which
        // corrupts GTK's X connection off the main thread (the Linux app
        // then exits), so the whole show runs on the main thread.
        let app = self.app.clone();
        let _ = self.app.run_on_main_thread(move || {
            let Some(window) = app.get_webview_window(LABEL) else {
                return;
            };
            // A pill being dragged stays under the cursor when the view
            // changes (for example the key is released mid-drag).
            if !dragging() {
                if let Some(area) = work_area(&app) {
                    let size = logical_size(&window, area.scale);
                    let (x, y) = place_sized(area, position, size);

                    let _ = window.set_position(PhysicalPosition::new(x, y));
                }
            }
            show_no_activate(&app, &window);
            let _ = window.set_ignore_cursor_events(false);
        });
    }

    fn hide_window(&self) {
        self.visible.store(false, Ordering::Release);
        finish_drag_on_hide(&self.app);
        if let Some(window) = self.app.get_webview_window(LABEL) {
            hide_window(&self.app, &window);
        }
    }

    /// The overlay's on-screen rectangle, if shown. Must not be called
    /// on the main thread (it waits for it).
    pub fn rect(&self) -> Option<OverlayRect> {
        if !self.visible.load(Ordering::Acquire) {
            return None;
        }
        on_main_thread(&self.app, |app| {
            let w = app.get_webview_window(LABEL)?;
            let p = w.outer_position().ok()?;
            let s = w.outer_size().ok()?;
            Some(OverlayRect {
                x: p.x,
                y: p.y,
                width: s.width,
                height: s.height,
                scale: w.scale_factor().unwrap_or(1.0),
            })
        })
        .flatten()
    }
}

/// The work area of the monitor under the cursor, or the primary one.
/// Main thread only.
fn work_area(app: &AppHandle) -> Option<WorkArea> {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let wa = monitor.work_area();
    Some(WorkArea {
        x: wa.position.x,
        y: wa.position.y,
        width: wa.size.width,
        height: wa.size.height,
        scale: monitor.scale_factor(),
    })
}

/// Run `f` on the main thread and wait up to two seconds for its result.
fn on_main_thread<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce(&AppHandle) -> T + Send + 'static,
) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(f(&handle));
    })
    .ok()?;
    rx.recv_timeout(std::time::Duration::from_secs(2)).ok()
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
        self.show_window();
        if let Some(after) = hide {
            let (gen, app) = (self.generation.clone(), self.app.clone());
            let visible = self.visible.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(after).await;
                if gen.load(Ordering::Acquire) == generation {
                    visible.store(false, Ordering::Release);
                    finish_drag_on_hide(&app);
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

    fn release(&self) {
        // On macOS the overlay is an NSPanel (tauri-nspanel), which aborts
        // the process when its window is destroyed and rebuilt, so there
        // the overlay stays alive.
        if cfg!(target_os = "macos") {
            return;
        }
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.visible.store(false, Ordering::Release);
        finish_drag_on_hide(&self.app);
        let app = self.app.clone();
        let _ = self.app.run_on_main_thread(move || {
            if let Some(w) = app.get_webview_window(LABEL) {
                if let Err(e) = w.destroy() {
                    log::warn!("overlay: could not release the window: {e}");
                } else {
                    log::info!("overlay: window released after idle");
                }
            }
        });
    }

    fn ensure(&self) {
        if self.app.get_webview_window(LABEL).is_some() {
            return;
        }
        // Queued on the main thread ahead of any show, which then finds
        // the window; the page asks for the current view once loaded.
        let app = self.app.clone();
        let _ = self.app.run_on_main_thread(move || {
            if app.get_webview_window(LABEL).is_none() {
                match build(&app) {
                    Ok(()) => log::info!("overlay: window recreated"),
                    Err(e) => log::error!("overlay: could not recreate the window: {e}"),
                }
            }
        });
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
    fn custom_is_clamped_onto_the_work_area() {
        let inside = place(AREA, OverlayPosition::Custom { fx: 0.5, fy: 0.5 });
        assert_eq!(inside, (960 - 170, 520 - 48));
        let off = place(AREA, OverlayPosition::Custom { fx: 1.5, fy: 0.5 });
        assert_eq!(off, (1920 - 340, 520 - 48));
        let corner = place(AREA, OverlayPosition::Custom { fx: 0.0, fy: 0.0 });
        assert_eq!(corner, (0, 0));
        let far = place(AREA, OverlayPosition::Custom { fx: 1.0, fy: 1.0 });
        assert_eq!(far, (1920 - 340, 1040 - 96));
        let nan = place(
            AREA,
            OverlayPosition::Custom {
                fx: f64::NAN,
                fy: 0.5,
            },
        );
        assert_eq!(nan, place(AREA, OverlayPosition::BottomCenter));
    }

    #[test]
    fn a_dragged_position_round_trips_through_its_fractions() {
        let area = WorkArea {
            x: 1920,
            y: -200,
            width: 2880,
            height: 1800,
            scale: 2.0,
        };
        // The window is 340x96 logical, so 680x192 physical at scale 2.
        for (x, y) in [(1920, -200), (3000, 500), (1920 + 2880 - 680, 1600 - 192)] {
            let (fx, fy) = fraction_of(area, x, y, 680, 192);
            assert_eq!(place(area, OverlayPosition::Custom { fx, fy }), (x, y));
            // Stored with four decimals: still within a pixel or two.
            let round = |v: f64| (v * 10_000.0).round() / 10_000.0;
            let (rx, ry) = place(
                area,
                OverlayPosition::Custom {
                    fx: round(fx),
                    fy: round(fy),
                },
            );
            assert!((rx - x).abs() <= 1 && (ry - y).abs() <= 1);
        }
        // Dropped past the edge: the fractions clamp, and so does place.
        let (fx, fy) = fraction_of(area, 0, 5000, 680, 192);
        assert_eq!((fx, fy), (0.0, 1.0));
        assert_eq!(
            place(area, OverlayPosition::Custom { fx, fy }),
            (1920, -200 + 1800 - 192)
        );
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
        assert_eq!(hide_after(&View::Listening, true), None);
    }
}
