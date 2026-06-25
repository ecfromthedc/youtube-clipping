//! Owned ffmpeg enhancements — parity port of `src/ycp/enhance.py`.
//!
//! Only `pick_title` (the zero-dependency hook heuristic) is ported so far — it's the
//! fallback `hooks::best` uses when DeepSeek is unavailable. The native ffmpeg builders
//! (title/CTA overlay, gameplay vstack) land with the "native pipeline" row.

/// Heuristic hook title from the transcript: first question, else the longest line.
///
/// Mirrors enhance.py `pick_title` — zero-dependency fallback when the DeepSeek hook
/// agent is unavailable (no DEEPSEEK_API_KEY).
pub fn pick_title(transcript: &str, max_words: usize) -> String {
    // Python: transcript.replace("!", ".").replace("?", "?.").split(".") then strip+drop-empty.
    let replaced = transcript.replace('!', ".").replace('?', "?.");
    let sentences: Vec<&str> = replaced
        .split('.')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if sentences.is_empty() {
        return String::new();
    }
    // First question wins; else max(by char-length). Python `max` returns the FIRST max on
    // ties, so keep only strictly-longer to match.
    let pick = sentences.iter().find(|s| s.ends_with('?')).copied().unwrap_or_else(|| {
        let mut best = sentences[0];
        for &s in &sentences[1..] {
            if s.chars().count() > best.chars().count() {
                best = s;
            }
        }
        best
    });
    let words: Vec<&str> = pick.split_whitespace().collect();
    let joined = words.iter().take(max_words).copied().collect::<Vec<_>>().join(" ");
    if words.len() > max_words {
        format!("{joined}…")
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_the_question() {
        let out = pick_title("Why does nobody talk about this? It changes everything.", 9);
        assert_eq!(out, "Why does nobody talk about this?");
    }

    #[test]
    fn falls_back_to_longest_line() {
        let out = pick_title("Short. The longest sentence here is this one. Mid line.", 9);
        assert_eq!(out, "The longest sentence here is this one");
    }

    #[test]
    fn empty_transcript_is_empty() {
        assert_eq!(pick_title("   ", 9), "");
    }

    #[test]
    fn trims_to_max_words_with_ellipsis() {
        let out = pick_title("one two three four five six seven eight", 3);
        assert_eq!(out, "one two three…");
    }
}
