//! Opus-style word-by-word caption *chunking* — the pure, unit-tested half of
//! `src/ycp/captions.py`. Groups whisper segments into 1-3 word chunks with
//! approximate per-word timing.
//!
//! The Pillow render + ffmpeg overlay half (`render_overlay`, `burn_captions`,
//! font fitting, the `captions:` creative knobs) is deliberately NOT here — it's
//! the separate "captions render" row in README.md (Pillow → image+cosmic-text).
// The render fns mirror Pillow's many-arg signatures 1:1 (font/size/pos/color/stroke); a params
// struct would only obscure the parity, so allow the arg count here.
#![allow(clippy::too_many_arguments)]
use std::path::{Path, PathBuf};
use std::process::Command;

use ab_glyph::{point, Font, FontVec, Glyph, PxScale, ScaleFont};
use anyhow::{bail, Context, Result};
use image::{Rgba, RgbaImage};

use crate::srt::Segment;
use crate::util::round_to;

pub const MAX_WORDS: usize = 3;
pub const MIN_DWELL: f64 = 0.4; // seconds a chunk stays on screen, minimum

/// One word with approximate [start, end] timing.
#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

/// A 1-3 word group shown together, the active word highlighted at render time.
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub start: f64,
    pub end: f64,
    pub words: Vec<Word>,
}

impl Chunk {
    /// Space-joined chunk text (mirrors the Python `text` property).
    pub fn text(&self) -> String {
        self.words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Distribute a segment's [start, end] evenly across its words (approx word timing).
pub fn split_words(seg: &Segment) -> Vec<Word> {
    let toks: Vec<&str> = seg.text.split_whitespace().collect();
    if toks.is_empty() {
        return Vec::new();
    }
    let span = (seg.end - seg.start).max(0.01);
    let step = span / toks.len() as f64;
    toks.iter()
        .enumerate()
        .map(|(i, t)| Word {
            text: (*t).to_string(),
            start: round_to(seg.start + i as f64 * step, 3),
            end: round_to(seg.start + (i + 1) as f64 * step, 3),
        })
        .collect()
}

/// Group words into <=max_words chunks, non-overlapping, each held >= min_dwell.
pub fn build_chunks(segments: &[Segment], max_words: usize, min_dwell: f64) -> Vec<Chunk> {
    let words: Vec<Word> = segments.iter().flat_map(split_words).collect();
    let mut chunks = Vec::new();
    let mut cursor = 0.0_f64;
    let mut i = 0;
    while i < words.len() {
        let grp = &words[i..(i + max_words).min(words.len())];
        let start = grp[0].start.max(cursor);
        let end = grp[grp.len() - 1].end.max(start + min_dwell);
        chunks.push(Chunk {
            start: round_to(start, 3),
            end: round_to(end, 3),
            words: grp.to_vec(),
        });
        cursor = end; // cursor tracks the UN-rounded end, like the Python.
        i += max_words;
    }
    chunks
}

// -- rendering (Pillow → ab_glyph + image) ------------------------------------
//
// Parity port of the render half of `src/ycp/captions.py`. The Python renders each
// frame's caption (1-3 words, active word highlighted) + the hook title to a
// transparent PNG with Pillow, then ffmpeg `overlay` composites onto the clip
// (this ffmpeg has no libass/drawtext). This reproduces that with ab_glyph (glyph
// rasterization) + image (RGBA PNG) — pure Rust, so the single binary still ships
// without a Python/Pillow env on target machines.
//
// NOT byte-identical to Pillow: different rasterizers, different stroker. Parity
// here is structural — same fonts, same layout math, same colors, same fat black
// outline, and a byte-identical FRAME SCHEDULE (frame count, which chunk/title is
// shown per frame), which is the deterministic part cross-checked via `ycp caprender`.

/// Heavy display fonts (opus look). First existing path wins. Mirrors FONT_CANDIDATES.
pub(crate) const FONT_CANDIDATES: [&str; 3] = [
    "/System/Library/Fonts/Supplemental/Arial Black.ttf",
    "/System/Library/Fonts/Supplemental/Impact.ttf",
    "/Library/Fonts/Arial Black.ttf",
];
pub const FPS: u32 = 15;
pub const SIZE: (u32, u32) = (1080, 1920);
const ACTIVE: Rgba<u8> = Rgba([255, 222, 0, 255]); // highlighted (current) word — yellow
pub(crate) const IDLE: Rgba<u8> = Rgba([255, 255, 255, 255]); // other words in the chunk — white
pub(crate) const OUTLINE: Rgba<u8> = Rgba([0, 0, 0, 255]); // fat black stroke for legibility
const CAPTION_CASE: &str = "lower";
const CAPTION_SIZE_PCT: f64 = 10.0; // caption height cap, as % of frame width
const HOOK_HOLD_SEC: f64 = 7.0; // hook/title stays on screen at least this long

/// Creative caption knobs from settings.yaml (captions:), with safe fallbacks.
/// Mirrors `_caption_cfg` — rendering must never hard-break on bad/missing config.
#[derive(Debug, Clone)]
pub struct CapCfg {
    pub case: String,
    pub size_pct: f64,
    pub hook_hold_sec: f64,
}

fn yaml_num(v: &serde_yaml::Value, key: &str) -> Option<f64> {
    let x = v.get(key)?;
    x.as_f64().or_else(|| x.as_i64().map(|i| i as f64))
}

pub fn caption_cfg(settings: Option<&serde_yaml::Value>) -> CapCfg {
    let c = settings.and_then(|s| s.get("captions"));
    let case = c
        .and_then(|c| c.get("case"))
        .and_then(|v| v.as_str())
        .unwrap_or(CAPTION_CASE)
        .to_lowercase();
    let size_pct = c
        .and_then(|c| yaml_num(c, "size_pct"))
        .unwrap_or(CAPTION_SIZE_PCT)
        / 100.0;
    let hook_hold_sec = c
        .and_then(|c| yaml_num(c, "hook_hold_sec"))
        .unwrap_or(HOOK_HOLD_SEC);
    CapCfg {
        case,
        size_pct,
        hook_hold_sec,
    }
}

/// Mirror `_case`.
fn case_str(s: &str, case: &str) -> String {
    if case == "lower" {
        s.to_lowercase()
    } else {
        s.to_uppercase()
    }
}

/// First existing TTF (font_path override, then FONT_CANDIDATES). ab_glyph fonts are
/// size-independent — the px size is applied at draw time, so we load once (unlike
/// Pillow, which reloads per size).
/// ponytail: no embedded fallback font. Pillow falls back to a tiny bitmap default; here a
/// missing TTF → blank caption frames (caller `clip.py` already ships a plain clip on caption
/// failure). The target machines have Arial Black, so this is academic. Add an embedded font
/// only if a fontless host ever needs legible captions.
pub(crate) fn load_font(font_path: Option<&str>) -> Option<FontVec> {
    let paths = font_path.into_iter().chain(FONT_CANDIDATES.iter().copied());
    for p in paths {
        if Path::new(p).is_file() {
            if let Ok(bytes) = std::fs::read(p) {
                if let Ok(f) = FontVec::try_from_vec(bytes) {
                    return Some(f);
                }
            }
        }
    }
    None
}

/// Advance-based text width + stroke padding, rounded to whole px (Pillow's textbbox
/// returns ints). Approximates `_text_width`.
pub(crate) fn text_width(font: &FontVec, text: &str, px: f32, stroke: u32) -> f32 {
    let sf = font.as_scaled(PxScale::from(px));
    let adv: f32 = text.chars().map(|c| sf.h_advance(font.glyph_id(c))).sum();
    (adv + 2.0 * stroke as f32).round()
}

/// Full font height + stroke padding (uses ascent/descent, like Pillow's "Ay" bbox).
/// Approximates `_text_height`.
pub(crate) fn text_height(font: &FontVec, px: f32, stroke: u32) -> f32 {
    let sf = font.as_scaled(PxScale::from(px));
    (sf.ascent() - sf.descent()).round() + 2.0 * stroke as f32
}

/// Largest px size (<= max_px, stepping by 4, floor 14) whose `text` fits max_w. Mirrors `_fit_font`.
pub(crate) fn fit_px(font: &FontVec, text: &str, max_px: f32, max_w: f32, stroke: u32) -> f32 {
    let mut size = max_px;
    while size > 14.0 {
        if text_width(font, text, size, stroke) <= max_w {
            return size;
        }
        size -= 4.0;
    }
    14.0
}

/// Integer disk offsets within `stroke` radius (the fat outline kernel). Built once per word.
fn disk(stroke: u32) -> Vec<(i32, i32)> {
    let s = stroke as i32;
    let mut v = Vec::with_capacity(((2 * s + 1) * (2 * s + 1)) as usize);
    for dy in -s..=s {
        for dx in -s..=s {
            if dx * dx + dy * dy <= s * s {
                v.push((dx, dy));
            }
        }
    }
    v
}

/// Straight (non-premultiplied) src-over of `color` at `coverage` onto a pixel.
fn blend(img: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>, coverage: f32) {
    if x < 0 || y < 0 || x >= img.width() as i32 || y >= img.height() as i32 {
        return;
    }
    let sa = coverage.clamp(0.0, 1.0) * (color[3] as f32 / 255.0);
    if sa <= 0.0 {
        return;
    }
    let (x, y) = (x as u32, y as u32);
    let dst = *img.get_pixel(x, y);
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        return;
    }
    let mut out = [0u8; 4];
    for i in 0..3 {
        let v = (color[i] as f32 * sa + dst[i] as f32 * da * (1.0 - sa)) / out_a;
        out[i] = v.round().clamp(0.0, 255.0) as u8;
    }
    out[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
    img.put_pixel(x, y, Rgba(out));
}

/// Draw one word at (x_left, y_top) anchor "la" (y = ascender top): fat black outline
/// pass, then the colored fill on top — mirroring Pillow's per-call stroke-then-fill.
/// ponytail: outline = stamp a stroke-radius disk per covered glyph pixel — O(ink_px · stroke²).
/// Fine for a background pipeline; if a frame ever renders slow, swap to a separable max-dilation
/// of a single glyph coverage mask.
pub(crate) fn draw_word(
    img: &mut RgbaImage,
    font: &FontVec,
    text: &str,
    px: f32,
    x_left: f32,
    y_top: f32,
    fill: Rgba<u8>,
    stroke: u32,
) {
    let scale = PxScale::from(px);
    let sf = font.as_scaled(scale);
    let baseline = y_top + sf.ascent();
    let kernel = disk(stroke);
    // Lay out glyphs once (advance-based pen), reuse for both passes.
    let mut pen = x_left + stroke as f32;
    let mut glyphs: Vec<Glyph> = Vec::new();
    for ch in text.chars() {
        let gid = font.glyph_id(ch);
        glyphs.push(gid.with_scale_and_position(scale, point(pen, baseline)));
        pen += sf.h_advance(gid);
    }
    // Pass 1 — fat black outline.
    for g in &glyphs {
        if let Some(o) = font.outline_glyph(g.clone()) {
            let b = o.px_bounds();
            let (ox, oy) = (b.min.x as i32, b.min.y as i32);
            o.draw(|gx, gy, c| {
                if c <= 0.0 {
                    return;
                }
                for (dx, dy) in &kernel {
                    blend(img, ox + gx as i32 + dx, oy + gy as i32 + dy, OUTLINE, c);
                }
            });
        }
    }
    // Pass 2 — colored fill on top.
    for g in &glyphs {
        if let Some(o) = font.outline_glyph(g.clone()) {
            let b = o.px_bounds();
            let (ox, oy) = (b.min.x as i32, b.min.y as i32);
            o.draw(|gx, gy, c| blend(img, ox + gx as i32, oy + gy as i32, fill, c));
        }
    }
}

/// Center a chunk's words at vertical `y`, highlight the active word at time `t`. Mirrors `_draw_chunk`.
fn draw_chunk(
    img: &mut RgbaImage,
    font: &FontVec,
    chunk: &Chunk,
    t: f64,
    w: u32,
    y: f32,
    max_px: f32,
    stroke: u32,
    case: &str,
) {
    let joined = case_str(&chunk.text(), case);
    let px = fit_px(font, &joined, max_px, w as f32 * 0.92, stroke);
    let gap = (w as f64 * 0.018) as i64 as f32; // int(w*0.018)
    let words: Vec<String> = chunk
        .words
        .iter()
        .map(|wd| case_str(&wd.text, case))
        .collect();
    let widths: Vec<f32> = words
        .iter()
        .map(|s| text_width(font, s, px, stroke))
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + gap * (words.len() as f32 - 1.0);
    let mut x = ((w as f32 - total) / 2.0).floor(); // (w - total)//2
    for ((wd, txt), ww) in chunk.words.iter().zip(words.iter()).zip(widths.iter()) {
        let active = wd.start <= t && t < wd.end;
        let fill = if active { ACTIVE } else { IDLE };
        draw_word(img, font, txt, px, x, y, fill, stroke);
        x += ww + gap;
    }
}

/// Word-wrap the hook title to `max_w` and draw centered lines from `y`. Mirrors `_draw_title`.
fn draw_title(
    img: &mut RgbaImage,
    font: &FontVec,
    text: &str,
    px: f32,
    max_w: f32,
    w: u32,
    y: f32,
    stroke: u32,
) {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let trial = if cur.is_empty() {
            word.to_string()
        } else {
            format!("{cur} {word}")
        };
        if cur.is_empty() || text_width(font, &trial, px, stroke) <= max_w {
            cur = trial;
        } else {
            lines.push(cur);
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    let lh = (text_height(font, px, stroke) * 1.15) as i64 as f32; // int(height * 1.15)
    for (i, line) in lines.iter().enumerate() {
        let lw = text_width(font, line, px, stroke);
        let lx = ((w as f32 - lw) / 2.0).floor();
        draw_word(img, font, line, px, lx, y + i as f32 * lh, IDLE, stroke);
    }
}

/// Render a transparent PNG sequence (00000.png ...) for the clip; return frame count.
/// Mirrors `render_overlay`. `settings` carries the `captions:` knobs (None → defaults).
pub fn render_overlay(
    chunks: &[Chunk],
    duration: f64,
    out_dir: &Path,
    title: Option<&str>,
    size: (u32, u32),
    fps: u32,
    font_path: Option<&str>,
    settings: Option<&serde_yaml::Value>,
) -> Result<u32> {
    std::fs::create_dir_all(out_dir).with_context(|| format!("mkdir {}", out_dir.display()))?;
    let cfg = caption_cfg(settings);
    let (w, h) = size;
    let stroke = (w / 135).max(6);
    let n_frames = ((duration * fps as f64).ceil().max(1.0)) as u32;
    let title_dur = cfg.hook_hold_sec;
    let cap_max = (w as f64 * cfg.size_pct) as i64 as f32; // int(w * size_pct)
    let title_size = (w as f64 * 0.072) as i64 as f32; // int(w * 0.072)
    let font = load_font(font_path);
    for f in 0..n_frames {
        let t = f as f64 / fps as f64;
        let mut img = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
        if let (Some(ttl), Some(fnt)) = (title, font.as_ref()) {
            if t < title_dur {
                let cased = case_str(ttl, &cfg.case);
                draw_title(
                    &mut img,
                    fnt,
                    &cased,
                    title_size,
                    w as f32 * 0.86,
                    w,
                    (h as f64 * 0.10) as i64 as f32,
                    stroke,
                );
            }
        }
        if let Some(fnt) = font.as_ref() {
            if let Some(ch) = chunks.iter().find(|c| c.start <= t && t < c.end) {
                draw_chunk(
                    &mut img,
                    fnt,
                    ch,
                    t,
                    w,
                    (h as f64 * 0.70) as i64 as f32,
                    cap_max,
                    stroke,
                    &cfg.case,
                );
            }
        }
        img.save(out_dir.join(format!("{f:05}.png")))
            .with_context(|| format!("write frame {f}"))?;
    }
    Ok(n_frames)
}

/// Clip duration via ffprobe; 0.0 if unreadable. Mirrors `_probe_duration`.
#[allow(dead_code)] // wired by the autopilot row
fn probe_duration(path: &Path) -> f64 {
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

/// Render caption frames and overlay them onto base_clip with ffmpeg (no libass needed).
/// Mirrors `burn_captions`.
#[allow(dead_code)] // wired by the autopilot row
pub fn burn_captions(
    base_clip: &Path,
    chunks: &[Chunk],
    out_path: &Path,
    workdir: &Path,
    title: Option<&str>,
    fps: u32,
    size: (u32, u32),
    font_path: Option<&str>,
    settings: Option<&serde_yaml::Value>,
) -> Result<PathBuf> {
    let probed = probe_duration(base_clip);
    let duration = if probed > 0.0 {
        probed
    } else {
        chunks.iter().map(|c| c.end).fold(0.0_f64, f64::max) + 0.5
    };
    let frames = workdir.join("capframes");
    render_overlay(
        chunks, duration, &frames, title, size, fps, font_path, settings,
    )?;
    let tmp_out = workdir.join("captioned.mp4");
    let frame_glob = frames.join("%05d.png");
    let out = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(base_clip)
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
        .arg(&tmp_out)
        .output()?;
    if !out.status.success() || !tmp_out.exists() {
        let err = String::from_utf8_lossy(&out.stderr);
        let tail = &err[err.len().saturating_sub(400)..];
        bail!("caption overlay failed: {}", tail.trim());
    }
    if let Some(p) = out_path.parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::rename(&tmp_out, out_path).or_else(|_| {
        std::fs::copy(&tmp_out, out_path)
            .map(|_| ())
            .and_then(|_| std::fs::remove_file(&tmp_out))
    })?;
    Ok(out_path.to_path_buf())
}

// ── rank-badge overlay (listicle format) ──────────────────────────────────────
//
// Used by `listicle::compile` — stamps a giant numeral ("1", "2", ...) at the left edge
// of every frame so a countdown/compilation video can show rank. Same opus font + fat
// black outline as the captions/title, but filled with the brand magenta so the badge
// pops against arbitrary footage. PNG sequence + ffmpeg overlay, identical to render_overlay.

/// Brand magenta fill for the rank numeral (matches the Rising Tides palette).
const RANK_FILL: Rgba<u8> = Rgba([225, 0, 195, 255]);

/// Render a transparent PNG sequence (00000.png ...) with a giant `rank` numeral stamped
/// at the left-center of the frame. Returns the frame count. Mirrors `render_overlay`'s
/// shape so the two sequences composite cleanly at the same fps/size.
pub(crate) fn render_rank_overlay(
    rank: usize,
    duration: f64,
    out_dir: &Path,
    size: (u32, u32),
    fps: u32,
    font_path: Option<&str>,
) -> Result<u32> {
    std::fs::create_dir_all(out_dir).with_context(|| format!("mkdir {}", out_dir.display()))?;
    let (w, h) = size;
    let stroke = (w / 135).max(6);
    let n_frames = ((duration * fps as f64).ceil().max(1.0)) as u32;
    let font = load_font(font_path);
    let numeral = rank.to_string();
    // Half-frame-tall numeral, fit to ~32% of frame width so it never crowds the captions.
    let max_px = h as f32 * 0.5;
    let max_w = w as f32 * 0.32;
    let px = font
        .as_ref()
        .map_or(max_px, |f| fit_px(f, &numeral, max_px, max_w, stroke));

    for f in 0..n_frames {
        let mut img = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
        if let Some(fnt) = font.as_ref() {
            let tw = text_width(fnt, &numeral, px, stroke);
            let th = text_height(fnt, px, stroke);
            // Left-edge anchor with breathing room; vertically centered.
            let x_left = (w as f32 * 0.06).floor();
            let y_top = ((h as f32 - th) / 2.0).floor();
            // Stamp a soft glow disk behind the numeral for extra pop on busy footage.
            let _ = tw; // width computed for centering if we ever want it; left-anchor for now
            draw_word(
                &mut img, fnt, &numeral, px, x_left, y_top, RANK_FILL, stroke,
            );
        }
        img.save(out_dir.join(format!("{f:05}.png")))
            .with_context(|| format!("write rank frame {f}"))?;
    }
    Ok(n_frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_words_even_distribution() {
        let seg = Segment::new(0.0, 3.0, "one two three");
        let w = split_words(&seg);
        assert_eq!(w.len(), 3);
        assert_eq!(
            w[0],
            Word {
                text: "one".into(),
                start: 0.0,
                end: 1.0
            }
        );
        assert_eq!(
            w[1],
            Word {
                text: "two".into(),
                start: 1.0,
                end: 2.0
            }
        );
        assert_eq!(
            w[2],
            Word {
                text: "three".into(),
                start: 2.0,
                end: 3.0
            }
        );
        // empty text → no words.
        assert!(split_words(&Segment::new(0.0, 1.0, "")).is_empty());
    }

    #[test]
    fn build_chunks_groups_and_enforces_dwell() {
        // 4 words over 4s → two chunks of 3 + 1; second chunk held >= MIN_DWELL.
        let segs = vec![Segment::new(0.0, 4.0, "a b c d")];
        let chunks = build_chunks(&segs, MAX_WORDS, MIN_DWELL);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text(), "a b c");
        assert_eq!(chunks[1].text(), "d");
        assert_eq!(chunks[0].start, 0.0);
        assert_eq!(chunks[0].end, 3.0);
        // last word spans 3.0..4.0, already > min_dwell from start.
        assert_eq!(chunks[1].start, 3.0);
        assert_eq!(chunks[1].end, 4.0);
    }

    #[test]
    fn build_chunks_min_dwell_extends_short_chunk() {
        // a single very short word must still be held MIN_DWELL.
        let segs = vec![Segment::new(0.0, 0.1, "hi")];
        let chunks = build_chunks(&segs, MAX_WORDS, MIN_DWELL);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 0.0);
        assert_eq!(chunks[0].end, 0.4); // extended to min_dwell
    }
}
