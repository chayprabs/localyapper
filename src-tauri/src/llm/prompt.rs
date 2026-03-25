// Prompt construction -- system prompt builder for mode-aware cleanup
/// Build the system prompt content by combining the mode's system prompt with app context.
///
/// If `app_name` is empty or `"Unknown"`, the app context line is omitted.
/// This is used with mistral.rs TextMessages which handles chat templating internally.
pub fn build_system_prompt(system_prompt: &str, app_name: &str) -> String {
    if !app_name.is_empty() && app_name != "Unknown" {
        format!("{system_prompt}\nThe user is currently typing in: {app_name}")
    } else {
        system_prompt.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_app_context() {
        let prompt = build_system_prompt("You are a helpful assistant.", "VS Code");
        assert!(prompt.contains("You are a helpful assistant."));
        assert!(prompt.contains("The user is currently typing in: VS Code"));
    }

    #[test]
    fn unknown_app_omits_context() {
        let prompt = build_system_prompt("System prompt.", "Unknown");
        assert!(!prompt.contains("typing in"));
        assert_eq!(prompt, "System prompt.");
    }

    #[test]
    fn empty_app_omits_context() {
        let prompt = build_system_prompt("System prompt.", "");
        assert!(!prompt.contains("typing in"));
        assert_eq!(prompt, "System prompt.");
    }
}
