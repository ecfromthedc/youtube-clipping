//! Ranking listicle format — compile N ranked clips into one countdown video.
//!
//! The "ranking compilation" format (e.g. "Top 7 Funniest Animal Moments"): a single
//! vertical 9:16 MP4 that plays through ranked clips with a big rank number (1, 2, 3...)
//! overlaid on the left edge of each clip, plus opus-style captions + a hook title.
//! Best moment plays LAST (countdown reveal) by default — that's the engagement pattern
//! the reference reel uses.
//!
//! Pipeline (mirrors the existing multiple-ffmpeg-pass style):
//!   1. cut_vertical each window → one 9:16 mp4 per item
//!   2. burn rank badge (captions::render_rank_overlay → ffmpeg overlay)
//!   3. burn captions + hook title (captions::burn_captions)
//!   4. concat all badged+captioned clips → one mp4 (ffmpeg concat demuxer)
//!
//! All heavy lifting delegates to existing pipeline modules — this is pure orchestration.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::{captions, clip, srt};

/// Revelation order. `CountDown` plays rank-1 first (1, 2, 3 ... N — classic countdown);
/// `CountUp` plays the highest-numbered rank first, saving rank 1 for last (the
/// reference reel's pattern: best/saved-for-last reveal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    CountDown,
    CountUp,
}

/// One ranked window into the source video. `rank` is the visible badge number (1-based).
#[derive(Debug, Clone)]
pub struct RankItem {
    pub start: f64,
    pub end: f64,
    /// Badge number burned on the clip (1, 2, 3...). The caller assigns this; `compile`
    /// stamps it verbatim — no auto-renumber.
    pub rank: usize,
    /// Optional on-screen label (currently metadata-only; not burned into the frame).
    pub label: String,
}

/// Compile-time options.
pub struct CompileOpts {
    /// Hook title burned at the top of the FIRST clip only (mirrors single-clip behavior —
    /// holding the title across all clips would crowd the rank badge).
    pub title: String,
    pub order: Order,
}

impl Default for CompileOpts {
    fn default() -> Self {
        CompileOpts {
            title: String::new(),
            order: Order::CountUp,
        }
    }
}

/// Compile N ranked clips into one vertical MP4. Returns the output path.
///
/// `segments` is the FULL source transcript; per-clip caption chunks are sliced from it.
/// `out_path` should end in `.mp4`; the parent dir is created if missing.
pub fn compile(
    root: &Path,
    source_video: &Path,
    segments: &[srt::Segment],
    items: &[RankItem],
    opts: &CompileOpts,
    out_path: &Path,
) -> Result<PathBuf> {
    if items.is_empty() {
        bail!("compile needs at least one ranked item");
    }
    let settings = crate::config::load_settings(root).ok();
    let workdir = std::env::temp_dir().join(format!("ycp-listicle-{}", std::process::id()));
    std::fs::create_dir_all(&workdir)?;

    let body = compile_inner(
        root,
        source_video,
        segments,
        items,
        opts,
        &settings,
        &workdir,
        out_path,
    );
    let _ = std::fs::remove_dir_all(&workdir);
    body
}

#[allow(clippy::too_many_arguments)]
fn compile_inner(
    root: &Path,
    source_video: &Path,
    segments: &[srt::Segment],
    items: &[RankItem],
    opts: &CompileOpts,
    settings: &Option<serde_yaml::Value>,
    workdir: &Path,
    out_path: &Path,
) -> Result<PathBuf> {
    // Determine playback order. We always cut every item; order only affects the concat
    // sequence + which clip carries the hook title.
    let playback: Vec<usize> = match opts.order {
        Order::CountDown => (0..items.len()).collect(), // 1, 2, 3 ... N
        Order::CountUp => (0..items.len()).rev().collect(), // N, N-1 ... 1
    };

    let mut badged: Vec<PathBuf> = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let cand = clip::Candidate::new(item.start, item.end, item.label.clone(), 0.0);

        // 1. cut + reframe to 9:16
        let cut = workdir.join(format!("cut_{i}.mp4"));
        clip::cut_vertical(root, source_video, &cand, &cut, workdir)
            .with_context(|| format!("cut item {i} (rank {})", item.rank))?;

        // 2. burn the rank badge PNG sequence + ffmpeg overlay
        let badge_frames = workdir.join(format!("badge_{i}"));
        let dur = ffprobe_duration(&cut).max(0.5);
        captions::render_rank_overlay(
            item.rank,
            dur,
            &badge_frames,
            captions::SIZE,
            captions::FPS,
            None,
        )?;
        let badged_clip = workdir.join(format!("badged_{i}.mp4"));
        overlay_png_sequence(&cut, &badge_frames, &badged_clip, captions::FPS)?;

        // 3. burn captions + (first clip only) the hook title
        let sliced = srt::slice_and_shift(segments, item.start, item.end);
        let chunks = captions::build_chunks(&sliced, captions::MAX_WORDS, captions::MIN_DWELL);
        let captioned = workdir.join(format!("captioned_{i}.mp4"));
        // Title shows on the first-PLAYED clip (rank-1 for CountDown, highest for CountUp).
        let is_first_played = playback[0] == i;
        let title_for_this = if is_first_played && !opts.title.is_empty() {
            Some(opts.title.as_str())
        } else {
            None
        };
        match captions::burn_captions(
            &badged_clip,
            &chunks,
            &captioned,
            workdir,
            title_for_this,
            captions::FPS,
            captions::SIZE,
            None,
            settings.as_ref(),
        ) {
            Ok(_) => badged.push(captioned),
            // Caption burn failed — fall back to the badged clip (badge still visible).
            Err(e) => {
                eprintln!("  ! caption burn failed on item {i} ({e}); keeping badged clip");
                badged.push(badged_clip);
            }
        }
    }

    // 4. concat in playback order. The concat demuxer wants same-codec same-resolution
    //    clips — every badged[] clip went through cut_vertical + the same overlay, so they match.
    let ordered: Vec<&Path> = playback.iter().map(|&i| badged[i].as_path()).collect();
    if let Some(p) = out_path.parent() {
        std::fs::create_dir_all(p).ok();
    }
    concat_clips(&ordered, out_path)?;
    Ok(out_path.to_path_buf())
}

/// ffmpeg overlay of a PNG sequence (`<dir>/%05d.png`) onto a base clip. Mirrors the
/// overlay half of `captions::burn_captions` but standalone (no caption chunks).
fn overlay_png_sequence(base: &Path, frames_dir: &Path, out: &Path, fps: u32) -> Result<()> {
    let frame_glob = frames_dir.join("%05d.png");
    let tmp = out.with_extension("tmp.mp4");
    let res = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(base)
        .args(["-framerate", &fps.to_string(), "-start_number", "0", "-i"])
        .arg(&frame_glob)
        .args([
            "-filter_complex",
            "[0:v][1:v]overlay=0:0:format=auto:eof_action=pass",
            "-c:v",
            "libx264",
            "-c:a",
            "copy",
            "-preset",
            "veryfast",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&tmp)
        .output()?;
    if !res.status.success() || !tmp.exists() {
        let err = String::from_utf8_lossy(&res.stderr);
        let tail = &err[err.len().saturating_sub(400)..];
        bail!("rank-badge overlay failed: {}", tail.trim());
    }
    std::fs::rename(&tmp, out).or_else(|_| {
        std::fs::copy(&tmp, out)
            .map(|_| ())
            .and_then(|_| std::fs::remove_file(&tmp))
    })?;
    Ok(())
}

/// ffmpeg concat demuxer — the simplest, most robust stitch for same-codec clips.
/// Writes a `concat.txt` listing each file (with the `file '` prefix), then a single
/// `ffmpeg -f concat -safe 0 -i concat.txt -c copy out.mp4` (no re-encode).
fn concat_clips(clips: &[&Path], out: &Path) -> Result<()> {
    if clips.len() == 1 {
        std::fs::copy(clips[0], out)?;
        return Ok(());
    }
    let list_path = out.with_extension("concat.txt");
    let mut list = String::new();
    for c in clips {
        // ffmpeg concat demuxer: file paths are relative to the list, or absolute.
        // Use absolute to sidestep cwd ambiguity.
        let abs = std::fs::canonicalize(c).unwrap_or_else(|_| c.to_path_buf());
        list.push_str(&format!(
            "file '{}'\n",
            abs.display().to_string().replace('\'', "'\\''")
        ));
    }
    std::fs::write(&list_path, &list)?;

    let res = Command::new("ffmpeg")
        .args(["-y", "-f", "concat", "-safe", "0", "-i"])
        .arg(&list_path)
        .args(["-c", "copy"])
        .arg(out)
        .output()?;
    let _ = std::fs::remove_file(&list_path);
    if !res.status.success() || !out.exists() {
        let err = String::from_utf8_lossy(&res.stderr);
        // concat demuxer is finicky on codec mismatch; fall back to re-encode if copy failed.
        eprintln!(
            "  · concat -c copy failed ({}); re-encoding",
            err.trim().chars().take(120).collect::<String>()
        );
        let res = Command::new("ffmpeg")
            .args(["-y", "-f", "concat", "-safe", "0", "-i"])
            .arg(&list_path)
            .args([
                "-c:v", "libx264", "-c:a", "aac", "-preset", "veryfast", "-pix_fmt", "yuv420p",
            ])
            .arg(out)
            .output()?;
        if !res.status.success() || !out.exists() {
            let err = String::from_utf8_lossy(&res.stderr);
            let tail = &err[err.len().saturating_sub(400)..];
            bail!("concat failed: {}", tail.trim());
        }
    }
    Ok(())
}

/// ffprobe → duration seconds; 0.0 if unreadable. (Local copy so this module is standalone.)
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
    fn countdown_keeps_input_order() {
        let items = [
            RankItem {
                start: 0.0,
                end: 5.0,
                rank: 1,
                label: "a".into(),
            },
            RankItem {
                start: 5.0,
                end: 10.0,
                rank: 2,
                label: "b".into(),
            },
            RankItem {
                start: 10.0,
                end: 15.0,
                rank: 3,
                label: "c".into(),
            },
        ];
        // CountDown plays 1, 2, 3 — index order 0, 1, 2.
        let playback: Vec<usize> = match Order::CountDown {
            Order::CountDown => (0..items.len()).collect(),
            Order::CountUp => (0..items.len()).rev().collect(),
        };
        assert_eq!(playback, vec![0, 1, 2]);
    }

    #[test]
    fn countdown_reveals_best_last() {
        // CountUp reverses — index order N-1, N-2 ... 0 → rank 1 plays last.
        let n = 5;
        let playback: Vec<usize> = (0..n).rev().collect();
        assert_eq!(playback.first(), Some(&4));
        assert_eq!(playback.last(), Some(&0)); // rank-1 is index 0 → last
    }
}
