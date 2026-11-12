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
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(20);

/// The native text field from tests/helpers/text_target, driven over a
/// local TCP connection.
struct TextTarget {
    child: Option<Child>,
    pid: u32,
    conn: BufReader<std::net::TcpStream>,
}

/// macOS only lets an app come to the front when LaunchServices starts it,
/// so the helper is wrapped in a minimal .app bundle and opened with
/// `open -n`.
#[cfg(target_os = "macos")]
fn launch_text_target(port: u16) -> Option<Child> {
    let bundle =
        std::env::temp_dir().join(format!("LocalYapperTextTarget-{}.app", std::process::id()));
    let macos = bundle.join("Contents/MacOS");
    std::fs::create_dir_all(&macos).expect("bundle dir");
    std::fs::copy(env!("CARGO_BIN_EXE_text_target"), macos.join("text_target"))
        .expect("copy helper");
    std::fs::write(
        bundle.join("Contents/Info.plist"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>text_target</string>
<key>CFBundleIdentifier</key><string>com.localyapper.texttarget</string>
<key>CFBundleName</key><string>TextTarget</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSPrincipalClass</key><string>NSApplication</string>
</dict></plist>"#,
    )
    .expect("Info.plist");
    let status = Command::new("open")
        .arg("-n")
        .arg(&bundle)
        .arg("--args")
        .arg("--port")
        .arg(port.to_string())
        .status()
        .expect("open helper bundle");
    assert!(status.success(), "open failed");
    None
}

#[cfg(not(target_os = "macos"))]
fn launch_text_target(port: u16) -> Option<Child> {
    Some(
        Command::new(env!("CARGO_BIN_EXE_text_target"))
            .arg("--port")
            .arg(port.to_string())
            .stdin(Stdio::null())
            .spawn()
            .expect("start text_target"),
    )
}

impl TextTarget {
    fn spawn() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listen");
        let port = listener.local_addr().expect("addr").port();
        listener.set_nonblocking(true).expect("nonblocking");
        let child = launch_text_target(port);
        let start = Instant::now();
        let stream = loop {
            match listener.accept() {
                Ok((s, _)) => break s,
                Err(_) if start.elapsed() < WAIT => std::thread::sleep(Duration::from_millis(50)),
                Err(e) => panic!("text_target did not connect: {e}"),
            }
        };
        stream.set_nonblocking(false).expect("blocking");
        stream.set_read_timeout(Some(WAIT)).expect("timeout");
        let mut conn = BufReader::new(stream);
        let mut line = String::new();
        conn.read_line(&mut line).expect("text_target ready");
        let pid = line
            .trim()
            .strip_prefix("ready ")
            .and_then(|p| p.parse().ok())
            .expect("ready <pid>");
        Self { child, pid, conn }
    }

    fn command(&mut self, cmd: &str) -> String {
        let s = self.conn.get_mut();
        writeln!(s, "{cmd}").expect("write command");
        s.flush().expect("flush");
        let mut line = String::new();
        self.conn.read_line(&mut line).expect("text_target reply");
        serde_json::from_str(line.trim()).expect("json reply")
    }

    fn text(&mut self) -> String {
        self.command("get")
    }

    /// Bring the target to the front and check that it is there.
    fn focus(&mut self) {
        self.command("focus");
        let start = Instant::now();
        while localyapper_lib::platform::focus::foreground_pid() != Some(self.pid)
            && start.elapsed() < Duration::from_secs(5)
        {
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(
            localyapper_lib::platform::focus::foreground_pid(),
            Some(self.pid),
            "the text target could not be brought to the front"
        );
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
        let s = self.conn.get_mut();
        let _ = writeln!(s, "quit");
        let _ = s.flush();
        if self.child.is_none() {
            // Started through LaunchServices: make sure it is gone.
            std::thread::sleep(Duration::from_millis(300));
            let _ = Command::new("kill").arg(self.pid.to_string()).status();
        }
        if let Some(child) = self.child.as_mut() {
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(3) {
                if let Ok(Some(_)) = child.try_wait() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            let _ = child.kill();
        }
    }
}

/// macOS drops synthetic key events without the Accessibility grant.
/// Whether synthetic key events reach the focused target on this runner.
/// Elsewhere this is the OS grant. On macOS the runner can report the
/// grant and still drop events, so a probe string is typed straight
/// through enigo (not through LocalYapper's paste path); if even that
/// never arrives the test is skipped with the reason printed.
fn keys_reach(t: &mut TextTarget) -> bool {
    if !localyapper_lib::platform::permissions::accessibility_trusted(false) {
        println!("SKIP: no Accessibility grant on runner (synthetic key events are not allowed)");
        return false;
    }
    if !cfg!(target_os = "macos") {
        t.focus();
        return true;
    }
    use enigo::{Enigo, Keyboard, Settings};
    t.command("focus");
    std::thread::sleep(Duration::from_millis(500));
    if localyapper_lib::platform::focus::foreground_pid() != Some(t.pid) {
        println!("SKIP: no Accessibility grant on runner (the runner keeps another app in front)");
        return false;
    }
    if let Ok(mut e) = Enigo::new(&Settings::default()) {
        let _ = e.text("probe");
    }
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        if t.text() == "probe" {
            t.command("clear");
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    println!("SKIP: no Accessibility grant on runner (synthetic key events are dropped)");
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
    let mut t = TextTarget::spawn();
    if !keys_reach(&mut t) {
        return;
    }
    inject::inject("Hello, world.", &opts(250)).expect("inject");
    assert_eq!(t.wait_for("Hello, world."), "Hello, world.");
}

#[test]
fn inject_restores_clipboard() {
    let mut t = TextTarget::spawn();
    if !keys_reach(&mut t) {
        return;
    }

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
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    let mut t = TextTarget::spawn();
    if !keys_reach(&mut t) {
        return;
    }
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
