// Platform detection -- OS and display server identification
/// Detected operating system and display server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOS,
    LinuxX11,
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
