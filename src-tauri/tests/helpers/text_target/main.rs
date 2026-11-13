//! Smoke-test text target: one window with one focused multi-line text
//! field. Started with `--port N`, it connects to 127.0.0.1:N (the test),
//! sends `ready <pid>` once it is shown and focused, then reads commands,
//! one per line (TCP rather than stdio so that on macOS it can be launched
//! through LaunchServices as a normal, frontmost app):
//!
//! - `get`   prints the field's text as one JSON string line
//! - `focus` brings the window to the front and focuses the field
//! - `clear` empties the field
//! - `cover` resizes the window to fill the screen
//! - `clicks` prints how many left clicks the window received
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
    Cover,
    Clicks,
    Quit,
}

/// Left clicks received by the window.
pub static CLICKS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn clicks() -> String {
    CLICKS
        .load(std::sync::atomic::Ordering::Relaxed)
        .to_string()
}

static CONN: std::sync::Mutex<Option<std::net::TcpStream>> = std::sync::Mutex::new(None);

fn connect() -> std::net::TcpStream {
    let mut args = std::env::args();
    let port: u16 = loop {
        match args.next().as_deref() {
            Some("--port") => break args.next().and_then(|p| p.parse().ok()).unwrap_or(0),
            Some(_) => continue,
            None => break 0,
        }
    };
    match std::net::TcpStream::connect(("127.0.0.1", port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("text_target: cannot reach the test on port {port}: {e}");
            std::process::exit(2);
        }
    }
}

/// Read commands from the test on a thread and hand each to `deliver`.
pub fn spawn_stdin(deliver: impl Fn(Cmd) + Send + 'static) {
    let stream = connect();
    let reader = match stream.try_clone() {
        Ok(r) => r,
        Err(_) => std::process::exit(2),
    };
    if let Ok(mut g) = CONN.lock() {
        *g = Some(stream);
    }
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(reader).lines() {
            let Ok(line) = line else { break };
            let cmd = match line.trim() {
                "get" => Cmd::Get,
                "focus" => Cmd::Focus,
                "clear" => Cmd::Clear,
                "cover" => Cmd::Cover,
                "clicks" => Cmd::Clicks,
                "quit" => Cmd::Quit,
                _ => continue,
            };
            deliver(cmd);
            if cmd == Cmd::Quit {
                return;
            }
        }
        deliver(Cmd::Quit);
    });
}

fn send(line: &str) {
    if let Ok(mut g) = CONN.lock() {
        if let Some(s) = g.as_mut() {
            let _ = writeln!(s, "{line}");
            let _ = s.flush();
        }
    }
}

pub fn reply(text: &str) {
    send(&serde_json::to_string(text).unwrap_or_default());
}

pub fn ready() {
    send(&format!("ready {}", std::process::id()));
}

fn main() {
    #[cfg(target_os = "linux")]
    linux::run();
    #[cfg(windows)]
    windows::run();
    #[cfg(target_os = "macos")]
    macos::run();
}
