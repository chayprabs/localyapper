//! Win32 window with a multi-line EDIT control.

use crate::{ready, reply, spawn_stdin, Cmd};
use std::sync::atomic::{AtomicIsize, Ordering};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::*;

const WM_CMD: u32 = WM_APP + 1;
static EDIT: AtomicIsize = AtomicIsize::new(0);
static MAIN: AtomicIsize = AtomicIsize::new(0);

fn edit() -> HWND {
    HWND(EDIT.load(Ordering::Acquire) as *mut _)
}

fn bring_to_front(hwnd: HWND) {
    // SAFETY: plain window-management calls on our own windows.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(edit());
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // SAFETY: standard window procedure; only touches our own windows.
    unsafe {
        match msg {
            WM_SIZE => {
                let w = (lp.0 & 0xFFFF) as i32;
                let h = ((lp.0 >> 16) & 0xFFFF) as i32;
                let _ = MoveWindow(edit(), 0, 0, w, h, true);
                LRESULT(0)
            }
            WM_SETFOCUS => {
                let _ = SetFocus(edit());
                LRESULT(0)
            }
            WM_CMD => {
                match wp.0 {
                    0 => {
                        let len = GetWindowTextLengthW(edit()).max(0) as usize;
                        let mut buf = vec![0u16; len + 1];
                        let n = GetWindowTextW(edit(), &mut buf).max(0) as usize;
                        reply(&String::from_utf16_lossy(&buf[..n]).replace("\r\n", "\n"));
                    }
                    1 => {
                        bring_to_front(hwnd);
                        reply("");
                    }
                    2 => {
                        let _ = SetWindowTextW(edit(), w!(""));
                        reply("");
                    }
                    _ => PostQuitMessage(0),
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

pub fn run() {
    // SAFETY: window class registration, creation and the message loop.
    unsafe {
        let Ok(module) = GetModuleHandleW(PCWSTR::null()) else {
            std::process::exit(2);
        };
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: module.into(),
            lpszClassName: w!("LocalYapperTextTarget"),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            ..Default::default()
        };
        RegisterClassW(&class);
        let Ok(main) = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("LocalYapperTextTarget"),
            w!("LocalYapper text target"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            200,
            200,
            480,
            240,
            None,
            None,
            HINSTANCE::from(module),
            None,
        ) else {
            std::process::exit(2);
        };
        MAIN.store(main.0 as isize, Ordering::Release);
        let style = WS_CHILD
            | WS_VISIBLE
            | WS_VSCROLL
            | WINDOW_STYLE((ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN) as u32);
        let Ok(e) = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            w!("EDIT"),
            w!(""),
            style,
            0,
            0,
            480,
            240,
            main,
            None,
            HINSTANCE::from(module),
            None,
        ) else {
            std::process::exit(2);
        };
        EDIT.store(e.0 as isize, Ordering::Release);
        bring_to_front(main);

        spawn_stdin(|cmd| {
            let code = match cmd {
                Cmd::Get => 0,
                Cmd::Focus => 1,
                Cmd::Clear => 2,
                Cmd::Quit => 3,
            };
            let main = HWND(MAIN.load(Ordering::Acquire) as *mut _);
            // Posting to our own window from the stdin thread (inside the
            // unsafe block above).
            let _ = PostMessageW(main, WM_CMD, WPARAM(code), LPARAM(0));
        });
        ready();
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
