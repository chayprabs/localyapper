//! CI smoke suite (plan section 8A). Needs a desktop session: run with
//! `cargo test --features test-hooks --test smoke -- --test-threads=1`
//! (under `xvfb-run` on Linux). Every wait has a timeout of 20 s or less.
//!
//! Tests that send synthetic keys on macOS first check the Accessibility
//! grant; without it macOS drops the events, so they print
//! `SKIP: no Accessibility grant on runner` and pass, and
//! docs/BUILD_REPORT.md lists them.
#![cfg(feature = "test-hooks")]

use localyapper_lib::platform::inject::{self, InjectOptions, PasteMethod};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(20);

/// The native text field from tests/helpers/text_target.
struct TextTarget {
    child: Child,
    stdin: ChildStdin,
    lines: mpsc::Receiver<String>,
}

impl TextTarget {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_text_target"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("start text_target");
        let stdin = child.stdin.take().expect("stdin");
        let stdout: ChildStdout = child.stdout.take().expect("stdout");
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let t = Self {
            child,
            stdin,
            lines,
        };
        let first = t.lines.recv_timeout(WAIT).expect("text_target ready");
        assert_eq!(first, "ready");
        t
    }

    fn command(&mut self, cmd: &str) -> String {
        writeln!(self.stdin, "{cmd}").expect("write command");
        self.stdin.flush().expect("flush");
        let line = self.lines.recv_timeout(WAIT).expect("text_target reply");
        serde_json::from_str(&line).expect("json reply")
    }

    fn text(&mut self) -> String {
        self.command("get")
    }

    fn focus(&mut self) {
        self.command("focus");
        std::thread::sleep(Duration::from_millis(300));
    }

    /// Poll until the field holds `want` or 20 s pass; return the last text.
    fn wait_for(&mut self, want: &str) -> String {
        let start = Instant::now();
        loop {
            let got = self.text();
            if got == want || start.elapsed() > WAIT {
                return got;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for TextTarget {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, "quit");
        let _ = self.stdin.flush();
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
    }
}

/// macOS drops synthetic key events without the Accessibility grant.
fn keys_allowed() -> bool {
    if localyapper_lib::platform::permissions::accessibility_trusted(false) {
        return true;
    }
    println!("SKIP: no Accessibility grant on runner");
    false
}

fn opts(restore_delay_ms: u64) -> InjectOptions {
    InjectOptions {
        method: PasteMethod::Auto,
        restore_clipboard: true,
        restore_delay_ms,
        target_process: "text_target".into(),
    }
}

fn wait_clipboard(want: &str) -> Option<String> {
    let start = Instant::now();
    loop {
        let got = inject::clipboard_text();
        if got.as_deref() == Some(want) || start.elapsed() > WAIT {
            return got;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn inject_types_into_target() {
    if !keys_allowed() {
        return;
    }
    let mut t = TextTarget::spawn();
    t.focus();
    inject::inject("Hello, world.", &opts(250)).expect("inject");
    assert_eq!(t.wait_for("Hello, world."), "Hello, world.");
}

#[test]
fn inject_restores_clipboard() {
    if !keys_allowed() {
        return;
    }
    let mut t = TextTarget::spawn();
    t.focus();

    // Text on the clipboard comes back after the restore delay.
    inject::set_clipboard_text("user clipboard").expect("set clipboard");
    inject::inject("dictated words", &opts(250)).expect("inject");
    assert_eq!(t.wait_for("dictated words"), "dictated words");
    assert_eq!(
        wait_clipboard("user clipboard").as_deref(),
        Some("user clipboard")
    );

    // An image on the clipboard comes back too.
    t.command("clear");
    t.focus();
    let pixels: Vec<u8> = (0..4 * 4 * 4).map(|i| (i * 7 % 256) as u8).collect();
    inject::set_clipboard_image(arboard::ImageData {
        width: 4,
        height: 4,
        bytes: pixels.clone().into(),
    })
    .expect("set image");
    inject::inject("over an image", &opts(250)).expect("inject");
    assert_eq!(t.wait_for("over an image"), "over an image");
    let start = Instant::now();
    let mut restored = None;
    while start.elapsed() < WAIT {
        if let Some(img) = inject::clipboard_image() {
            restored = Some(img);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let img = restored.expect("image restored");
    assert_eq!((img.width, img.height), (4, 4));
    assert_eq!(img.bytes.as_ref(), pixels.as_slice());
}

#[test]
fn inject_with_modifier_down() {
    if !keys_allowed() {
        return;
    }
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    let mut t = TextTarget::spawn();
    t.focus();
    let mut enigo = Enigo::new(&Settings::default()).expect("enigo");
    enigo.key(Key::Alt, Direction::Press).expect("alt down");
    let release = std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(150));
        if let Ok(mut e) = Enigo::new(&Settings::default()) {
            let _ = e.key(Key::Alt, Direction::Release);
        }
    });
    inject::inject("plain text", &opts(250)).expect("inject");
    let _ = release.join();
    assert_eq!(t.wait_for("plain text"), "plain text");
}
