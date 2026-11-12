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

    /// Poll until the field's words match `want` closely or 20 s pass.
    fn wait_for_words(&mut self, want: &str) -> String {
        let start = Instant::now();
        loop {
            let got = self.text();
            if (!got.trim().is_empty() && wer::wer(want, &got) < 0.15) || start.elapsed() > WAIT {
                return got;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
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
/// Bring the target to the front. On macOS the runner can keep another app
/// in front no matter what; then the test is skipped with the reason.
fn in_front(t: &mut TextTarget) -> bool {
    if !cfg!(target_os = "macos") {
        t.focus();
        return true;
    }
    t.command("focus");
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(3) {
        if localyapper_lib::platform::focus::foreground_pid() == Some(t.pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    println!("SKIP: the macOS runner keeps another app in front");
    false
}

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

// ---------------------------------------------------------------------
// The app under test: built with test-hooks, driven over the control
// channel (LOCALYAPPER_TEST_PORT) with fake audio from a fixture.
// ---------------------------------------------------------------------

use localyapper_lib::core::wer;
use serde_json::Value;
use std::io::Read;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn fixture(name: &str) -> (PathBuf, String) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let text = std::fs::read_to_string(dir.join(format!("{name}.txt"))).expect("fixture text");
    (dir.join(format!("{name}.wav")), text.trim().to_string())
}

/// One data folder for the whole run, with the speech model linked in.
fn data_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("localyapper-smoke-{}", std::process::id()));
        let model_src = std::env::var_os("LOCALYAPPER_TEST_MODEL_DIR")
            .map(PathBuf::from)
            .expect("set LOCALYAPPER_TEST_MODEL_DIR to the parakeet-110m model folder");
        let manifest = localyapper_lib::core::stt::catalog::manifest().expect("manifest");
        let entry = &manifest.models[localyapper_lib::core::stt::catalog::DEFAULT_MODEL];
        let model_dst = dir
            .join("models")
            .join(localyapper_lib::core::stt::catalog::DEFAULT_MODEL);
        std::fs::create_dir_all(&model_dst).expect("model dir");
        for entry in std::fs::read_dir(&model_src)
            .expect("read model dir")
            .flatten()
        {
            let to = model_dst.join(entry.file_name());
            if std::fs::hard_link(entry.path(), &to).is_err() {
                std::fs::copy(entry.path(), &to).expect("copy model file");
            }
        }
        // The files came from scripts/fetch-models.py, which checked every
        // SHA-256, so the install marker can be written directly.
        std::fs::write(
            model_dst.join(localyapper_lib::core::stt::catalog::VERIFIED_MARKER),
            localyapper_lib::core::stt::catalog::marker_text(entry),
        )
        .expect("marker");
        dir
    })
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .expect("free port")
}

struct App {
    child: Child,
    conn: BufReader<TcpStream>,
    log: PathBuf,
}

impl App {
    fn launch(audio: &Path) -> Self {
        Self::launch_with(audio, "")
    }

    fn launch_with(audio: &Path, extra_settings: &str) -> Self {
        let port = free_port();
        let log = data_dir().join(format!("app-{port}.log"));
        let log_file = std::fs::File::create(&log).expect("log file");
        let settings = format!(
            "setup_complete=true;setup_step=done;auto_start=false;idle_unload_seconds=0;\
             speech_model=parakeet-110m;\
             overlay_preview=true;activation_mode=hold;{extra_settings}"
        );
        let child = Command::new(env!("CARGO_BIN_EXE_localyapper"))
            .env("LOCALYAPPER_DATA_DIR", data_dir())
            .env("LOCALYAPPER_TEST_PORT", port.to_string())
            .env("LOCALYAPPER_FAKE_AUDIO", audio)
            .env("LOCALYAPPER_TEST_SETTINGS", settings)
            .env("RUST_LOG", "info")
            .stdout(Stdio::null())
            .stderr(log_file)
            .spawn()
            .expect("start app");
        let start = Instant::now();
        let stream = loop {
            match TcpStream::connect(("127.0.0.1", port)) {
                Ok(s) => break s,
                Err(_) if start.elapsed() < WAIT => std::thread::sleep(Duration::from_millis(100)),
                Err(e) => panic!("app control channel did not open: {e}"),
            }
        };
        stream.set_read_timeout(Some(WAIT)).expect("read timeout");
        let mut app = Self {
            child,
            conn: BufReader::new(stream),
            log,
        };
        // The controller answers once setup has finished.
        let st = app.state();
        assert_eq!(st["ok"], Value::Bool(true), "controller did not answer");
        app
    }

    fn cmd(&mut self, c: &str) -> Value {
        let s = self.conn.get_mut();
        writeln!(s, "{c}").expect("send");
        let mut line = String::new();
        if let Err(e) = self.conn.read_line(&mut line) {
            panic!(
                "app did not answer {c:?}: {e}\napp log:\n{}",
                self.log_text()
            );
        }
        serde_json::from_str(&line).expect("json")
    }

    fn state(&mut self) -> Value {
        self.cmd("state")
    }

    fn dictations(&mut self) -> u64 {
        self.state()["state"]["dictations"].as_u64().unwrap_or(0)
    }

    fn idle(&mut self) -> bool {
        self.state()["state"]["idle"].as_bool().unwrap_or(false)
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Whether this app process may send key events (macOS grant).
    fn can_type(&mut self) -> bool {
        let ok = self.cmd("ax")["trusted"].as_bool().unwrap_or(false);
        if !ok {
            println!(
                "SKIP: no Accessibility grant on runner (synthetic key events are not allowed)"
            );
        }
        ok
    }

    fn log_text(&self) -> String {
        let mut s = String::new();
        if let Ok(mut f) = std::fs::File::open(&self.log) {
            let _ = f.read_to_string(&mut s);
        }
        s
    }

    /// Hold the record key for `ms`, then release.
    fn dictate(&mut self, ms: u64) {
        self.cmd("down");
        std::thread::sleep(Duration::from_millis(ms));
        self.cmd("up");
    }

    fn wait_dictations(&mut self, n: u64) {
        let start = Instant::now();
        while self.dictations() < n {
            if start.elapsed() > WAIT {
                panic!(
                    "dictation {n} did not finish; app log:\n{}",
                    self.log_text()
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let log = self.log_text();
            let tail: Vec<&str> = log.lines().rev().take(60).collect();
            std::thread::sleep(Duration::from_millis(200));
            eprintln!("---- app exit status: {:?} ----", self.child.try_wait());
            eprintln!("---- app log (last 60 lines) ----");
            for line in tail.iter().rev() {
                eprintln!("{line}");
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn clip_ms(path: &Path) -> u64 {
    localyapper_lib::core::audio::wav::read_file(path)
        .expect("wav")
        .duration_ms()
}

fn fg_pid() -> Option<u32> {
    localyapper_lib::platform::focus::foreground_pid()
}

#[test]
fn hotkey_registers() {
    let (wav, _) = fixture("clip01_16k");
    let mut app = App::launch(&wav);
    let r = app.cmd("hotkeys");
    assert_eq!(r["ok"], Value::Bool(true), "hotkey errors: {r}");
}

#[test]
fn overlay_receives_levels() {
    let (wav, _) = fixture("clip01_16k");
    let mut app = App::launch(&wav);
    app.cmd("reset_seen");
    app.cmd("down");
    std::thread::sleep(Duration::from_millis(2500));
    let st = app.state();
    app.cmd("cancel");
    let levels = st["overlay"]["distinct_levels"].as_u64().unwrap_or(0);
    assert!(levels >= 10, "overlay saw {levels} distinct levels: {st}");
    let kinds = st["overlay"]["page_kinds"].to_string();
    assert!(
        kinds.contains("listening"),
        "page never showed Listening: {kinds}"
    );
}

#[test]
fn overlay_does_not_take_focus() {
    let (wav, _) = fixture("clip01_16k");
    let mut t = TextTarget::spawn();
    let mut app = App::launch(&wav);
    if !in_front(&mut t) {
        return;
    }
    let target = t.pid;
    assert_eq!(
        fg_pid(),
        Some(target),
        "text target is not in front to begin with"
    );
    app.cmd("down");
    std::thread::sleep(Duration::from_millis(300));
    let shown = app.state()["state"]["overlay_rect"].clone();
    let during = fg_pid();
    app.cmd("cancel");
    std::thread::sleep(Duration::from_millis(300));
    let after = fg_pid();
    assert!(!shown.is_null(), "overlay was not shown");
    assert_ne!(during, Some(app.pid()), "overlay took focus");
    assert_eq!(
        during,
        Some(target),
        "focus moved while the overlay was shown"
    );
    assert_eq!(after, Some(target), "focus moved after the overlay hid");
}

#[test]
fn overlay_is_click_through() {
    use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};
    let (wav, _) = fixture("clip01_16k");
    let mut t = TextTarget::spawn();
    t.command("cover");
    if !keys_reach(&mut t) {
        return;
    }
    let mut app = App::launch(&wav);
    app.cmd("down");
    std::thread::sleep(Duration::from_millis(500));
    let rect = app.state()["state"]["overlay_rect"].clone();
    assert!(!rect.is_null(), "overlay was not shown");
    let scale = rect["scale"].as_f64().unwrap_or(1.0);
    let px = rect["x"].as_f64().unwrap_or(0.0) + rect["width"].as_f64().unwrap_or(0.0) / 2.0;
    let py = rect["y"].as_f64().unwrap_or(0.0) + rect["height"].as_f64().unwrap_or(0.0) / 2.0;
    // enigo takes points on macOS and pixels elsewhere.
    let (cx, cy) = if cfg!(target_os = "macos") {
        (px / scale, py / scale)
    } else {
        (px, py)
    };
    let before: usize = t.command("clicks").parse().unwrap_or(0);
    let mut enigo = Enigo::new(&Settings::default()).expect("enigo");
    enigo
        .move_mouse(cx.round() as i32, cy.round() as i32, Coordinate::Abs)
        .expect("move");
    enigo.button(Button::Left, Direction::Click).expect("click");
    std::thread::sleep(Duration::from_millis(300));
    let after: usize = t.command("clicks").parse().unwrap_or(0);
    app.cmd("cancel");
    assert!(
        after > before,
        "the click at the pill's centre did not reach the window below"
    );
}

#[test]
fn cancel_discards() {
    let (wav, _) = fixture("clip01_16k");
    let mut t = TextTarget::spawn();
    let mut app = App::launch(&wav);
    app.cmd("down");
    std::thread::sleep(Duration::from_millis(1500));
    app.cmd("cancel");
    std::thread::sleep(Duration::from_millis(2000));
    assert!(app.idle(), "not idle after cancel");
    assert_eq!(app.dictations(), 0);
    assert_eq!(t.text(), "");
}

fn assert_words(got: &str, want: &str) {
    let w = wer::wer(want, got);
    assert!(
        w < 0.15,
        "WER {:.1} %: got {got:?}, want {want:?}",
        100.0 * w
    );
}

fn timings(app: &mut App) -> Value {
    app.state()["state"]["last_timings"].clone()
}

#[test]
fn end_to_end_dictation() {
    let (wav, want) = fixture("clip01_16k");
    let mut t = TextTarget::spawn();
    let mut app = App::launch(&wav);
    if !app.can_type() || !keys_reach(&mut t) {
        return;
    }
    t.focus();
    let target = t.pid;
    app.dictate(clip_ms(&wav) + 300);
    let during = fg_pid();
    app.wait_dictations(1);
    let got = t.wait_for_words(&want);
    assert_words(&got, &want);
    assert_eq!(during, Some(target), "focus moved during dictation");
    assert_eq!(fg_pid(), Some(target), "focus moved after dictation");
    let tm = timings(&mut app);
    println!("PERF end_to_end_dictation {tm}");
    let r2t = tm["release_to_text_ms"].as_u64().unwrap_or(u64::MAX);
    assert!(r2t < 5000, "release_to_text_ms {r2t}");
}

#[test]
fn end_to_end_cold_and_warm() {
    let (wav, want) = fixture("clip02_16k");
    let mut t = TextTarget::spawn();
    let mut app = App::launch(&wav);
    if !app.can_type() || !keys_reach(&mut t) {
        return;
    }
    t.focus();
    app.dictate(clip_ms(&wav) + 300);
    app.wait_dictations(1);
    assert_words(&t.wait_for_words(&want), &want);
    let cold = timings(&mut app);
    std::thread::sleep(Duration::from_secs(2));
    t.command("clear");
    t.focus();
    app.dictate(clip_ms(&wav) + 300);
    app.wait_dictations(2);
    assert_words(&t.wait_for_words(&want), &want);
    let warm = timings(&mut app);
    println!("PERF cold {cold}");
    println!("PERF warm {warm}");
    for tm in [&cold, &warm] {
        let r2t = tm["release_to_text_ms"].as_u64().unwrap_or(u64::MAX);
        assert!(r2t < 5000, "release_to_text_ms {r2t}");
    }
}

#[test]
fn back_to_back() {
    let (wav, want) = fixture("clip01_16k");
    let mut t = TextTarget::spawn();
    let mut app = App::launch(&wav);
    if !app.can_type() || !keys_reach(&mut t) {
        return;
    }
    t.focus();
    app.dictate(clip_ms(&wav) + 300);
    app.wait_dictations(1);
    // No pause: the next press comes as soon as the first paste lands.
    app.dictate(clip_ms(&wav) + 300);
    app.wait_dictations(2);
    let both = format!("{want} {want}");
    let got = t.wait_for_words(&both);
    assert_words(&got, &both);
}

#[test]
fn second_instance_exits() {
    let (wav, _) = fixture("clip01_16k");
    let mut first = App::launch(&wav);
    let mut second = Command::new(env!("CARGO_BIN_EXE_localyapper"))
        .env("LOCALYAPPER_DATA_DIR", data_dir())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("second instance");
    let start = Instant::now();
    let exited = loop {
        if let Ok(Some(status)) = second.try_wait() {
            break Some(status);
        }
        if start.elapsed() > Duration::from_secs(3) {
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    if exited.is_none() {
        let _ = second.kill();
    }
    assert!(exited.is_some(), "second instance kept running");
    assert!(
        first.child.try_wait().ok().flatten().is_none(),
        "first instance died"
    );
    assert_eq!(first.state()["ok"], Value::Bool(true));
}

#[cfg(target_os = "linux")]
#[test]
fn x11_repeat_hold() {
    let (wav, _) = fixture("clip01_16k");
    let mut app = App::launch_with(&wav, "hotkey_record=F8");
    let before = app.state()["state"].clone();
    let (d0, u0) = (
        before["record_downs"].as_u64().unwrap_or(0),
        before["record_ups"].as_u64().unwrap_or(0),
    );
    let run = |args: &[&str]| {
        Command::new("xdotool")
            .args(args)
            .status()
            .expect("xdotool")
    };
    assert!(run(&["keydown", "F8"]).success());
    std::thread::sleep(Duration::from_secs(2));
    assert!(run(&["keyup", "F8"]).success());
    std::thread::sleep(Duration::from_millis(300));
    let after = app.state()["state"].clone();
    app.cmd("cancel");
    assert_eq!(
        after["record_downs"].as_u64().unwrap_or(0) - d0,
        1,
        "{after}"
    );
    assert_eq!(after["record_ups"].as_u64().unwrap_or(0) - u0, 1, "{after}");
}
