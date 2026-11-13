//! The foreground app, read on `StartCapture` before any overlay call.

/// Who receives the dictated text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ForegroundApp {
    /// Display name for history ("Chrome", "VS Code").
    pub name: String,
    /// Process or bundle name, for terminal detection.
    pub process: String,
    /// Windows: the window belongs to an elevated process and we are not
    /// elevated, so key events would be dropped (UIPI).
    pub blocked_by_elevation: bool,
}

pub fn foreground_app() -> ForegroundApp {
    let (process, blocked) = os_foreground();
    ForegroundApp {
        name: crate::context::detector::friendly_name(&process),
        process,
        blocked_by_elevation: blocked,
    }
}

#[cfg(windows)]
fn os_foreground() -> (String, bool) {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
        PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    /// Some(true/false) when the token could be read, None when access
    /// was refused.
    fn elevated(process: HANDLE) -> Option<bool> {
        let mut token = HANDLE::default();
        // SAFETY: standard token query on a process handle we hold; the
        // out-parameters are sized locals and the token is closed.
        unsafe {
            OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
            let mut info = TOKEN_ELEVATION::default();
            let mut len = 0u32;
            let r = GetTokenInformation(
                token,
                TokenElevation,
                Some((&mut info as *mut TOKEN_ELEVATION).cast()),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut len,
            );
            let _ = CloseHandle(token);
            r.ok()?;
            Some(info.TokenIsElevated != 0)
        }
    }

    // SAFETY: returns a window handle or null.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return ("unknown".into(), false);
    }
    let mut pid = 0u32;
    // SAFETY: writes the owning process id into a local.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        return ("unknown".into(), false);
    }
    // SAFETY: limited query rights only; the handle is closed below.
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
        return ("unknown".into(), false);
    };
    let mut buf = [0u16; 512];
    let mut len = buf.len() as u32;
    // SAFETY: bounded write into a stack buffer.
    let ok = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    };
    let target_elevated = elevated(handle);
    // SAFETY: closing the handle opened above.
    unsafe {
        let _ = CloseHandle(handle);
    }
    // SAFETY: pseudo handle for this process, no close needed.
    let self_elevated = elevated(unsafe { GetCurrentProcess() }).unwrap_or(false);
    let process = if ok.is_ok() && len > 0 {
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    } else {
        "unknown".into()
    };
    // A token we may not read belongs to an elevated or protected process.
    let blocked = !self_elevated && target_elevated.unwrap_or(true);
    (process, blocked)
}

#[cfg(target_os = "macos")]
fn os_foreground() -> (String, bool) {
    use objc2_app_kit::NSWorkspace;
    let ws = NSWorkspace::sharedWorkspace();
    let name = ws
        .frontmostApplication()
        .and_then(|app| app.localizedName())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".into());
    (name, false)
}

#[cfg(target_os = "linux")]
fn os_foreground() -> (String, bool) {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("DISPLAY").is_none() {
        return ("unknown".into(), false);
    }
    (
        x11_active_process().unwrap_or_else(|| "unknown".into()),
        false,
    )
}

/// Process id of the foreground window on X11: `_NET_ACTIVE_WINDOW` and
/// its `_NET_WM_PID`, or, without a window manager, the input focus
/// window and its nearest ancestor that has `_NET_WM_PID`.
#[cfg(target_os = "linux")]
fn x11_focus_pid() -> Option<u32> {
    use x11_dl::xlib::{self, Xlib};
    let x = Xlib::open().ok()?;
    // SAFETY: Xlib calls on a display opened and closed here; returned
    // buffers are freed with XFree.
    unsafe {
        let dpy = (x.XOpenDisplay)(std::ptr::null());
        if dpy.is_null() {
            return None;
        }
        let read_card = |win: xlib::Window, name: &std::ffi::CStr| -> Option<u64> {
            let atom = (x.XInternAtom)(dpy, name.as_ptr(), xlib::True);
            if atom == 0 {
                return None;
            }
            let mut actual_type = 0;
            let mut format = 0;
            let mut n = 0;
            let mut after = 0;
            let mut data: *mut u8 = std::ptr::null_mut();
            let st = (x.XGetWindowProperty)(
                dpy,
                win,
                atom,
                0,
                1,
                xlib::False,
                xlib::AnyPropertyType as u64,
                &mut actual_type,
                &mut format,
                &mut n,
                &mut after,
                &mut data,
            );
            let v = if st == 0 && !data.is_null() && n > 0 && format == 32 {
                Some(*(data as *const std::os::raw::c_ulong) as u64)
            } else {
                None
            };
            if !data.is_null() {
                (x.XFree)(data.cast());
            }
            v
        };
        let root = (x.XDefaultRootWindow)(dpy);
        let mut pid = read_card(root, c"_NET_ACTIVE_WINDOW")
            .filter(|w| *w != 0)
            .and_then(|w| read_card(w, c"_NET_WM_PID"));
        if pid.is_none() {
            let mut focus: xlib::Window = 0;
            let mut revert = 0;
            (x.XGetInputFocus)(dpy, &mut focus, &mut revert);
            let mut win = focus;
            for _ in 0..16 {
                if win <= 1 || win == root {
                    break;
                }
                if let Some(p) = read_card(win, c"_NET_WM_PID") {
                    pid = Some(p);
                    break;
                }
                let mut r: xlib::Window = 0;
                let mut parent: xlib::Window = 0;
                let mut children: *mut xlib::Window = std::ptr::null_mut();
                let mut count = 0u32;
                if (x.XQueryTree)(dpy, win, &mut r, &mut parent, &mut children, &mut count) == 0 {
                    break;
                }
                if !children.is_null() {
                    (x.XFree)(children.cast());
                }
                win = parent;
            }
        }
        (x.XCloseDisplay)(dpy);
        pid.map(|p| p as u32)
    }
}

#[cfg(target_os = "linux")]
fn x11_active_process() -> Option<String> {
    let pid = x11_focus_pid()?;
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    let comm = comm.trim();
    (!comm.is_empty()).then(|| comm.to_string())
}

/// Process id owning the foreground window, where the OS exposes it.
pub fn foreground_pid() -> Option<u32> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId,
        };
        // SAFETY: reads the foreground window and its owning process id.
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            (pid != 0).then_some(pid)
        }
    }
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::NSWorkspace;
        let pid = NSWorkspace::sharedWorkspace()
            .frontmostApplication()?
            .processIdentifier();
        u32::try_from(pid).ok()
    }
    #[cfg(target_os = "linux")]
    {
        x11_focus_pid()
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn os_foreground() -> (String, bool) {
    ("unknown".into(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreground_never_panics_and_is_named() {
        let app = foreground_app();
        assert!(!app.name.is_empty());
        assert!(!app.process.is_empty());
    }
}
