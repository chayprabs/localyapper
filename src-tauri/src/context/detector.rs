// Context detection -- identifies the currently focused application
/// Map executable stems to human-friendly display names.
pub(crate) fn friendly_name(raw: &str) -> String {
    match raw.to_lowercase().as_str() {
        "notepad" => "Notepad".into(),
        "chrome" => "Chrome".into(),
        "code" => "VS Code".into(),
        "firefox" => "Firefox".into(),
        "msedge" => "Edge".into(),
        "winword" => "Word".into(),
        "excel" => "Excel".into(),
        "powerpnt" => "PowerPoint".into(),
        "onenote" => "OneNote".into(),
        "outlook" => "Outlook".into(),
        "slack" => "Slack".into(),
        "discord" => "Discord".into(),
        "telegram" => "Telegram".into(),
        "spotify" => "Spotify".into(),
        "windowsterminal" | "wt" => "Terminal".into(),
        "cmd" => "Command Prompt".into(),
        "powershell" | "pwsh" => "PowerShell".into(),
        "explorer" => "Explorer".into(),
        "teams" | "ms-teams" => "Teams".into(),
        "cursor" => "Cursor".into(),
        "notion" => "Notion".into(),
        "obsidian" => "Obsidian".into(),
        "figma" => "Figma".into(),
        "unknown" => "Unknown".into(),
        _ => title_case(raw),
    }
}

/// Title-case a string: first letter uppercase, rest as-is.
fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => {
            let mut result = c.to_uppercase().to_string();
            result.extend(chars);
            result
        }
        None => "Unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_name_mapping() {
        assert_eq!(friendly_name("notepad"), "Notepad");
        assert_eq!(friendly_name("chrome"), "Chrome");
        assert_eq!(friendly_name("Code"), "VS Code");
        assert_eq!(friendly_name("msedge"), "Edge");
        assert_eq!(friendly_name("WINWORD"), "Word");
        assert_eq!(friendly_name("wt"), "Terminal");
        assert_eq!(friendly_name("WindowsTerminal"), "Terminal");
    }

    #[test]
    fn friendly_name_unknown_gets_title_cased() {
        assert_eq!(friendly_name("myapp"), "Myapp");
        assert_eq!(friendly_name("Unknown"), "Unknown");
    }

    #[test]
    fn title_case_works() {
        assert_eq!(title_case("hello"), "Hello");
        assert_eq!(title_case(""), "Unknown");
        assert_eq!(title_case("A"), "A");
    }
}
