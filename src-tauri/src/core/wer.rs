//! Word error rate, used by the bench harness and the pipeline tests.

/// Lower-case, drop punctuation except apostrophes inside words, split on
/// whitespace.
pub fn normalize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric() || *c == '\'')
                .flat_map(char::to_lowercase)
                .collect::<String>()
                .trim_matches('\'')
                .to_string()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// Word-level Levenshtein distance between two normalised word lists.
pub fn edit_distance(reference: &[String], hypothesis: &[String]) -> usize {
    let mut prev: Vec<usize> = (0..=hypothesis.len()).collect();
    let mut cur = vec![0usize; hypothesis.len() + 1];
    for (i, r) in reference.iter().enumerate() {
        cur[0] = i + 1;
        for (j, h) in hypothesis.iter().enumerate() {
            let sub = prev[j] + usize::from(r != h);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[hypothesis.len()]
}

/// Word error rate of `hypothesis` against `reference`, as a fraction.
/// An empty reference gives 0 for an empty hypothesis and 1 otherwise.
pub fn wer(reference: &str, hypothesis: &str) -> f64 {
    let r = normalize(reference);
    let h = normalize(hypothesis);
    if r.is_empty() {
        return if h.is_empty() { 0.0 } else { 1.0 };
    }
    edit_distance(&r, &h) as f64 / r.len() as f64
}

/// Totals over many clips: (errors, reference words).
pub fn counts(reference: &str, hypothesis: &str) -> (usize, usize) {
    let r = normalize(reference);
    let h = normalize(hypothesis);
    (edit_distance(&r, &h), r.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_text_is_zero() {
        assert_eq!(wer("Hello, world.", "hello world"), 0.0);
    }

    #[test]
    fn counts_substitutions_insertions_deletions() {
        assert!((wer("a b c d", "a x c d") - 0.25).abs() < 1e-9);
        assert!((wer("a b c d", "a b c d e") - 0.25).abs() < 1e-9);
        assert!((wer("a b c d", "a c d") - 0.25).abs() < 1e-9);
        assert_eq!(wer("", ""), 0.0);
        assert_eq!(wer("", "x"), 1.0);
    }

    #[test]
    fn keeps_inner_apostrophes() {
        assert_eq!(normalize("Don't 'quote' me!"), vec!["don't", "quote", "me"]);
    }
}
