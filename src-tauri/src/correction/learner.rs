// Correction learner -- diff computation and confidence scoring
use rusqlite::Connection;

use crate::correction::engine::CorrectionEngine;
use crate::db::queries;
use crate::error::LocalYapperError;

/// Characters to strip from token edges before comparison.
const PUNCT_CHARS: &[char] = &[
    '.', ',', ';', ':', '!', '?', '\'', '"', '(', ')', '-', '[', ']', '{', '}',
];

/// A single word-level diff learned from the pipeline.
#[derive(Debug, Clone)]
pub struct LearnedCorrection {
    pub raw_word: String,
    pub corrected_word: String,
}

/// Compute word-level diffs between raw (Whisper) text and final (post-LLM) text.
///
/// Uses simple positional alignment: tokenize both by whitespace, zip-walk,
/// collect pairs where raw != final (case-insensitive to avoid learning pure-case diffs).
pub fn compute_diffs(raw_text: &str, final_text: &str) -> Vec<LearnedCorrection> {
    let raw_words: Vec<&str> = raw_text.split_whitespace().collect();
    let final_words: Vec<&str> = final_text.split_whitespace().collect();

    let mut diffs = Vec::new();

    for (raw_token, final_token) in raw_words.iter().zip(final_words.iter()) {
        let raw_core = strip_punct(raw_token);
        let final_core = strip_punct(final_token);

        // Skip empty, single-char, or identical cores
        if raw_core.is_empty() || final_core.is_empty() {
            continue;
        }
        if raw_core.len() <= 1 || final_core.len() <= 1 {
            continue;
        }

        let raw_lower = raw_core.to_lowercase();
        let final_lower = final_core.to_lowercase();

        if raw_lower == final_lower {
            continue;
        }

        diffs.push(LearnedCorrection {
            raw_word: raw_lower,
            corrected_word: final_lower,
        });
    }

    diffs
}

/// Write learned corrections to the database and update confidence scores.
///
/// For each diff: upsert correction → recompute confidence = min(1.0, count * 0.1).
/// After all writes, refresh the CorrectionEngine in-memory maps.
/// Returns the number of corrections written.
pub fn learn_and_refresh(
    conn: &Connection,
    diffs: &[LearnedCorrection],
    correction_engine: &CorrectionEngine,
) -> Result<usize, LocalYapperError> {
    let mut count = 0usize;

    for diff in diffs {
        let id = uuid::Uuid::new_v4().to_string();
        let correction = queries::insert_correction(conn, &id, &diff.raw_word, &diff.corrected_word)?;

        let new_confidence = (correction.count as f64 * 0.1).min(1.0);
        queries::update_correction_confidence(conn, &correction.id, new_confidence)?;

        count += 1;
    }

    if count > 0 {
        let threshold: f64 = queries::get_setting(conn, "confidence_threshold")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.6);
        correction_engine.refresh(conn, threshold)?;
    }

    Ok(count)
}

/// Strip leading and trailing punctuation from a token, returning the core.
fn strip_punct(token: &str) -> &str {
    let start = token
        .char_indices()
        .find(|(_, c)| !PUNCT_CHARS.contains(c))
        .map(|(i, _)| i)
        .unwrap_or(token.len());

    let trimmed = &token[start..];

    let end = trimmed
        .char_indices()
        .rev()
        .find(|(_, c)| !PUNCT_CHARS.contains(c))
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);

    &trimmed[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_text_no_diffs() {
        let diffs = compute_diffs("hello world", "hello world");
        assert!(diffs.is_empty());
    }

    #[test]
    fn single_word_change() {
        let diffs = compute_diffs("teh quick", "the quick");
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].raw_word, "teh");
        assert_eq!(diffs[0].corrected_word, "the");
    }

    #[test]
    fn multiple_changes() {
        let diffs = compute_diffs("teh quik brown", "the quick brown");
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].raw_word, "teh");
        assert_eq!(diffs[0].corrected_word, "the");
        assert_eq!(diffs[1].raw_word, "quik");
        assert_eq!(diffs[1].corrected_word, "quick");
    }

    #[test]
    fn different_length_texts() {
        // Only processes overlapping portion
        let diffs = compute_diffs("teh quick brown fox", "the quick");
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].raw_word, "teh");
    }

    #[test]
    fn punctuation_stripping() {
        let diffs = compute_diffs("teh, world!", "the, world!");
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].raw_word, "teh");
        assert_eq!(diffs[0].corrected_word, "the");
    }

    #[test]
    fn case_insensitive_no_diff() {
        let diffs = compute_diffs("Hello World", "hello world");
        assert!(diffs.is_empty());
    }

    #[test]
    fn skip_single_char() {
        let diffs = compute_diffs("a b", "x y");
        assert!(diffs.is_empty());
    }

    #[test]
    fn empty_texts() {
        let diffs = compute_diffs("", "");
        assert!(diffs.is_empty());
    }

    #[test]
    fn strip_punct_basic() {
        assert_eq!(strip_punct("hello"), "hello");
        assert_eq!(strip_punct("hello,"), "hello");
        assert_eq!(strip_punct("(hello)"), "hello");
        assert_eq!(strip_punct("\"hello\""), "hello");
        assert_eq!(strip_punct("..."), "");
    }
}
