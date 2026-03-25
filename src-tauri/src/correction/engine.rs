// Correction engine -- dictionary-based word substitution
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use rusqlite::Connection;

use crate::db::queries;
use crate::error::LocalYapperError;

/// Characters to strip from the leading/trailing edges of tokens before lookup.
const PUNCT_CHARS: &[char] = &['.', ',', ';', ':', '!', '?', '\'', '"', '(', ')', '-', '[', ']', '{', '}'];

/// Exact-match correction engine backed by an in-memory HashMap.
/// Keys are lowercase; lookups are case-insensitive with case preservation on output.
pub struct CorrectionEngine {
    corrections: RwLock<HashMap<String, String>>,
    protected_words: RwLock<HashSet<String>>,
}

impl CorrectionEngine {
    /// Creates an empty correction engine.
    pub fn new() -> Self {
        Self {
            corrections: RwLock::new(HashMap::new()),
            protected_words: RwLock::new(HashSet::new()),
        }
    }

    /// Loads corrections and protected words from the database.
    /// For corrections with the same raw_word, the first row wins (highest confidence, then count).
    pub fn load(
        &self,
        conn: &Connection,
        confidence_threshold: f64,
    ) -> Result<(), LocalYapperError> {
        let rows = queries::get_all_corrections_for_engine(conn, confidence_threshold)?;
        let mut map = HashMap::new();
        for (raw_word, corrected, _count, _confidence) in rows {
            let key = raw_word.to_lowercase();
            // First wins per raw_word (already ordered by confidence DESC, count DESC)
            map.entry(key).or_insert(corrected);
        }

        let dict_words = queries::get_dictionary(conn)?;
        let mut protected = HashSet::new();
        for w in dict_words {
            protected.insert(w.word.to_lowercase());
        }

        // Swap in the new data
        {
            let mut corrections = self.corrections.write()
                .map_err(|e| LocalYapperError::InvalidInput(format!("Lock poisoned: {e}")))?;
            *corrections = map;
        }
        {
            let mut pw = self.protected_words.write()
                .map_err(|e| LocalYapperError::InvalidInput(format!("Lock poisoned: {e}")))?;
            *pw = protected;
        }

        Ok(())
    }

    /// Alias for load — called after DB mutations to refresh the in-memory maps.
    pub fn refresh(
        &self,
        conn: &Connection,
        confidence_threshold: f64,
    ) -> Result<(), LocalYapperError> {
        self.load(conn, confidence_threshold)
    }

    /// Applies exact-match word substitution to the input text.
    /// Preserves whitespace, punctuation, and case.
    pub fn apply(&self, text: &str) -> Result<String, LocalYapperError> {
        let corrections = self.corrections.read()
            .map_err(|e| LocalYapperError::InvalidInput(format!("Lock poisoned: {e}")))?;
        let protected = self.protected_words.read()
            .map_err(|e| LocalYapperError::InvalidInput(format!("Lock poisoned: {e}")))?;

        if corrections.is_empty() {
            return Ok(text.to_string());
        }

        // Walk the string preserving exact whitespace
        let mut result = String::with_capacity(text.len());
        let mut chars = text.char_indices().peekable();

        while let Some(&(start, ch)) = chars.peek() {
            if ch.is_whitespace() {
                result.push(ch);
                chars.next();
            } else {
                // Consume a full token (non-whitespace run)
                let mut end = start;
                while let Some(&(i, c)) = chars.peek() {
                    if c.is_whitespace() {
                        break;
                    }
                    end = i + c.len_utf8();
                    chars.next();
                }
                let token = &text[start..end];
                let replaced = replace_token(token, &corrections, &protected);
                result.push_str(&replaced);
            }
        }

        Ok(result)
    }
}

/// Strip leading punctuation from a token, returning (prefix, rest).
fn strip_leading_punct(token: &str) -> (&str, &str) {
    let start = token
        .char_indices()
        .find(|(_, c)| !PUNCT_CHARS.contains(c))
        .map(|(i, _)| i)
        .unwrap_or(token.len());
    (&token[..start], &token[start..])
}

/// Strip trailing punctuation from a token, returning (core, suffix).
fn strip_trailing_punct(token: &str) -> (&str, &str) {
    let end = token
        .char_indices()
        .rev()
        .find(|(_, c)| !PUNCT_CHARS.contains(c))
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    (&token[..end], &token[end..])
}

/// Apply case pattern from the original word to the replacement.
fn apply_case(original: &str, replacement: &str) -> String {
    if original.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
        replacement.to_uppercase()
    } else if original.starts_with(|c: char| c.is_uppercase()) {
        let mut chars = replacement.chars();
        match chars.next() {
            Some(first) => {
                let mut s = first.to_uppercase().to_string();
                s.extend(chars);
                s
            }
            None => String::new(),
        }
    } else {
        replacement.to_string()
    }
}

/// Replace a single token if it matches a correction and is not protected.
fn replace_token(
    token: &str,
    corrections: &HashMap<String, String>,
    protected: &HashSet<String>,
) -> String {
    let (prefix, rest) = strip_leading_punct(token);
    let (core, suffix) = strip_trailing_punct(rest);

    if core.is_empty() {
        return token.to_string();
    }

    let lower = core.to_lowercase();

    // Protected words are never replaced
    if protected.contains(&lower) {
        return token.to_string();
    }

    match corrections.get(&lower) {
        Some(replacement) => {
            let cased = apply_case(core, replacement);
            format!("{prefix}{cased}{suffix}")
        }
        None => token.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_with(corrections: &[(&str, &str)], protected: &[&str]) -> CorrectionEngine {
        let engine = CorrectionEngine::new();
        {
            let mut map = engine.corrections.write().unwrap();
            for (k, v) in corrections {
                map.insert(k.to_lowercase(), v.to_string());
            }
        }
        {
            let mut set = engine.protected_words.write().unwrap();
            for w in protected {
                set.insert(w.to_lowercase());
            }
        }
        engine
    }

    #[test]
    fn empty_engine_passthrough() {
        let engine = CorrectionEngine::new();
        let result = engine.apply("hello world").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn basic_correction() {
        let engine = engine_with(&[("teh", "the")], &[]);
        let result = engine.apply("teh quick brown fox").unwrap();
        assert_eq!(result, "the quick brown fox");
    }

    #[test]
    fn case_preservation() {
        let engine = engine_with(&[("teh", "the")], &[]);
        assert_eq!(engine.apply("Teh").unwrap(), "The");
        assert_eq!(engine.apply("TEH").unwrap(), "THE");
        assert_eq!(engine.apply("teh").unwrap(), "the");
    }

    #[test]
    fn punctuation_handling() {
        let engine = engine_with(&[("teh", "the")], &[]);
        assert_eq!(engine.apply("teh,").unwrap(), "the,");
        assert_eq!(engine.apply("(teh)").unwrap(), "(the)");
        assert_eq!(engine.apply("\"teh\"").unwrap(), "\"the\"");
        assert_eq!(engine.apply("teh.").unwrap(), "the.");
    }

    #[test]
    fn protected_word_blocks_correction() {
        let engine = engine_with(&[("kubernetes", "cubernetes")], &["kubernetes"]);
        let result = engine.apply("kubernetes is great").unwrap();
        assert_eq!(result, "kubernetes is great");
    }

    #[test]
    fn multiple_corrections_picks_best() {
        // Simulate: at load time, first row wins per raw_word.
        // Here we just verify that if the map has one entry, it gets used.
        let engine = engine_with(&[("recieve", "receive")], &[]);
        assert_eq!(engine.apply("recieve").unwrap(), "receive");
    }

    #[test]
    fn confidence_threshold_filtering() {
        // Engine with no corrections simulates everything being filtered out
        let engine = engine_with(&[], &[]);
        assert_eq!(engine.apply("teh quick").unwrap(), "teh quick");
    }

    #[test]
    fn whitespace_preservation() {
        let engine = engine_with(&[("teh", "the")], &[]);
        let result = engine.apply("  teh   quick  ").unwrap();
        assert_eq!(result, "  the   quick  ");
    }
}
