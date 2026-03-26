// Platform detection -- OS and display server identification
/// Detected operating system and display server.
/// Used by the injector to select the correct clipboard and key simulation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Windows: uses enigo for Ctrl+V paste simulation.
    Windows,
    /// macOS: uses enigo for Cmd+V paste simulation.
    MacOS,
    /// Linux X11: uses xclip + xdotool for clipboard and key simulation.
    LinuxX11,
    /// Linux Wayland: uses wl-clipboard + wtype for clipboard and key simulation.
    LinuxWayland,
}

/// Detect the current platform at runtime.
pub fn detect() -> Platform {
    if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOS
    } else {
        // Linux: check WAYLAND_DISPLAY env var
        match std::env::var("WAYLAND_DISPLAY") {
            Ok(val) if !val.is_empty() => Platform::LinuxWayland,
            _ => Platform::LinuxX11,
        }
    }
}
