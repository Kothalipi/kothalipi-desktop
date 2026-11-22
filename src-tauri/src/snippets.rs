//! Kothalipi: Voice Snippets / Quick Triggers.
//!
//! Lets users define spoken shortcut phrases (in Bangla or English) that expand
//! instantly into full text, email addresses, links, or templates — e.g.:
//! - "আমার ইমেইল" -> "tanvir@kothalipi.com"
//! - "meeting link" -> "https://cal.com/tanvir/30min"

use crate::settings::VoiceSnippet;

/// Strip surrounding punctuation and collapse whitespace for resilient matching.
fn normalize_phrase(s: &str) -> String {
    let trimmed = s.trim().trim_matches(|c: char| {
        matches!(
            c,
            '.' | ',' | '!' | '?' | ';' | ':' | '।' | '"' | '\'' | '“' | '”' | '—' | '-'
        )
    });
    trimmed
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Apply configured voice snippets to a transcribed text.
///
/// 1. If the entire utterance matches a snippet trigger (ignoring case and
///    trailing punctuation like `.` or `।`), return the expansion directly.
/// 2. Otherwise, for multi-word triggers (or exact phrase matches), replace
///    occurrences inline (longest trigger first).
pub fn apply_voice_snippets(text: &str, snippets: &[VoiceSnippet]) -> String {
    if text.trim().is_empty() || snippets.is_empty() {
        return text.to_string();
    }

    let mut active: Vec<(&VoiceSnippet, String)> = snippets
        .iter()
        .filter_map(|s| {
            let norm = normalize_phrase(&s.trigger);
            if norm.is_empty() || s.expansion.is_empty() {
                None
            } else {
                Some((s, norm))
            }
        })
        .collect();

    if active.is_empty() {
        return text.to_string();
    }

    // Longest trigger first so "আমার অফিস ইমেইল" wins over "আমার ইমেইল".
    active.sort_by(|a, b| b.1.chars().count().cmp(&a.1.chars().count()));

    // 1. Exact full-utterance match
    let norm_text = normalize_phrase(text);
    for (snippet, norm_trigger) in &active {
        if norm_text == *norm_trigger {
            return snippet.expansion.clone();
        }
    }

    // 2. Inline replacement for triggers appearing inside a longer sentence
    let mut output = text.to_string();
    for (snippet, _) in &active {
        let trigger = snippet.trigger.trim();
        if trigger.is_empty() {
            continue;
        }
        // Case-insensitive replacement for ASCII triggers, exact for Bangla/Unicode
        if trigger.is_ascii() {
            let lower_out = output.to_lowercase();
            let lower_trig = trigger.to_lowercase();
            if let Some(pos) = lower_out.find(&lower_trig) {
                let end = pos + lower_trig.len();
                let before_ok = pos == 0
                    || !output[..pos]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_alphanumeric());
                let after_ok = end == output.len()
                    || !output[end..]
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_alphanumeric());
                if before_ok && after_ok {
                    output.replace_range(pos..end, &snippet.expansion);
                }
            }
        } else if output.contains(trigger) {
            output = output.replace(trigger, &snippet.expansion);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snip(trigger: &str, expansion: &str) -> VoiceSnippet {
        VoiceSnippet {
            id: "1".into(),
            trigger: trigger.into(),
            expansion: expansion.into(),
        }
    }

    #[test]
    fn exact_bangla_and_english_triggers_expand() {
        let snippets = vec![
            snip("আমার ইমেইল", "tanvir@kothalipi.com"),
            snip("meeting link", "https://cal.com/tanvir/30min"),
        ];
        assert_eq!(
            apply_voice_snippets("আমার ইমেইল।", &snippets),
            "tanvir@kothalipi.com"
        );
        assert_eq!(
            apply_voice_snippets("Meeting Link.", &snippets),
            "https://cal.com/tanvir/30min"
        );
    }

    #[test]
    fn inline_trigger_expands_inside_sentence() {
        let snippets = vec![snip("আমার ইমেইল", "tanvir@kothalipi.com")];
        assert_eq!(
            apply_voice_snippets("প্লিজ আমার ইমেইল এ পাঠিয়ে দিও", &snippets),
            "প্লিজ tanvir@kothalipi.com এ পাঠিয়ে দিও"
        );
    }

    #[test]
    fn longest_trigger_wins() {
        let snippets = vec![
            snip("আমার ইমেইল", "personal@example.com"),
            snip("আমার অফিস ইমেইল", "work@company.com"),
        ];
        assert_eq!(
            apply_voice_snippets("আমার অফিস ইমেইল।", &snippets),
            "work@company.com"
        );
    }
}
