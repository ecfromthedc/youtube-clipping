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

from .config import ROOT
from .srt import Segment

# Heavy display fonts (opus look). First existing path wins; falls back to Pillow default.
FONT_CANDIDATES = (
    "/System/Library/Fonts/Supplemental/Arial Black.ttf",
    "/System/Library/Fonts/Supplemental/Impact.ttf",
    "/Library/Fonts/Arial Black.ttf",
)
# Hook title font — brand display face (Poppins), bundled as TTF (Pillow can't read woff2).
# Falls back to the heavy display fonts above, then Pillow default.
HELVETICA_TTC = "/System/Library/Fonts/Helvetica.ttc"   # macOS Helvetica collection (index 1 = Bold)
HOOK_FONT_CANDIDATES = (
    HELVETICA_TTC,                                          # clean Helvetica Bold (operator's pick)
    str(ROOT / "assets" / "fonts" / "Poppins-Bold.ttf"),
) + FONT_CANDIDATES
# Captions use the same Helvetica Bold for a cohesive look.
CAPTION_FONT_CANDIDATES = (HELVETICA_TTC,
                           str(ROOT / "assets" / "fonts" / "Poppins-Bold.ttf")) + FONT_CANDIDATES
MAX_WORDS = 3
MIN_DWELL = 0.4          # seconds a chunk stays on screen, minimum
FPS = 15
SIZE = (1080, 1920)
ACTIVE = (255, 222, 0, 255)     # highlighted (current) word — yellow
IDLE = (255, 255, 255, 255)     # other words in the chunk — white
OUTLINE = (0, 0, 0, 255)        # fat black stroke for legibility over any footage
# Hook title: near-black text on a clean white highlight box, dead-centre. High-contrast,
# pops on mute over any footage, on-brand (brand near-white #FAFCFF + near-black #060606).
HOOK_BOX = (250, 252, 255, 255)   # #FAFCFF near-white — the hook bar
HOOK_TEXT = (6, 6, 6, 255)        # #060606 near-black — hook text on the bar
HOOK_POS = 0.34                   # hook block centre at ~2/3 height (upper third), fraction from top
HOOK_MAX_LINES = 2                # hooks shrink to fit this many lines — keeps them punchy, never long

# Creative knobs — tunable in settings.yaml under `captions:`. NOTE on size: video
# captions scale to FRAME WIDTH, not document point size. size_pct=10 → caption height
# capped at 10% of a 1080-wide frame (~108px, the Opus look). 10px would be invisible.
CAPTION_CASE = "lower"          # subtitle case: lower | upper
CAPTION_SIZE_PCT = 10.0         # caption height cap, as % of frame width
HOOK_HOLD_SEC = 7.0             # hook/title stays on screen at least this long


def _hex_rgba(s: str, fallback: tuple[int, int, int, int]) -> tuple[int, int, int, int]:
    """'#E100C3' / 'E100C3' -> (225,0,195,255). Bad input -> fallback (render never breaks)."""
    try:
        h = str(s).lstrip("#")
        return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), 255)
    except (ValueError, IndexError, TypeError):
        return fallback


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
        # Hook title look. hook_box false → outline-only (old look). Defaults: white box, black text.
        "hook_box": bool(c.get("hook_box", True)),
        "hook_box_color": _hex_rgba(c.get("hook_box_color", "#FAFCFF"), HOOK_BOX),
        "hook_text_color": _hex_rgba(c.get("hook_text_color", "#060606"), HOOK_TEXT),
        "hook_pos": float(c.get("hook_pos", HOOK_POS)),
        "hook_max_lines": int(c.get("hook_max_lines", HOOK_MAX_LINES)),
    }


def _case(s: str, case: str) -> str:
    return s.lower() if case == "lower" else s.upper()


CAP_ZONE_TOP = 0.62      # captions live below this; keep the hook above it
HOOK_MIN_BAND = 0.16     # a clear band must be at least this tall to hold a 2-line hook


def _clear_hook_pos(band: tuple[float, float] | None, default: float) -> float:
    """Hook centre placed in the biggest clear vertical band: above the face if there's room,
    else below it (always above the caption zone). Pure — `band` is normalized (y_top, y_bottom)
    of the face, or None. Falls back to `default`."""
    if not band:
        return default
    y0, y1 = band
    above_h, below_h = y0 - 0.05, CAP_ZONE_TOP - y1
    if above_h >= below_h and above_h >= HOOK_MIN_BAND:
        pos = (0.05 + y0) / 2          # centre the hook in the gap above the face
    elif below_h >= HOOK_MIN_BAND:
        pos = (y1 + CAP_ZONE_TOP) / 2  # tuck it between face and captions
    else:
        return default                 # no clear band big enough — leave it at the default
    return max(0.12, min(0.6, pos))


def _adaptive_hook_pos(base_clip: Path, default: float) -> float:
    """Read the clip's first frame, find the face, return a hook position that clears it."""
    try:
        from . import reframe
        return _clear_hook_pos(reframe.face_band(base_clip, t=0.3), default)
    except Exception:  # noqa: BLE001 — placement is best-effort; never break the render
        return default


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


def _breaks_after(text: str, group_len: int) -> bool:
    """Whether a chunk should flush after this word — break on natural syntax boundaries so
    captions read as phrases, not arbitrary 3-word slices. Strong terminal/clause punctuation
    (.!?;:) always breaks; a comma breaks only once the chunk has >=2 words (no orphan 'well,')."""
    last = text.rstrip("\"')]}")[-1:]   # ignore trailing quotes/brackets
    if last in ".!?;:":
        return True
    return last == "," and group_len >= 2


def build_chunks(segments: list[Segment], max_words: int = MAX_WORDS,
                 min_dwell: float = MIN_DWELL) -> list[Chunk]:
    """Group words into <=max_words chunks that break on phrase/clause boundaries (punctuation),
    non-overlapping, each held >= min_dwell. Falls back to a hard max_words cap when a run of
    words has no punctuation, so captions never exceed max_words on screen."""
    words = [w for seg in segments for w in split_words(seg)]
    chunks: list[Chunk] = []
    cursor = 0.0
    grp: list[Word] = []
    for w in words:
        grp.append(w)
        if len(grp) >= max_words or _breaks_after(w.text, len(grp)):
            start = max(grp[0].start, cursor)
            end = max(grp[-1].end, start + min_dwell)
            chunks.append(Chunk(round(start, 3), round(end, 3), tuple(grp)))
            cursor = end
            grp = []
    if grp:
        start = max(grp[0].start, cursor)
        end = max(grp[-1].end, start + min_dwell)
        chunks.append(Chunk(round(start, 3), round(end, 3), tuple(grp)))
    return chunks


# -- rendering (Pillow) -------------------------------------------------------

def _load_font(size: int, font_path: str | None):
    from PIL import ImageFont
    for p in ([font_path] if font_path else []) + list(FONT_CANDIDATES):
        if p and Path(p).exists():
            if p.endswith(".ttc"):                    # font collection (e.g. Helvetica): index 1 = Bold
                try:
                    return ImageFont.truetype(p, size, index=1)
                except Exception:  # noqa: BLE001
                    pass
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()


def _hook_font_path() -> str | None:
    """First existing hook font (Helvetica Bold, then Poppins/display fallbacks). None → default."""
    for p in HOOK_FONT_CANDIDATES:
        if Path(p).exists():
            return p
    return None


def _caption_font_path() -> str | None:
    """First existing caption font (Helvetica Bold, then fallbacks)."""
    for p in CAPTION_FONT_CANDIDATES:
        if Path(p).exists():
            return p
    return None


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
        # Per-word focus: the word being spoken at `t` is white, the rest stay yellow.
        focused = word.start <= t < word.end
        draw.text((x, y), _case(word.text, case), font=font, anchor="la",
                  fill=IDLE if focused else ACTIVE, stroke_width=stroke, stroke_fill=OUTLINE)
        x += ww + gap


def _wrap_lines(draw, text: str, font, max_w: int, stroke: int) -> list[str]:
    """Greedy word-wrap `text` to lines that fit within max_w."""
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
    return lines


def _draw_title(draw, text: str, size: int, max_w: int, w: int, h: int,
                font_path: str | None, stroke: int, *, box: bool = True,
                box_color=HOOK_BOX, text_color=HOOK_TEXT, pos_frac: float = HOOK_POS,
                max_lines: int = HOOK_MAX_LINES) -> None:
    """Hook title: each wrapped line on its own highlight box, the whole block centred vertically
    at `pos_frac`. Shrinks the font until the text fits in `max_lines` (keeps hooks punchy, never
    long). box=False → outline-only text (the old look)."""
    # Boxed hook: NO outline — the white box carries the contrast, so a stroke just makes the
    # text look chunky/heavy at this size. Outline-only mode (no box) still needs the fat stroke.
    tstroke = 0 if box else stroke
    # Shrink-to-fit: find the largest size (down to 62% of target) that fits in max_lines.
    font = _load_font(size, font_path)
    lines = _wrap_lines(draw, text, font, max_w, tstroke)
    sz = size
    while len(lines) > max_lines and sz > int(size * 0.62):
        sz -= 4
        font = _load_font(sz, font_path)
        lines = _wrap_lines(draw, text, font, max_w, tstroke)
    size = sz
    # Box padding gives the text air; a thin stroke keeps edges crisp on the bar (the box,
    # not a fat outline, is now what carries contrast).
    pad_x, pad_y = int(size * 0.34), int(size * 0.16)
    gap = int(size * 0.18)                       # vertical gap between stacked bars
    # Per-line glyph box (tight) so each bar hugs its text.
    boxes = [draw.textbbox((0, 0), ln, font=font, stroke_width=tstroke) for ln in lines]
    line_hs = [b[3] - b[1] for b in boxes]
    block_h = sum(lh + 2 * pad_y for lh in line_hs) + gap * (len(lines) - 1)
    y = int(h * pos_frac) - block_h // 2
    cx = w // 2
    for ln, bb, lh in zip(lines, boxes, line_hs):
        lw = bb[2] - bb[0]
        if box:
            x0, x1 = cx - lw // 2 - pad_x, cx + lw // 2 + pad_x
            y0, y1 = y, y + lh + 2 * pad_y
            draw.rounded_rectangle([x0, y0, x1, y1], radius=int(size * 0.14), fill=box_color)
        # anchor "la" draws from the ascender; offset by the bbox top so glyphs sit in the bar.
        draw.text((cx - lw // 2 - bb[0], y + pad_y - bb[1]), ln, font=font, anchor="la",
                  fill=text_color, stroke_width=tstroke, stroke_fill=OUTLINE)
        y += lh + 2 * pad_y + gap


def render_overlay(chunks: list[Chunk], duration: float, out_dir: Path, *,
                   title: str | None = None, size: tuple[int, int] = SIZE,
                   fps: int = FPS, font_path: str | None = None,
                   hook_pos: float | None = None) -> int:
    """Render a transparent PNG sequence (00000.png ...) for the clip; return frame count."""
    from PIL import Image, ImageDraw
    out_dir.mkdir(parents=True, exist_ok=True)
    cfg = _caption_cfg()
    w, h = size
    stroke = max(6, w // 135)
    n_frames = max(1, math.ceil(duration * fps))
    title_dur = cfg["hook_hold_sec"]
    pos_frac = cfg["hook_pos"] if hook_pos is None else hook_pos   # adaptive override from caller
    cap_max = int(w * cfg["size_pct"])
    cap_stroke = max(3, w // 300)              # thin outline on captions — they're not in a box
    title_size = int(w * 0.085)                # dead-centre hero hook
    hook_font = _hook_font_path()              # Helvetica Bold (falls back to Poppins/display)
    cap_font = _caption_font_path()            # Helvetica Bold captions
    for f in range(n_frames):
        t = f / fps
        img = Image.new("RGBA", size, (0, 0, 0, 0))
        d = ImageDraw.Draw(img)
        # The hook (top) is ALWAYS shown during its window — it's a hook, not a subtitle —
        # and coexists with our captions. RULE #1 (Eric) is about SUBTITLES: never our
        # word-by-word captions on top of a source that already has its own. That's handled
        # upstream by skipping caption-burn for captioned sources (chunks == []); here, an
        # empty chunk list simply means the hook renders with no second subtitle track.
        if title and t < title_dur:
            _draw_title(d, _case(title, cfg["case"]), title_size, int(w * 0.78), w, h,
                        hook_font, stroke, box=cfg["hook_box"], box_color=cfg["hook_box_color"],
                        text_color=cfg["hook_text_color"], pos_frac=pos_frac,
                        max_lines=cfg["hook_max_lines"])
        ch = next((c for c in chunks if c.start <= t < c.end), None)
        if ch:
            _draw_chunk(d, ch, t, w, int(h * 0.72), cap_max, cap_font, cap_stroke, cfg["case"])
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
    # Place the hook clear of the speaker's face (read from the first frame), not at a fixed height.
    hook_pos = _adaptive_hook_pos(base_clip, _caption_cfg()["hook_pos"]) if title else None
    render_overlay(chunks, duration, frames, title=title, size=size, fps=fps,
                   font_path=font_path, hook_pos=hook_pos)
    tmp_out = workdir / "captioned.mp4"
    cmd = [
        "ffmpeg", "-y", "-i", str(base_clip),
        "-framerate", str(fps), "-start_number", "0", "-i", str(frames / "%05d.png"),
        "-filter_complex", "[0:v][1:v]overlay=0:0:format=auto:eof_action=pass",
        "-c:v", "libx264", "-crf", "18", "-c:a", "copy", "-preset", "veryfast", "-pix_fmt", "yuv420p",
        str(tmp_out),
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    if proc.returncode != 0 or not tmp_out.exists():
        raise RuntimeError(f"caption overlay failed: {proc.stderr.strip()[-400:]}")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(str(tmp_out), str(out_path))
    return out_path
