//! Stage 2.2 — FACE-PAN REFRAME. Parity port of `src/ycp/reframe.py`.
//!
//! Crop a 9:16 window that follows the speaker. The pure crop-x expression builder
//! (`crop_x_expr`, `smooth`) is cross-checked byte-for-byte against the Python (`ycp crop-x`).
//! `probe_dims` + `reframe` shell out to ffprobe/ffmpeg exactly as the Python does.
//!
//! PARITY GAP — `face_track`: the Python uses OpenCV (cv2) Haar cascades to find the speaker.
//! There is no pure-Rust OpenCV without a heavy system dependency, so `face_track` here returns
//! an empty track — the SAME behavior as the Python's `except ImportError: return [], 0` path
//! (cv2 absent). `reframe` then takes the static center crop, which is byte-identical to the
//! Python center-crop fallback. Wiring real OpenCV face panning is a deliberate later call.
#![allow(dead_code)] // probe/reframe wired by the clip/autopilot rows

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Result};

use crate::util::round_to;

pub const TARGET_W: i64 = 1080;
pub const TARGET_H: i64 = 1920;

/// Probe (width, height) via ffprobe; (0, 0) if unreadable. Mirrors `_probe_dims`.
fn probe_dims(video: &Path) -> (i64, i64) {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0:s=x",
        ])
        .arg(video)
        .output();
    let stdout = match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => return (0, 0),
    };
    let mut it = stdout.split('x');
    match (
        it.next().and_then(|s| s.trim().parse().ok()),
        it.next().and_then(|s| s.trim().parse().ok()),
    ) {
        (Some(w), Some(h)) => (w, h),
        _ => (0, 0),
    }
}

/// OpenCV face track — NOT ported (no pure-Rust cv2). Returns ([], 0), matching the Python
/// `except ImportError` path so callers fall back to a center crop. Mirrors `face_track`'s
/// cv2-absent branch.
pub fn face_track(_video: &Path) -> (Vec<(f64, f64)>, usize) {
    (Vec::new(), 0)
}

/// Box-mean smoothing with a +/- `win` window. Mirrors `_smooth`. Pure.
pub fn smooth(vals: &[f64], win: usize) -> Vec<f64> {
    (0..vals.len())
        .map(|i| {
            let a = i.saturating_sub(win);
            let b = (i + win + 1).min(vals.len());
            vals[a..b].iter().sum::<f64>() / (b - a) as f64
        })
        .collect()
}

/// Piecewise ffmpeg crop-x expression following the (smoothed) face track; hard-cuts only when
/// the target shifts more than `jump` of the frame width. None if the track is empty. Pure.
/// Mirrors `crop_x_expr`.
pub fn crop_x_expr(track: &[(f64, f64)], scaled_w: i64, crop_w: i64, jump: f64) -> Option<String> {
    if track.is_empty() {
        return None;
    }
    let hi = (scaled_w - crop_w).max(0);
    let to_x = |frac: f64| -> i64 {
        // Python: int(max(0, min(hi, frac*scaled_w - crop_w/2))) — clamp then truncate (>=0).
        let v = (frac * scaled_w as f64 - crop_w as f64 / 2.0)
            .min(hi as f64)
            .max(0.0);
        v as i64
    };
    let sm = smooth(&track.iter().map(|(_, f)| *f).collect::<Vec<_>>(), 5);
    let mut segs: Vec<(f64, i64)> = Vec::new();
    let mut cur = to_x(sm[0]);
    let thresh = jump * scaled_w as f64;
    for ((t, _), f) in track.iter().zip(sm.iter()) {
        let x = to_x(*f);
        if (x - cur).abs() as f64 > thresh {
            segs.push((*t, cur));
            cur = x;
        }
    }
    segs.push((f64::INFINITY, cur));
    if segs.len() == 1 {
        return Some(segs[0].1.to_string());
    }
    let mut expr = segs[segs.len() - 1].1.to_string();
    for (end_t, x) in segs[..segs.len() - 1].iter().rev() {
        expr = format!("if(lt(t,{end_t:.3}),{x},{expr})");
    }
    Some(expr)
}

/// Scale to target height and crop a 9:16 window — face-following or static center.
/// Mirrors `reframe`. Raises on ffmpeg failure.
pub fn reframe(
    video: &Path,
    out_path: &Path,
    workdir: &Path,
    mode: &str,
    size: (i64, i64),
) -> Result<std::path::PathBuf> {
    let (w, h) = size;
    let (sw, sh) = probe_dims(video);
    // round(sw * h / sh / 2) * 2 if sh else 0
    let scaled_w = if sh != 0 {
        (round_to(sw as f64 * h as f64 / sh as f64 / 2.0, 0) * 2.0) as i64
    } else {
        0
    };
    let mut expr: Option<String> = None;
    if mode == "face" && scaled_w > w {
        let (track, sampled) = face_track(video);
        if sampled > 0 && track.len() as f64 >= 0.45 * sampled as f64 {
            expr = crop_x_expr(&track, scaled_w, w, 0.05);
        }
    }
    let center_vf = format!("scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}");
    let face_vf = match expr {
        Some(e) if scaled_w > w => Some(format!("scale=-2:{h},crop={w}:{h}:x='{e}':y=0")),
        _ => None,
    };

    let tmp = workdir.join("reframed.mp4");
    let mut last_err = String::new();
    let vfs: Vec<String> = match face_vf {
        Some(f) => vec![f, center_vf],
        None => vec![center_vf],
    };
    for vf in vfs {
        let out = Command::new("ffmpeg")
            .args(["-y", "-i"])
            .arg(video)
            .args([
                "-vf", &vf, "-c:v", "libx264", "-c:a", "copy", "-preset", "veryfast", "-pix_fmt",
                "yuv420p",
            ])
            .arg(&tmp)
            .output()?;
        if out.status.success() && tmp.exists() {
            if let Some(d) = out_path.parent() {
                std::fs::create_dir_all(d).ok();
            }
            std::fs::rename(&tmp, out_path).or_else(|_| {
                std::fs::copy(&tmp, out_path)
                    .map(|_| ())
                    .and_then(|_| std::fs::remove_file(&tmp))
            })?;
            return Ok(out_path.to_path_buf());
        }
        let err = String::from_utf8_lossy(&out.stderr);
        let e = err.trim();
        last_err = e[e.len().saturating_sub(400)..].to_string();
    }
    bail!("reframe failed: {last_err}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_track_returns_none() {
        assert_eq!(crop_x_expr(&[], 3413, TARGET_W, 0.05), None);
    }

    #[test]
    fn static_track_centers_and_clamps() {
        let track: Vec<(f64, f64)> = (0..40).map(|i| (i as f64 * 0.3, 0.5)).collect();
        let expected = (0.5 * 3413.0 - 540.0) as i64; // int(0.5*3413 - 1080/2)
        assert_eq!(
            crop_x_expr(&track, 3413, 1080, 0.05),
            Some(expected.to_string())
        );
    }

    #[test]
    fn track_clamps_to_frame_edges() {
        let left: Vec<(f64, f64)> = (0..40).map(|i| (i as f64 * 0.3, 0.0)).collect();
        let right: Vec<(f64, f64)> = (0..40).map(|i| (i as f64 * 0.3, 1.0)).collect();
        assert_eq!(
            crop_x_expr(&left, 3413, TARGET_W, 0.05),
            Some("0".to_string())
        );
        assert_eq!(
            crop_x_expr(&right, 3413, TARGET_W, 0.05),
            Some((3413 - 1080).to_string())
        );
    }

    #[test]
    fn sustained_pan_builds_conditional() {
        let mut track: Vec<(f64, f64)> = (0..20).map(|i| (i as f64 * 0.3, 0.2)).collect();
        track.extend((0..20).map(|i| (6.0 + i as f64 * 0.3, 0.85)));
        let expr = crop_x_expr(&track, 3413, TARGET_W, 0.05).unwrap();
        assert!(expr.starts_with("if(lt(t"));
    }

    #[test]
    fn smooth_box_mean() {
        // window +/-1: middle element averages its 3 neighbors.
        assert_eq!(smooth(&[0.0, 3.0, 0.0], 1), vec![1.5, 1.0, 1.5]);
    }
}
