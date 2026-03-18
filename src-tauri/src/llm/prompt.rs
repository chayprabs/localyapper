/// Build a ChatML prompt from a mode's system prompt, user text, and optional app context.
///
/// Uses `<|im_start|>` / `<|im_end|>` delimiters expected by Qwen 2.5.
/// If `app_name` is empty or `"Unknown"`, the app context line is omitted.
pub fn build_prompt(system_prompt: &str, user_text: &str, app_name: &str) -> String {
    let app_context = if !app_name.is_empty() && app_name != "Unknown" {
        format!("\nThe user is currently typing in: {app_name}")
    } else {
        String::new()
    };

    format!(
        "<|im_start|>system\n{system_prompt}{app_context}<|im_end|>\n\
         <|im_start|>user\n{user_text}<|im_end|>\n\
         <|im_start|>assistant\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_prompt_format() {
        let prompt = build_prompt("You are a helpful assistant.", "Hello world", "VS Code");
        assert!(prompt.starts_with("<|im_start|>system\n"));
        assert!(prompt.contains("You are a helpful assistant."));
        assert!(prompt.contains("The user is currently typing in: VS Code"));
        assert!(prompt.contains("<|im_start|>user\nHello world<|im_end|>"));
        assert!(prompt.ends_with("<|im_start|>assistant\n"));
    }

    #[test]
    fn unknown_app_omits_context() {
        let prompt = build_prompt("System prompt.", "Test input", "Unknown");
        assert!(!prompt.contains("typing in"));
    }

    #[test]
    fn empty_app_omits_context() {
        let prompt = build_prompt("System prompt.", "Test input", "");
        assert!(!prompt.contains("typing in"));
    }

    #[test]
    fn prompt_preserves_user_text_exactly() {
        let prompt = build_prompt("sys", "Hello, world! This is a test.", "App");
        assert!(prompt.contains("<|im_start|>user\nHello, world! This is a test.<|im_end|>"));
    }
}
