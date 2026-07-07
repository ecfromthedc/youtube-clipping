//! Commentary format — react / narrate over a source clip.
//!
//! Kellan's #4 format and his highest-RPM niche (35-40¢/1k vs the 20¢ average).
//! Take an existing viral clip, write a commentary script, synthesize a VO, lay
//! the VO ON TOP of the clip's original audio (ducked), burn captions for the VO,
//! reframed to 9:16. The "transformative value" the algorithm rewards for monetization.
//!
//! Pipeline:
//!   1. Reframe source clip → 9:16 vertical (clip::cut_vertical on the whole clip)
//!   2. Synthesize VO via OmniVoice → voiceover.wav
//!   3. Transcribe VO with whisper → word timing for captions
//!   4. Render opus captions
//!   5. ffmpeg: vertical clip + VO mixed in at full vol + original audio ducked to ~25%

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::{captions, clip, transcribe, voice};

/// Commentary render options.
pub struct CommentaryOpts {
    /// Path to the source clip to commentate over.
    pub source: PathBuf,
    /// The commentary script to be spoken over the clip.
    pub script: String,
    /// OmniVoice voice id.
    pub voice: String,
    /// Optional on-screen hook title.
    pub title: Option<String>,
    /// VO speed multiplier (None = default).
    pub speed: Option<f32>,
    /// Language hint for TTS + whisper.
    pub language: Option<String>,
    /// How loud the original clip audio sits under the VO, 0.0–1.0. Default 0.25.
    pub duck_volume: f32,
}

impl Default for CommentaryOpts {
    fn default() -> Self {
        CommentaryOpts {
            source: PathBuf::new(),
            script: String::new(),
            voice: "default".to_string(),
            title: None,
            speed: None,
            language: None,
            duck_volume: 0.25,
        }
    }
}

/// Render a commentary Short. `out_path` should end in `.mp4`.
pub fn render(root: &Path, opts: &CommentaryOpts, out_path: &Path) -> Result<PathBuf> {
    if opts.script.trim().is_empty() {
        bail!("commentary script is empty");
    }
    if !opts.source.exists() {
        bail!("source clip not found: {}", opts.source.display());
    }
    let duck = opts.duck_volume.clamp(0.0, 1.0);

    let settings = crate::config::load_settings(root).ok();
    let workdir = std::env::temp_dir().join(format!("ycp-commentary-{}", std::process::id()));
    std::fs::create_dir_all(&workdir)?;

    let body = render_inner(root, opts, duck, &settings, &workdir, out_path);
    let _ = std::fs::remove_dir_all(&workdir);
    body
}

#[allow(clippy::too_many_arguments)]
fn render_inner(
    root: &Path,
    opts: &CommentaryOpts,
    duck: f32,
    settings: &Option<serde_yaml::Value>,
    workdir: &Path,
    out_path: &Path,
) -> Result<PathBuf> {
    // 1. Reframe source to 9:16 (cut the whole clip — start=0, end=duration).
    let src_dur = ffprobe_duration(&opts.source).max(0.5);
    let cand = clip::Candidate::new(0.0, src_dur, "", 0.0);
    let vertical = workdir.join("vertical.mp4");
    clip::cut_vertical(root, &opts.source, &cand, &vertical, workdir)
        .context("reframe source clip to 9:16")?;

    // 2. Synthesize the commentary VO.
    let vo_path = workdir.join("voiceover.wav");
    voice::synthesize(
        root,
        &opts.script,
        &opts.voice,
        &vo_path,
        opts.speed,
        opts.language.as_deref(),
    )
    .context("synthesize commentary VO")?;

    // 3. Transcribe VO for caption timing.
    let segs =
        transcribe::transcribe(root, &vo_path, workdir).context("transcribe commentary VO")?;
    let chunks = captions::build_chunks(&segs, captions::MAX_WORDS, captions::MIN_DWELL);

    // 4. Render caption PNGs.
    let cap_dir = workdir.join("capframes");
    captions::render_overlay(
        &chunks,
        src_dur,
        &cap_dir,
        opts.title.as_deref(),
        captions::SIZE,
        captions::FPS,
        None,
        settings.as_ref(),
    )?;

    // 5. Composite: vertical clip video + (VO at full vol + clip audio ducked) + captions.
    if let Some(p) = out_path.parent() {
        std::fs::create_dir_all(p).ok();
    }
    let frame_glob = cap_dir.join("%05d.png");
    let tmp = workdir.join("composed.mp4");

    // Filter graph:
    //   [1:v] = captions
    //   [0:v][1:v]overlay = vertical clip with captions
    //   [2:a]volume=1.0 = VO at full volume
    //   [0:a]volume=<duck> = clip audio ducked
    //   amix the two audio streams
    let filter = format!(
        "[0:v][1:v]overlay=0:0:format=auto:eof_action=pass[v];\
         [2:a]volume=1.0[vo];\
         [0:a]volume={duck}[bg];\
         [vo][bg]amix=inputs=2:duration=first:dropout_transition=0[a]"
    );

    let out = Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(&vertical) // input 0: vertical clip (v + a)
        .args([
            "-framerate",
            &captions::FPS.to_string(),
            "-start_number",
            "0",
            "-i",
        ])
        .arg(&frame_glob) // input 1: captions
        .arg("-i")
        .arg(&vo_path) // input 2: VO
        .args(["-filter_complex", &filter])
        .args(["-map", "[v]", "-map", "[a]"])
        .args([
            "-c:v", "libx264", "-c:a", "aac", "-preset", "veryfast", "-pix_fmt", "yuv420p",
        ])
        .args(["-shortest"])
        .arg(&tmp)
        .output()?;

    if !out.status.success() || !tmp.exists() {
        let err = String::from_utf8_lossy(&out.stderr);
        let tail = &err[err.len().saturating_sub(500)..];
        bail!("commentary compose failed: {}", tail.trim());
    }
    std::fs::rename(&tmp, out_path).or_else(|_| {
        std::fs::copy(&tmp, out_path)
            .map(|_| ())
            .and_then(|_| std::fs::remove_file(&tmp))
    })?;
    Ok(out_path.to_path_buf())
}

/// ffprobe → duration seconds; 0.0 if unreadable.
fn ffprobe_duration(path: &Path) -> f64 {
    match Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .trim()
            .parse()
            .unwrap_or(0.0),
        Err(_) => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commentary_opts_defaults_are_safe() {
        let o = CommentaryOpts::default();
        assert_eq!(o.voice, "default");
        assert_eq!(o.duck_volume, 0.25);
        assert!(o.script.is_empty());
    }

    #[test]
    fn duck_volume_clamps() {
        // Caller can pass out-of-range; render() clamps. Just exercise the clamp.
        let raw = 1.5_f32;
        let clamped = raw.clamp(0.0, 1.0);
        assert_eq!(clamped, 1.0);
    }
}
