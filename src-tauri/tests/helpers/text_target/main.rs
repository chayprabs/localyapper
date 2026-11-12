//! Smoke-test text target: one window with one focused multi-line text
//! field. Prints `ready` once it is shown and focused, then reads commands
//! from stdin, one per line:
//!
//! - `get`   prints the field's text as one JSON string line
//! - `focus` brings the window to the front and focuses the field
//! - `clear` empties the field
//! - `quit`  exits
//!
//! Native controls only: GTK TextView on Linux, a Win32 EDIT control on
//! Windows, NSTextView on macOS. Built only with `--features test-hooks`.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

use std::io::{BufRead, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
    Get,
    Focus,
    Clear,
    Quit,
}

/// Read stdin on a thread and hand each command to `deliver`.
pub fn spawn_stdin(deliver: impl Fn(Cmd) + Send + 'static) {
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            let cmd = match line.trim() {
                "get" => Cmd::Get,
                "focus" => Cmd::Focus,
                "clear" => Cmd::Clear,
                "quit" => Cmd::Quit,
                _ => continue,
            };
            deliver(cmd);
            if cmd == Cmd::Quit {
                break;
            }
        }
        deliver(Cmd::Quit);
    });
}

pub fn reply(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", serde_json::to_string(text).unwrap_or_default());
    let _ = out.flush();
}

pub fn ready() {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "ready");
    let _ = out.flush();
}

fn main() {
    #[cfg(target_os = "linux")]
    linux::run();
    #[cfg(windows)]
    windows::run();
    #[cfg(target_os = "macos")]
    macos::run();
}
