//! Text injection (plan Appendix E). All in-process.
//!
//! Default flow: wait for the hotkey's modifiers to be released, save the
//! clipboard (text, or an image), set the dictated text excluded from
//! clipboard history, send the paste keys, return to the controller, and
//! restore the saved clipboard on a detached thread after
//! `restore_delay_ms`, only if the clipboard still holds the dictated
//! text. `type` and `clipboard_only` are alternatives chosen in settings.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use linux as os;
#[cfg(target_os = "macos")]
use macos as os;
#[cfg(windows)]
use windows as os;

use std::sync::Mutex;
use std::time::Duration;

/// Whether this is a Wayland session (Linux only).
pub fn is_wayland_session() -> bool {
    #[cfg(target_os = "linux")]
    {
        linux::is_wayland()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// How the text reaches the target app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasteMethod {
    /// Cmd+V on macOS; Ctrl+Shift+V for known terminals, Ctrl+V otherwise.
    Auto,
    CtrlV,
    CtrlShiftV,
    ShiftInsert,
    /// Type the text as key events, without the clipboard.
    Type,
    /// Leave the text on the clipboard for the user to paste.
    ClipboardOnly,
}

impl PasteMethod {
    pub const ALL: [&'static str; 6] = [
        "auto",
        "ctrl_v",
        "ctrl_shift_v",
        "shift_insert",
        "type",
        "clipboard_only",
    ];

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "auto" => Self::Auto,
            "ctrl_v" => Self::CtrlV,
            "ctrl_shift_v" => Self::CtrlShiftV,
            "shift_insert" => Self::ShiftInsert,
            "type" => Self::Type,
            "clipboard_only" => Self::ClipboardOnly,
            _ => return None,
        })
    }
}

/// The key chord a paste method resolves to on this OS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chord {
    /// Cmd+V on macOS, Ctrl+V elsewhere.
    PrimaryV,
    CtrlShiftV,
    ShiftInsert,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InjectOptions {
    pub method: PasteMethod,
    pub restore_clipboard: bool,
    pub restore_delay_ms: u64,
    /// Process name of the target app (for `auto` terminal detection).
    pub target_process: String,
}

impl Default for InjectOptions {
    fn default() -> Self {
        Self {
            method: PasteMethod::Auto,
            restore_clipboard: true,
            restore_delay_ms: 250,
            target_process: String::new(),
        }
    }
}

/// What happened, for the overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Pasted,
    Typed,
    /// The text is on the clipboard but was not pasted.
    CopiedOnly {
        reason: Option<String>,
    },
}

const TERMINALS: &[&str] = &[
    "windowsterminal",
    "wt",
    "cmd",
    "conhost",
    "powershell",
    "pwsh",
    "mintty",
    "alacritty",
    "wezterm",
    "wezterm-gui",
    "kitty",
    "gnome-terminal",
    "gnome-terminal-server",
    "konsole",
    "xterm",
    "uxterm",
    "urxvt",
    "rxvt",
    "tilix",
    "terminator",
    "xfce4-terminal",
    "mate-terminal",
    "lxterminal",
    "qterminal",
    "terminology",
    "foot",
    "st",
    "guake",
    "yakuake",
    "ptyxis",
    "kgx",
];

pub fn is_terminal(process: &str) -> bool {
    let p = process.to_lowercase();
    let p = p.trim_end_matches(".exe");
    TERMINALS.contains(&p)
}

/// Resolve the chord for a clipboard paste method. `None` means the
/// method does not paste through the clipboard.
pub fn chord_for(method: PasteMethod, target_process: &str) -> Option<Chord> {
    match method {
        PasteMethod::Auto => {
            if cfg!(target_os = "macos") || !is_terminal(target_process) {
                Some(Chord::PrimaryV)
            } else {
                Some(Chord::CtrlShiftV)
            }
        }
        PasteMethod::CtrlV => Some(Chord::PrimaryV),
        PasteMethod::CtrlShiftV => Some(Chord::CtrlShiftV),
        PasteMethod::ShiftInsert => Some(Chord::ShiftInsert),
        PasteMethod::Type | PasteMethod::ClipboardOnly => None,
    }
}

/// Why the paste keys could not be sent.
#[derive(Debug)]
pub enum PasteError {
    /// This session cannot send key events at all; fall back to
    /// leaving the text on the clipboard.
    Unavailable(String),
    Failed(String),
}

/// Saved clipboard content.
enum Saved {
    Text(String),
    Image(arboard::ImageData<'static>),
    Empty,
}

/// One long-lived clipboard handle. On Linux the owning process serves
/// the selection, so the handle must outlive the paste.
static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

fn with_clipboard<T>(
    f: impl FnOnce(&mut arboard::Clipboard) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = CLIPBOARD
        .lock()
        .map_err(|_| "clipboard lock poisoned".to_string())?;
    if guard.is_none() {
        *guard =
            Some(arboard::Clipboard::new().map_err(|e| format!("Clipboard unavailable: {e}"))?);
    }
    match guard.as_mut() {
        Some(cb) => f(cb),
        None => Err("Clipboard unavailable".into()),
    }
}

fn set_text_excluded(cb: &mut arboard::Clipboard, text: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    use arboard::SetExtApple;
    #[cfg(target_os = "linux")]
    use arboard::SetExtLinux;
    #[cfg(windows)]
    use arboard::SetExtWindows;
    cb.set()
        .exclude_from_history()
        .text(text.to_owned())
        .map_err(|e| format!("Could not set the clipboard: {e}"))
}

/// Read the clipboard as text, for tests and the restore check.
pub fn clipboard_text() -> Option<String> {
    with_clipboard(|cb| cb.get_text().map_err(|e| e.to_string())).ok()
}

/// Put text on the clipboard (tests, the overlay's copy button).
pub fn set_clipboard_text(text: &str) -> Result<(), String> {
    with_clipboard(|cb| cb.set_text(text.to_owned()).map_err(|e| e.to_string()))
}

/// Put an image on the clipboard (tests).
pub fn set_clipboard_image(image: arboard::ImageData<'static>) -> Result<(), String> {
    with_clipboard(|cb| cb.set_image(image).map_err(|e| e.to_string()))
}

/// Read the clipboard as an image (tests).
pub fn clipboard_image() -> Option<arboard::ImageData<'static>> {
    with_clipboard(|cb| cb.get_image().map_err(|e| e.to_string())).ok()
}

fn save_clipboard() -> Option<Saved> {
    with_clipboard(|cb| {
        if let Ok(t) = cb.get_text() {
            return Ok(Some(Saved::Text(t)));
        }
        Ok(Some(cb.get_image().map_or(Saved::Empty, Saved::Image)))
    })
    .ok()
    .flatten()
}

fn restore_later(saved: Saved, dictated: String, delay_ms: u64) {
    let spawned = std::thread::Builder::new()
        .name("clipboard-restore".into())
        .spawn(move || {
            std::thread::sleep(Duration::from_millis(delay_ms));
            let result = with_clipboard(|cb| {
                // The user may have copied something else meanwhile.
                if cb.get_text().ok().as_deref() != Some(dictated.as_str()) {
                    return Ok(());
                }
                match saved {
                    Saved::Text(t) => cb.set_text(t),
                    Saved::Image(i) => cb.set_image(i),
                    Saved::Empty => cb.clear(),
                }
                .map_err(|e| e.to_string())
            });
            if let Err(e) = result {
                log::warn!("inject: clipboard restore failed: {e}");
            }
        });
    if let Err(e) = spawned {
        log::warn!("inject: could not start clipboard restore: {e}");
    }
}

/// Wait until Ctrl, Alt, Shift and the OS key are all up (at most
/// 400 ms), so the paste is not combined with the hotkey's modifiers.
pub fn wait_for_modifiers() {
    os::wait_modifiers_up(Duration::from_millis(400));
}

/// Inject `text` into the focused app. Blocking; run off the async
/// runtime. Returns once the keys are sent; the clipboard is restored
/// later on its own thread.
pub fn inject(text: &str, opts: &InjectOptions) -> Result<Outcome, String> {
    if text.is_empty() {
        return Ok(Outcome::Pasted);
    }
    wait_for_modifiers();

    if opts.method == PasteMethod::Type {
        os::type_text(text)?;
        return Ok(Outcome::Typed);
    }

    let saved = if opts.restore_clipboard && opts.method != PasteMethod::ClipboardOnly {
        save_clipboard()
    } else {
        None
    };
    with_clipboard(|cb| set_text_excluded(cb, text))?;

    let Some(chord) = chord_for(opts.method, &opts.target_process) else {
        return Ok(Outcome::CopiedOnly { reason: None });
    };
    std::thread::sleep(Duration::from_millis(20));
    match os::send_chord(chord) {
        Ok(()) => {}
        Err(PasteError::Unavailable(reason)) => {
            // No way to send keys (for example Wayland without a virtual
            // keyboard): leave the text on the clipboard and say so.
            return Ok(Outcome::CopiedOnly {
                reason: Some(reason),
            });
        }
        Err(PasteError::Failed(e)) => {
            if let Some(s) = saved {
                restore_later(s, text.to_string(), 0);
            }
            return Err(e);
        }
    }
    if let Some(s) = saved {
        restore_later(s, text.to_string(), opts.restore_delay_ms);
    }
    Ok(Outcome::Pasted)
}

/// Presses keys and releases them in reverse order on drop, so an error
/// between press and release can never leave a modifier stuck down.
pub(crate) struct Held<'a> {
    enigo: &'a mut enigo::Enigo,
    keys: Vec<enigo::Key>,
}

impl<'a> Held<'a> {
    pub(crate) fn new(enigo: &'a mut enigo::Enigo) -> Self {
        Self {
            enigo,
            keys: Vec::new(),
        }
    }

    pub(crate) fn press(&mut self, key: enigo::Key) -> Result<(), String> {
        use enigo::Keyboard;
        self.enigo
            .key(key, enigo::Direction::Press)
            .map_err(|e| format!("key press failed: {e}"))?;
        self.keys.push(key);
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) fn click(&mut self, key: enigo::Key) -> Result<(), String> {
        use enigo::Keyboard;
        self.enigo
            .key(key, enigo::Direction::Click)
            .map_err(|e| format!("key click failed: {e}"))
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn click_raw(&mut self, code: u16) -> Result<(), String> {
        use enigo::Keyboard;
        self.enigo
            .raw(code, enigo::Direction::Click)
            .map_err(|e| format!("key click failed: {e}"))
    }
}

impl Drop for Held<'_> {
    fn drop(&mut self) {
        use enigo::Keyboard;
        while let Some(k) = self.keys.pop() {
            let _ = self.enigo.key(k, enigo::Direction::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paste_methods_parse() {
        for m in PasteMethod::ALL {
            assert!(PasteMethod::parse(m).is_some(), "{m}");
        }
        assert_eq!(PasteMethod::parse("ctrl+v"), None);
    }

    #[test]
    fn auto_uses_ctrl_shift_v_only_for_terminals_off_macos() {
        let term = chord_for(PasteMethod::Auto, "gnome-terminal-server");
        let editor = chord_for(PasteMethod::Auto, "gedit");
        assert_eq!(editor, Some(Chord::PrimaryV));
        if cfg!(target_os = "macos") {
            assert_eq!(term, Some(Chord::PrimaryV));
        } else {
            assert_eq!(term, Some(Chord::CtrlShiftV));
        }
        assert!(is_terminal("WindowsTerminal.exe"));
        assert!(!is_terminal("notepad"));
        assert_eq!(chord_for(PasteMethod::Type, "x"), None);
        assert_eq!(chord_for(PasteMethod::ClipboardOnly, "x"), None);
        assert_eq!(
            chord_for(PasteMethod::ShiftInsert, "x"),
            Some(Chord::ShiftInsert)
        );
    }
}
