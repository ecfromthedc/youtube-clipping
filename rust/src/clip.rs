//! Hybrid clip pipeline — the free, uncapped volume engine. Parity port of the PURE +
//! native-step half of `src/ycp/clip.py`.
//!
//! `score_candidate`, `plan_clips`, `window_text`, `Candidate` are pure and cross-checked
//! byte-for-byte against the Python (`ycp plan` / `ycp score-cand`). `download` + `cut_vertical`
//! shell out to yt-dlp/ffmpeg (→ `reframe`) exactly as the Python does.
//!
//! The full `run()` orchestrator (vision moment-picking, A/B hook sets, Pillow caption burn,
//! gameplay stacking, archive) is intentionally deferred to the autopilot row: it depends on
//! the unported "captions render" row + vision.py + enhance.stack_gameplay. Same staging the
//! capture/distribute ports used — pure cores first, orchestration last.
#![allow(dead_code)] // download/cut_vertical wired by the autopilot row

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{bail, Result};

use crate::config;
use crate::reframe;
use crate::srt::Segment;
use crate::util::round_to;

pub const MAX_CLIP_SEC: f64 = 45.0; // hard cap on a clip window

fn hook_words() -> &'static HashSet<&'static str> {
    static W: OnceLock<HashSet<&'static str>> = OnceLock::new();
    W.get_or_init(|| {
        [
            "why", "how", "never", "secret", "nobody", "actually", "truth", "mistake", "stop",
            "biggest", "worst", "best", "everyone", "wrong",
        ]
        .into_iter()
        .collect()
    })
}

/// A scored candidate window (mirrors the frozen `Candidate` dataclass).
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub score: f64,
}

impl Candidate {
    pub fn new(start: f64, end: f64, text: impl Into<String>, score: f64) -> Self {
        Self { start, end, text: text.into(), score }
    }
    /// round(end - start, 2)
    pub fn duration(&self) -> f64 {
        round_to(self.end - self.start, 2)
    }
}

/// Cheap, deterministic 'is this a hook?' heuristic. Higher = more promising. Mirrors
/// `score_candidate`. Pure.
pub fn score_candidate(text: &str, duration: f64) -> f64 {
    let t = text.to_lowercase();
    let mut score = 1.0;
    score += t.matches('?').count() as f64 * 0.6;
    score += t.matches('!').count() as f64 * 0.3;
    score += if t.chars().any(|c| c.is_ascii_digit()) { 0.4 } else { 0.0 };
    let tokens: Vec<&str> = t.split_whitespace().collect();
    let hooks = hook_words().iter().filter(|w| tokens.contains(*w)).count();
    score += 0.3 * hooks as f64;
    // duration sweet spot ~30s; taper outside [18, 50]
    if (18.0..=50.0).contains(&duration) {
        score += 1.0;
    } else {
        score -= ((duration - 32.0).abs() / 20.0).min(1.5);
    }
    round_to(score, 3)
}

/// Group consecutive segments into candidate windows at the max_len boundary, score each, return
/// ranked (best first). Mirrors `plan_clips`. Pure.
pub fn plan_clips(segments: &[Segment], min_len: f64, max_len: f64, top: Option<usize>) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut buf: Vec<&Segment> = Vec::new();

    let flush = |buf: &[&Segment], out: &mut Vec<Candidate>| {
        if buf.is_empty() {
            return;
        }
        let (start, end) = (buf[0].start, buf[buf.len() - 1].end);
        if end - start >= min_len {
            let text = buf.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ").trim().to_string();
            let score = score_candidate(&text, end - start);
            out.push(Candidate::new(start, end, text, score));
        }
    };

    for seg in segments {
        if !buf.is_empty() && (seg.end - buf[0].start) > max_len {
            flush(&buf, &mut candidates);
            buf.clear();
        }
        buf.push(seg);
    }
    flush(&buf, &mut candidates);

    // sorted(..., reverse=True) — stable on score ties.
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    match top {
        Some(n) => candidates.into_iter().take(n).collect(),
        None => candidates,
    }
}

/// Transcript text overlapping [start, end] — the hook + caption source for a window.
/// Mirrors `_window_text`. Pure.
pub fn window_text(segments: &[Segment], start: f64, end: f64) -> String {
    segments
        .iter()
        .filter(|s| s.end > start && s.start < end)
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

// ── native steps (thin shell wrappers) ───────────────────────────────────────

/// Download a source video with yt-dlp; bound long sources to the first `window_sec`. Mirrors
/// `download`.
pub fn download(url: &str, workdir: &Path, window_sec: Option<i64>) -> Result<PathBuf> {
    let out = workdir.join("source.mp4");
    let mut cmd = Command::new("yt-dlp");
    cmd.args(["-f", "mp4/best", "-o"]).arg(&out);
    if let Some(w) = window_sec {
        cmd.args(["--download-sections", &format!("*0-{w}"), "--force-keyframes-at-cuts"]);
    }
    cmd.arg(url);
    let proc = cmd.output()?;
    if out.exists() {
        return Ok(out);
    }
    // yt-dlp may have remuxed to a sibling extension.
    if let Ok(entries) = std::fs::read_dir(workdir) {
        for e in entries.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("source") && (name.ends_with(".mp4") || name.ends_with(".mkv")) {
                return Ok(e.path());
            }
        }
    }
    let err = String::from_utf8_lossy(&proc.stderr);
    bail!("download failed: {}", &err.trim()[..err.trim().len().min(300)]);
}

/// Trim the candidate window, then reframe to a 9:16 vertical. Mirrors `cut_vertical`.
pub fn cut_vertical(root: &Path, video: &Path, cand: &Candidate, out_path: &Path, workdir: &Path) -> Result<PathBuf> {
    let trimmed = workdir.join("trim.mp4");
    let proc = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(video)
        .args(["-ss", &fmt_secs(cand.start), "-t", &fmt_secs(cand.duration())])
        .args(["-c:v", "libx264", "-c:a", "aac", "-preset", "veryfast"])
        .arg(&trimmed)
        .current_dir(workdir)
        .output()?;
    if !proc.status.success() || !trimmed.exists() {
        let err = String::from_utf8_lossy(&proc.stderr);
        let e = err.trim();
        bail!("ffmpeg trim failed: {}", &e[e.len().saturating_sub(400)..]);
    }
    let mode = config::load_settings(root)
        .ok()
        .and_then(|s| s["reframe"]["mode"].as_str().map(String::from))
        .unwrap_or_else(|| "face".to_string());
    reframe::reframe(&trimmed, out_path, workdir, &mode, (reframe::TARGET_W, reframe::TARGET_H))
}

/// Plain decimal seconds for ffmpeg -ss/-t (no scientific notation).
fn fmt_secs(x: f64) -> String {
    let s = format!("{x}");
    if s.contains('e') || s.contains('E') { format!("{x:.3}") } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_rewards_hooks_and_sweet_spot_duration() {
        let hooky = score_candidate("Why did nobody tell you the truth about 3 mistakes?", 30.0);
        let bland = score_candidate("and then we walked over there slowly", 30.0);
        assert!(hooky > bland);
        assert!(score_candidate("a normal sentence", 30.0) > score_candidate("a normal sentence", 90.0));
    }

    #[test]
    fn plan_clips_windows_and_ranks() {
        let segs = vec![
            Segment::new(0.0, 12.0, "intro rambling here"),
            Segment::new(12.0, 24.0, "why is this the biggest mistake? 5 reasons!"),
            Segment::new(24.0, 36.0, "calm outro"),
        ];
        let cands = plan_clips(&segs, 15.0, 60.0, None);
        assert!(!cands.is_empty());
        assert!(cands.iter().all(|c| (15.0..=60.0).contains(&c.duration())));
        // ranked best-first
        let mut sorted = cands.clone();
        sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        assert_eq!(cands, sorted);
    }

    #[test]
    fn plan_clips_respects_max_len() {
        let segs: Vec<Segment> =
            (0..10).map(|i| Segment::new(i as f64 * 10.0, (i + 1) as f64 * 10.0, format!("seg {i}"))).collect();
        let cands = plan_clips(&segs, 15.0, 40.0, None);
        assert!(cands.iter().all(|c| c.duration() <= 40.0));
    }

    #[test]
    fn window_text_overlap_only() {
        let segs = vec![
            Segment::new(0.0, 5.0, "alpha"),
            Segment::new(5.0, 10.0, "beta"),
            Segment::new(10.0, 15.0, "gamma"),
        ];
        assert_eq!(window_text(&segs, 4.0, 11.0), "alpha beta gamma");
        assert_eq!(window_text(&segs, 5.5, 9.0), "beta");
    }
}
