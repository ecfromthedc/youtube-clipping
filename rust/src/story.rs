//! Storytelling format — the "Roblox Rants" / "Meme Story" pattern.
//!
//! Kellan's #2 format: take a script (a funny story, hot take, or narrated post),
//! synthesize an AI voiceover, lay it over looping background footage (gameplay,
//! Minecraft parkour, subway surfer, etc.), burn opus-style captions timed to the
//! VO, and emit a 9:16 vertical Short ready to post.
//!
//! This is the first format in the engine that GENERATES content from a script
//! rather than clipping existing footage — it turns Tides & Ships from a clip
//! editor into a content factory (the playbook's actual shape).
//!
//! Pipeline:
//!   1. Synthesize VO via OmniVoice (voice.rs) → voiceover.wav
//!   2. Probe VO duration; loop/trim the background gameplay to match
//!   3. Transcribe the VO with whisper → word-level timing
//!   4. Render opus captions from the transcript
//!   5. ffmpeg: loop bg + replace audio with VO + overlay caption PNGs → 9:16 mp4

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::{captions, srt, transcribe, voice};

/// Storytelling render options.
pub struct StoryOpts {
    /// The script to be spoken. Required.
    pub script: String,
    /// OmniVoice voice id ("default", a preset, or a cloned-profile id).
    pub voice: String,
    /// Path to looping background footage (gameplay, Minecraft, etc.).
    pub background: PathBuf,
    /// Optional on-screen hook title (burned at the top, holds 7s).
    pub title: Option<String>,
    /// Optional speed multiplier for the VO (None = let OmniVoice pick).
    pub speed: Option<f32>,
    /// Optional ISO 639-1 language hint for both TTS and whisper.
    pub language: Option<String>,
}

impl Default for StoryOpts {
    fn default() -> Self {
        StoryOpts {
            script: String::new(),
            voice: "default".to_string(),
            background: PathBuf::new(),
            title: None,
            speed: None,
            language: None,
        }
    }
}

/// Render a storytelling Short. `out_path` should end in `.mp4`.
pub fn render(root: &Path, opts: &StoryOpts, out_path: &Path) -> Result<PathBuf> {
    if opts.script.trim().is_empty() {
        bail!("storytelling script is empty");
    }
    if !opts.background.exists() {
        bail!(
            "background footage not found: {}",
            opts.background.display()
        );
    }

    let settings = crate::config::load_settings(root).ok();
    let workdir = std::env::temp_dir().join(format!("ycp-story-{}", std::process::id()));
    std::fs::create_dir_all(&workdir)?;

    let body = render_inner(root, opts, &settings, &workdir, out_path);
    let _ = std::fs::remove_dir_all(&workdir);
    body
}

#[allow(clippy::too_many_arguments)]
fn render_inner(
    root: &Path,
    opts: &StoryOpts,
    settings: &Option<serde_yaml::Value>,
    workdir: &Path,
    out_path: &Path,
) -> Result<PathBuf> {
    // 1. Synthesize VO via OmniVoice.
    let vo_path = workdir.join("voiceover.wav");
    voice::synthesize(
        root,
        &opts.script,
        &opts.voice,
        &vo_path,
        opts.speed,
        opts.language.as_deref(),
    )
    .with_context(|| "synthesize VO via OmniVoice")?;

    // 2. Probe VO duration.
    let vo_dur = ffprobe_duration(&vo_path);
    if vo_dur <= 0.0 {
        bail!("VO duration unreadable");
    }

    // 3. Transcribe the VO with whisper → word-level segments for caption timing.
    //    Use the same workdir; whisper writes transcript.srt alongside.
    let segs = transcribe::transcribe(root, &vo_path, workdir)
        .with_context(|| "transcribe VO for caption timing")?;

    // 4. Build caption chunks (1-3 words, opus style).
    let chunks = captions::build_chunks(&segs, captions::MAX_WORDS, captions::MIN_DWELL);

    // 5. Render the caption PNG sequence (1080×1920 @ 15fps).
    let cap_dir = workdir.join("capframes");
    captions::render_overlay(
        &chunks,
        vo_dur,
        &cap_dir,
        opts.title.as_deref(),
        captions::SIZE,
        captions::FPS,
        None,
        settings.as_ref(),
    )?;

    // 6. Composite: background looped/trimmed to VO length + VO audio + caption overlay.
    if let Some(p) = out_path.parent() {
        std::fs::create_dir_all(p).ok();
    }
    let frame_glob = cap_dir.join("%05d.png");
    let tmp = workdir.join("composed.mp4");

    // filter graph:
    //   [0:v] scale + crop to 1080x1920, then loop the (short) bg via -stream_loop on input 0
    //   [0:v][1:v] overlay captions
    //   audio comes from the VO (input 2), not the background
    let (w, h) = (captions::SIZE.0, captions::SIZE.1);
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y")
        // input 0: background video, looped so it always covers VO duration
        .args(["-stream_loop", "-1", "-i"])
        .arg(&opts.background)
        // input 1: caption PNG sequence
        .args([
            "-framerate", &captions::FPS.to_string(),
            "-start_number", "0",
            "-i",
        ])
        .arg(&frame_glob)
        // input 2: voiceover audio (replaces background audio)
        .arg("-i").arg(&vo_path);

    // filter: scale bg to cover 9:16, crop center, overlay captions, then trim to VO length
    let filter = format!(
        "[0:v]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},setsar=1[bg];\
         [bg][1:v]overlay=0:0:format=auto:eof_action=pass[v]"
    );
    cmd.args(["-filter_complex", &filter])
        .args(["-map", "[v]", "-map", "2:a"])
        .args(["-c:v", "libx264", "-c:a", "aac", "-preset", "veryfast", "-pix_fmt", "yuv420p"])
        .args(["-shortest"]) // stop at the shorter of (looped bg, VO)
        .arg(&tmp);

    let out = cmd.output()?;
    if !out.status.success() || !tmp.exists() {
        let err = String::from_utf8_lossy(&out.stderr);
        let tail = &err[err.len().saturating_sub(500)..];
        bail!("storytelling compose failed: {}", tail.trim());
    }
    std::fs::rename(&tmp, out_path).or_else(|_| {
        std::fs::copy(&tmp, out_path).map(|_| ()).and_then(|_| std::fs::remove_file(&tmp))
    })?;
    Ok(out_path.to_path_buf())
}

/// ffprobe → duration seconds; 0.0 if unreadable.
fn ffprobe_duration(path: &Path) -> f64 {
    match Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"])
        .arg(path)
        .output()
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().parse().unwrap_or(0.0),
        Err(_) => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn story_opts_defaults_are_safe() {
        let o = StoryOpts::default();
        assert_eq!(o.voice, "default");
        assert!(o.script.is_empty());
        assert!(o.title.is_none());
        assert!(o.speed.is_none());
    }
}
