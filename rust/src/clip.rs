//! Hybrid clip pipeline — the free, uncapped volume engine. Parity port of `src/ycp/clip.py`.
//!
//! `score_candidate`, `plan_clips`, `window_text`, `Candidate` are pure and cross-checked
//! byte-for-byte against the Python (`ycp plan` / `ycp score-cand`). `download` + `cut_vertical`
//! shell out to yt-dlp/ffmpeg (→ `reframe`) exactly as the Python does.
//!
//! `run()` is the full orchestrator (vision moment-picking → cut → A/B hook sets → caption burn
//! → gameplay stack → archive → register pending_qc). It mirrors clip.py `run` step-for-step;
//! the native shell-outs (yt-dlp/ffmpeg/whisper) + live API calls (DeepSeek hooks, Gemini vision)
//! are not byte-diffable, but the structure, clip-id hashing (sha1[:8]), windowing and DB writes
//! are exact. Wired into `autopilot`.
#![allow(dead_code)] // some helpers/fields are parity-carried, exercised only via autopilot

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{bail, Result};

use crate::srt::{slice_and_shift, Segment};
use crate::util::round_to;
use crate::{archive, captions, config, db, enhance, hooks, optimize, reframe, transcribe, vision};

pub const MAX_CLIP_SEC: f64 = 38.0; // hard cap on a clip window (20-35s sweet spot; matches Python)

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
        let start = buf[0].start;
        let end = (buf[buf.len() - 1].end).min(start + max_len); // cap window at max_len (mirror Python)
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

// ── full pipeline (orchestrator) ─────────────────────────────────────────────

fn clips_dir(root: &Path) -> PathBuf {
    root.join("data").join("clips")
}

/// `hashlib.sha1(url.encode()).hexdigest()[:8]` — byte-identical clip-id prefix.
fn sha1_hex8(url: &str) -> String {
    use sha1::{Digest, Sha1};
    let digest = Sha1::digest(url.as_bytes());
    let mut hex = String::with_capacity(8);
    for b in digest.iter().take(4) {
        hex.push_str(&format!("{b:02x}"));
    }
    hex
}

/// Per-call clipping options (mirrors the keyword args of clip.py `run`). `hook_cta` is carried
/// for signature parity but — as in Python — unused by the caption-burn path.
pub struct RunOpts<'a> {
    pub max_clips: usize,
    pub lane: &'a str,
    pub source_creator: &'a str,
    pub channel: &'a str,
    pub hook_cta: bool,
    pub title: Option<&'a str>,
    pub gameplay: Option<&'a Path>,
    pub source_video_id: Option<&'a str>,
    pub angle: &'a str,
    pub window_sec: Option<i64>,
    pub captions_on: bool,
}

impl Default for RunOpts<'_> {
    fn default() -> Self {
        RunOpts {
            max_clips: 6,
            lane: "owned",
            source_creator: "unknown",
            channel: "clips",
            hook_cta: true,
            title: None,
            gameplay: None,
            source_video_id: None,
            angle: "",
            window_sec: None,
            captions_on: true,
        }
    }
}

/// A produced clip (mirrors the dicts clip.py `run` returns; autopilot reads only the count).
pub struct Created {
    pub clip_id: String,
    pub file: String,
    pub score: f64,
    pub len: f64,
    pub preview: String,
}

/// Full pipeline: url → ranked vertical clips with hook title + captions, registered for QC.
/// Mirrors clip.py `run`. The temp workdir is removed on the way out (best-effort), like the
/// Python `TemporaryDirectory`.
pub fn run(conn: &rusqlite::Connection, root: &Path, url: &str, opts: &RunOpts) -> Result<Vec<Created>> {
    let settings = config::load_settings(root)?;
    let mut created: Vec<Created> = Vec::new();
    let vid_hash = sha1_hex8(url);
    let workdir = std::env::temp_dir().join(format!("ycp-clip-{}-{vid_hash}", std::process::id()));
    std::fs::create_dir_all(&workdir)?;

    let body = run_inner(conn, root, url, opts, &settings, &vid_hash, &workdir, &mut created);
    let _ = std::fs::remove_dir_all(&workdir);
    body?;
    Ok(created)
}

#[allow(clippy::too_many_arguments)]
fn run_inner(
    conn: &rusqlite::Connection,
    root: &Path,
    url: &str,
    opts: &RunOpts,
    settings: &serde_yaml::Value,
    vid_hash: &str,
    workdir: &Path,
    created: &mut Vec<Created>,
) -> Result<()> {
    let video = download(url, workdir, opts.window_sec)?;
    let segments = transcribe::transcribe(root, &video, workdir)?;

    // Gemini picks moments by watching the footage; else the transcript heuristic.
    let moments = vision::rank_moments(root, &video, opts.max_clips, settings);
    let candidates: Vec<Candidate> = if !moments.is_empty() {
        println!("  · Gemini vision picked {} moment(s)", moments.len());
        moments
            .iter()
            .map(|m| {
                let end = m.end.min(m.start + MAX_CLIP_SEC);
                Candidate::new(m.start, end, window_text(&segments, m.start, end), m.score)
            })
            .collect()
    } else {
        plan_clips(&segments, 15.0, MAX_CLIP_SEC, Some(opts.max_clips))
    };

    // A/B only the SINGLE best moment per source (mirror Python) — gating every hero explodes variants.
    let top_idx = candidates
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i);

    let prefer = optimize::preferred_hooks(&optimize::Paths::new(root));
    let ab = &settings["ab"];
    let ab_enabled = ab["enabled"].as_bool().unwrap_or(true);
    let hero_score = ab["hero_score"].as_f64().unwrap_or(0.9);
    let variants_k = ab["variants"].as_i64().unwrap_or(3) as usize;

    for (i, cand) in candidates.iter().enumerate() {
        let clip_id = format!("{vid_hash}-{i:02}");
        // captions_on=false → no chunks → hook renders alone (defer to the source's captions).
        let chunks = if opts.captions_on {
            captions::build_chunks(
                &slice_and_shift(&segments, cand.start, cand.end),
                captions::MAX_WORDS,
                captions::MIN_DWELL,
            )
        } else {
            Vec::new()
        };
        let staged = workdir.join(format!("{clip_id}.mp4"));
        if let Err(exc) = cut_vertical(root, &video, cand, &staged, workdir) {
            println!("  ! skip {clip_id}: {exc}");
            continue;
        }

        // Pick the hook set: a manual title, an A/B hero set, or a single best hook.
        let (hook_set, exp_id): (Vec<hooks::Hook>, Option<String>) = if let Some(t) = opts.title {
            (vec![hooks::Hook { text: t.to_string(), typ: "manual".to_string() }], None)
        } else if ab_enabled && Some(i) == top_idx && cand.score >= hero_score {
            let hs = hooks::variants(root, &cand.text, opts.angle, variants_k, 10, None, &prefer);
            let eid = if hs.len() > 1 { Some(format!("{clip_id}-ab")) } else { None };
            (hs, eid)
        } else {
            (vec![hooks::best(root, &cand.text, opts.angle, 6, 10, None, &prefer)], None)
        };
        if exp_id.is_some() {
            println!("  · hero moment (score {:.2}) → A/B {} hook angles", cand.score, hook_set.len());
        }

        for (vi, hook) in hook_set.iter().enumerate() {
            let variant_id =
                if hook_set.len() == 1 { clip_id.clone() } else { format!("{clip_id}-v{vi}") };
            let mut cur = staged.clone();
            match captions::burn_captions(
                &staged,
                &chunks,
                &workdir.join(format!("{variant_id}_cap.mp4")),
                workdir,
                Some(&hook.text),
                captions::FPS,
                captions::SIZE,
                None,
                Some(settings),
            ) {
                Ok(p) => cur = p,
                Err(exc) => println!("  · captions failed ({exc}); shipping plain clip"),
            }
            if let Some(gp) = opts.gameplay {
                cur = enhance::stack_gameplay(&cur, gp, &workdir.join(format!("{variant_id}_gp.mp4")))?;
            }
            let out = clips_dir(root).join(format!("{variant_id}.mp4"));
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p).ok();
            }
            // Copy (not move) when the burn fell back to `staged`, so the shared base survives
            // for the other variants in this A/B set.
            if cur == staged {
                std::fs::copy(&staged, &out)?;
            } else {
                std::fs::rename(&cur, &out).or_else(|_| {
                    std::fs::copy(&cur, &out).map(|_| ()).and_then(|_| std::fs::remove_file(&cur))
                })?;
            }
            db::insert_clip(
                conn,
                &db::NewClip {
                    clip_id: variant_id.clone(),
                    source_video_id: opts.source_video_id.map(String::from),
                    source_creator: Some(opts.source_creator.to_string()),
                    channel: opts.channel.to_string(),
                    platform: "youtube".to_string(),
                    lane: opts.lane.to_string(),
                    fmt: Some("auto-clip".to_string()),
                    hook_type: Some(hook.typ.clone()),
                    length_sec: Some(cand.duration() as i64),
                    post_title: Some(hook.text.clone()),
                    experiment_id: exp_id.clone(),
                    variant: exp_id.as_ref().map(|_| hook.typ.clone()),
                    post_url: Some(out.display().to_string()),
                },
            )?;
            created.push(Created {
                clip_id: variant_id.clone(),
                file: out.display().to_string(),
                score: cand.score,
                len: cand.duration(),
                preview: cand.text.chars().take(80).collect(),
            });
            // Archive to the Phoenix Protocol drive (best-effort; never blocks posting).
            let meta = serde_json::json!({
                "clip_id": variant_id,
                "channel": opts.channel,
                "hook": hook.text,
                "hook_type": hook.typ,
                "source_creator": opts.source_creator,
                "score": cand.score,
                "length_sec": cand.duration() as i64,
                "experiment_id": exp_id,
            });
            if let Some(dest) = archive::archive_clip(settings, &out, &meta) {
                println!("  · archived → {dest}");
            }
        }
    }
    Ok(())
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
