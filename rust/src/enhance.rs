//! Owned ffmpeg enhancements — parity port of `src/ycp/enhance.py`.
//!
//! `pick_title` is the zero-dependency hook heuristic `hooks::best` falls back to.
//! `stack_gameplay` (split-screen retention) shells out to ffmpeg, mirroring `vstack_cmd`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Result};

/// ffmpeg args to stack `clip` over looping `gameplay` (split-screen). Mirrors `vstack_cmd`:
/// gameplay loops to the clip's length, clip audio kept, gameplay audio dropped. Pure builder.
pub fn vstack_cmd(clip: &Path, gameplay: &Path, out: &Path) -> Vec<String> {
    let (top_h, bottom_h, width) = (1152, 768, 1080);
    let fc = format!(
        "[0:v]scale={width}:{top_h}:force_original_aspect_ratio=increase,crop={width}:{top_h}[top];\
         [1:v]scale={width}:{bottom_h}:force_original_aspect_ratio=increase,crop={width}:{bottom_h},setsar=1[bot];\
         [top][bot]vstack=inputs=2[v]"
    );
    vec![
        "-y".into(),
        "-i".into(),
        clip.display().to_string(),
        "-stream_loop".into(),
        "-1".into(),
        "-i".into(),
        gameplay.display().to_string(),
        "-filter_complex".into(),
        fc,
        "-map".into(),
        "[v]".into(),
        "-map".into(),
        "0:a?".into(),
        "-c:v".into(),
        "libx264".into(),
        "-c:a".into(),
        "aac".into(),
        "-preset".into(),
        "veryfast".into(),
        "-shortest".into(),
        out.display().to_string(),
    ]
}

/// Stack `clip` over a looping `gameplay` file (split-screen retention). Mirrors `stack_gameplay`.
pub fn stack_gameplay(clip: &Path, gameplay: &Path, out: &Path) -> Result<PathBuf> {
    if !gameplay.exists() {
        bail!("gameplay loop not found: {}", gameplay.display());
    }
    let res = Command::new("ffmpeg")
        .args(vstack_cmd(clip, gameplay, out))
        .output()?;
    if !res.status.success() {
        let err = String::from_utf8_lossy(&res.stderr);
        bail!(
            "gameplay vstack failed: {}",
            err[err.len().saturating_sub(400)..].trim()
        );
    }
    Ok(out.to_path_buf())
}

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
    let pick = sentences
        .iter()
        .find(|s| s.ends_with('?'))
        .copied()
        .unwrap_or_else(|| {
            let mut best = sentences[0];
            for &s in &sentences[1..] {
                if s.chars().count() > best.chars().count() {
                    best = s;
                }
            }
            best
        });
    let words: Vec<&str> = pick.split_whitespace().collect();
    let joined = words
        .iter()
        .take(max_words)
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
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
