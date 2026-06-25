"""Opus-style word-by-word captions via Pillow + ffmpeg `overlay`.

This ffmpeg has no libass/freetype (no `subtitles`/`drawtext` filters), so we render
each frame's caption (1-3 words, the active word highlighted) plus the hook title to a
transparent PNG sequence, then composite with the always-available `overlay` filter.
Self-contained — no libass, no network. The chunk logic is pure and unit-tested; the
rendering + ffmpeg overlay are thin wrappers verified on a real clip.

Word timing is approximate: each whisper segment's [start, end] is split evenly across
its words. Good enough for 1-3 word chunks; upgrade to true word-level whisper later.
"""
from __future__ import annotations

import math
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

from .srt import Segment

# Heavy display fonts (opus look). First existing path wins; falls back to Pillow default.
FONT_CANDIDATES = (
    "/System/Library/Fonts/Supplemental/Arial Black.ttf",
    "/System/Library/Fonts/Supplemental/Impact.ttf",
    "/Library/Fonts/Arial Black.ttf",
)
MAX_WORDS = 3
MIN_DWELL = 0.4          # seconds a chunk stays on screen, minimum
FPS = 15
SIZE = (1080, 1920)
ACTIVE = (255, 222, 0, 255)     # highlighted (current) word — yellow
IDLE = (255, 255, 255, 255)     # other words in the chunk — white
OUTLINE = (0, 0, 0, 255)        # fat black stroke for legibility over any footage

# Creative knobs — tunable in settings.yaml under `captions:`. NOTE on size: video
# captions scale to FRAME WIDTH, not document point size. size_pct=10 → caption height
# capped at 10% of a 1080-wide frame (~108px, the Opus look). 10px would be invisible.
CAPTION_CASE = "lower"          # subtitle case: lower | upper
CAPTION_SIZE_PCT = 10.0         # caption height cap, as % of frame width
HOOK_HOLD_SEC = 7.0             # hook/title stays on screen at least this long


def _caption_cfg() -> dict:
    """Creative caption knobs from settings.yaml (captions:), with safe fallbacks.
    Rendering must never hard-break on bad/missing config."""
    try:
        from .config import settings
        c = settings().get("captions") or {}
    except Exception:  # noqa: BLE001
        c = {}
    return {
        "case": str(c.get("case", CAPTION_CASE)).lower(),
        "size_pct": float(c.get("size_pct", CAPTION_SIZE_PCT)) / 100.0,
        "hook_hold_sec": float(c.get("hook_hold_sec", HOOK_HOLD_SEC)),
    }


def _case(s: str, case: str) -> str:
    return s.lower() if case == "lower" else s.upper()


@dataclass(frozen=True)
class Word:
    text: str
    start: float
    end: float


@dataclass(frozen=True)
class Chunk:
    start: float
    end: float
    words: tuple[Word, ...]

    @property
    def text(self) -> str:
        return " ".join(w.text for w in self.words)


def split_words(seg: Segment) -> list[Word]:
    """Distribute a segment's [start, end] evenly across its words (approx word timing)."""
    toks = seg.text.split()
    if not toks:
        return []
    span = max(seg.end - seg.start, 0.01)
    step = span / len(toks)
    return [
        Word(t, round(seg.start + i * step, 3), round(seg.start + (i + 1) * step, 3))
        for i, t in enumerate(toks)
    ]


def build_chunks(segments: list[Segment], max_words: int = MAX_WORDS,
                 min_dwell: float = MIN_DWELL) -> list[Chunk]:
    """Group words into <=max_words chunks, non-overlapping, each held >= min_dwell."""
    words = [w for seg in segments for w in split_words(seg)]
    chunks: list[Chunk] = []
    cursor = 0.0
    for i in range(0, len(words), max_words):
        grp = words[i:i + max_words]
        start = max(grp[0].start, cursor)
        end = max(grp[-1].end, start + min_dwell)
        chunks.append(Chunk(round(start, 3), round(end, 3), tuple(grp)))
        cursor = end
    return chunks


# -- rendering (Pillow) -------------------------------------------------------

def _load_font(size: int, font_path: str | None):
    from PIL import ImageFont
    for p in ([font_path] if font_path else []) + list(FONT_CANDIDATES):
        if p and Path(p).exists():
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()


def _text_width(draw, text: str, font, stroke: int) -> int:
    b = draw.textbbox((0, 0), text, font=font, stroke_width=stroke)
    return b[2] - b[0]


def _text_height(draw, text: str, font, stroke: int) -> int:
    b = draw.textbbox((0, 0), text, font=font, stroke_width=stroke)
    return b[3] - b[1]


def _fit_font(draw, text: str, max_size: int, max_w: int, font_path: str | None, stroke: int):
    """Largest font (<= max_size) whose `text` fits within max_w."""
    size = max_size
    while size > 14:
        font = _load_font(size, font_path)
        if _text_width(draw, text, font, stroke) <= max_w:
            return font
        size -= 4
    return _load_font(14, font_path)


def _draw_chunk(draw, chunk: Chunk, t: float, w: int, y: int, max_size: int,
                font_path: str | None, stroke: int, case: str = "upper") -> None:
    font = _fit_font(draw, _case(chunk.text, case), max_size, int(w * 0.92), font_path, stroke)
    gap = int(w * 0.018)
    widths = [_text_width(draw, _case(word.text, case), font, stroke) for word in chunk.words]
    total = sum(widths) + gap * (len(chunk.words) - 1)
    x = (w - total) // 2
    for word, ww in zip(chunk.words, widths):
        active = word.start <= t < word.end
        draw.text((x, y), _case(word.text, case), font=font, anchor="la",
                  fill=ACTIVE if active else IDLE, stroke_width=stroke, stroke_fill=OUTLINE)
        x += ww + gap


def _draw_title(draw, text: str, size: int, max_w: int, w: int, y: int,
                font_path: str | None, stroke: int) -> None:
    font = _load_font(size, font_path)
    lines: list[str] = []
    cur = ""
    for word in text.split():
        trial = f"{cur} {word}".strip()
        if not cur or _text_width(draw, trial, font, stroke) <= max_w:
            cur = trial
        else:
            lines.append(cur)
            cur = word
    if cur:
        lines.append(cur)
    lh = int(_text_height(draw, "Ay", font, stroke) * 1.15)
    for i, line in enumerate(lines):
        lw = _text_width(draw, line, font, stroke)
        draw.text(((w - lw) // 2, y + i * lh), line, font=font, anchor="la",
                  fill=IDLE, stroke_width=stroke, stroke_fill=OUTLINE)


def render_overlay(chunks: list[Chunk], duration: float, out_dir: Path, *,
                   title: str | None = None, size: tuple[int, int] = SIZE,
                   fps: int = FPS, font_path: str | None = None) -> int:
    """Render a transparent PNG sequence (00000.png ...) for the clip; return frame count."""
    from PIL import Image, ImageDraw
    out_dir.mkdir(parents=True, exist_ok=True)
    cfg = _caption_cfg()
    w, h = size
    stroke = max(6, w // 135)
    n_frames = max(1, math.ceil(duration * fps))
    title_dur = cfg["hook_hold_sec"]
    cap_max = int(w * cfg["size_pct"])
    title_size = int(w * 0.072)
    for f in range(n_frames):
        t = f / fps
        img = Image.new("RGBA", size, (0, 0, 0, 0))
        d = ImageDraw.Draw(img)
        if title and t < title_dur:
            # Hook + subtitles share the `case` knob (lower); the hook stays BIG via title_size.
            _draw_title(d, _case(title, cfg["case"]), title_size, int(w * 0.86), w, int(h * 0.10),
                        font_path, stroke)
        ch = next((c for c in chunks if c.start <= t < c.end), None)
        if ch:
            _draw_chunk(d, ch, t, w, int(h * 0.70), cap_max, font_path, stroke, cfg["case"])
        img.save(out_dir / f"{f:05d}.png")
    return n_frames


# -- composite (ffmpeg overlay -- no libass) ----------------------------------

def _probe_duration(path: Path) -> float:
    try:
        out = subprocess.run(
            ["ffprobe", "-v", "error", "-show_entries", "format=duration",
             "-of", "csv=p=0", str(path)],
            capture_output=True, text=True, timeout=60).stdout.strip()
        return float(out)
    except (OSError, subprocess.SubprocessError, ValueError):
        return 0.0


def burn_captions(base_clip: Path, chunks: list[Chunk], out_path: Path, workdir: Path, *,
                  title: str | None = None, fps: int = FPS,
                  size: tuple[int, int] = SIZE, font_path: str | None = None) -> Path:
    """Render caption frames and overlay them onto base_clip with ffmpeg (no libass needed)."""
    duration = _probe_duration(base_clip) or (max((c.end for c in chunks), default=0.0) + 0.5)
    frames = workdir / "capframes"
    render_overlay(chunks, duration, frames, title=title, size=size, fps=fps, font_path=font_path)
    tmp_out = workdir / "captioned.mp4"
    cmd = [
        "ffmpeg", "-y", "-i", str(base_clip),
        "-framerate", str(fps), "-start_number", "0", "-i", str(frames / "%05d.png"),
        "-filter_complex", "[0:v][1:v]overlay=0:0:format=auto:eof_action=pass",
        "-c:v", "libx264", "-c:a", "copy", "-preset", "veryfast", "-pix_fmt", "yuv420p",
        str(tmp_out),
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    if proc.returncode != 0 or not tmp_out.exists():
        raise RuntimeError(f"caption overlay failed: {proc.stderr.strip()[-400:]}")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(str(tmp_out), str(out_path))
    return out_path
