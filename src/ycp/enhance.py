"""Owned Ssemble-parity enhancements — pure ffmpeg, no app, no ceiling.

Replaces Ssemble's "Hook Title & CTA" and "Game Video" features with local ffmpeg.
The filter/command *builders* are pure functions (unit-tested without running
ffmpeg); the thin `apply_*` wrappers execute them. Fonts default to a macOS system
font so drawtext works without fontconfig setup.

See SSEMBLE-PARITY.md for the full map.
"""
from __future__ import annotations

import subprocess
from pathlib import Path

# A font that exists on stock macOS; override per call if needed.
DEFAULT_FONT = "/System/Library/Fonts/Supplemental/Arial Bold.ttf"


def escape_drawtext(text: str) -> str:
    """Escape a string for ffmpeg drawtext (colons, quotes, backslashes, %)."""
    return (text.replace("\\", "\\\\").replace(":", r"\:")
            .replace("'", r"\'").replace("%", r"\%"))


def title_filter(text: str, font: str = DEFAULT_FONT, fontsize: int = 56) -> str:
    """Top hook-title banner, shown for the whole clip."""
    t = escape_drawtext(text)
    return (f"drawtext=fontfile='{font}':text='{t}':fontcolor=white:fontsize={fontsize}:"
            f"box=1:boxcolor=black@0.55:boxborderw=18:x=(w-text_w)/2:y=90:line_spacing=8")


def cta_filter(text: str, start: float, end: float, font: str = DEFAULT_FONT,
               fontsize: int = 48) -> str:
    """Timed bottom CTA banner (e.g. 'Subscribe for more'), shown start→end."""
    t = escape_drawtext(text)
    return (f"drawtext=fontfile='{font}':text='{t}':fontcolor=black:fontsize={fontsize}:"
            f"box=1:boxcolor=yellow@0.95:boxborderw=20:x=(w-text_w)/2:"
            f"y=h-text_h-160:enable='between(t,{start},{end})'")


def hook_cta_vf(title: str | None, cta: str | None, cta_window: tuple[float, float],
                font: str = DEFAULT_FONT) -> str:
    """Compose the title + CTA drawtext chain into one -vf string."""
    parts: list[str] = []
    if title:
        parts.append(title_filter(title, font))
    if cta:
        parts.append(cta_filter(cta, cta_window[0], cta_window[1], font))
    return ",".join(parts)


def vstack_cmd(clip: Path, gameplay: Path, out: Path,
               top_h: int = 1152, bottom_h: int = 768, width: int = 1080) -> list[str]:
    """Build the ffmpeg arg list to stack `clip` over looping `gameplay` (split-screen).

    Gameplay loops to the clip's length; clip audio is kept, gameplay audio dropped.
    """
    fc = (f"[0:v]scale={width}:{top_h}:force_original_aspect_ratio=increase,"
          f"crop={width}:{top_h}[top];"
          f"[1:v]scale={width}:{bottom_h}:force_original_aspect_ratio=increase,"
          f"crop={width}:{bottom_h},setsar=1[bot];"
          f"[top][bot]vstack=inputs=2[v]")
    return [
        "ffmpeg", "-y", "-i", str(clip), "-stream_loop", "-1", "-i", str(gameplay),
        "-filter_complex", fc, "-map", "[v]", "-map", "0:a?",
        "-c:v", "libx264", "-c:a", "aac", "-preset", "veryfast", "-shortest", str(out),
    ]


# ── thin executors ───────────────────────────────────────────────────────────

def _run(cmd: list[str], what: str) -> None:
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    if proc.returncode != 0:
        raise RuntimeError(f"{what} failed: {proc.stderr.strip()[-400:]}")


def apply_overlay(video: Path, out: Path, title: str | None = None, cta: str | None = None,
                  cta_window: tuple[float, float] = (2.0, 7.0), font: str = DEFAULT_FONT) -> Path:
    vf = hook_cta_vf(title, cta, cta_window, font)
    if not vf:
        return video
    _run(["ffmpeg", "-y", "-i", str(video), "-vf", vf, "-c:v", "libx264",
          "-c:a", "copy", "-preset", "veryfast", str(out)], "overlay")
    return out


def stack_gameplay(clip: Path, gameplay: Path, out: Path) -> Path:
    if not gameplay.exists():
        raise FileNotFoundError(f"gameplay loop not found: {gameplay}")
    _run(vstack_cmd(clip, gameplay, out), "gameplay vstack")
    return out


def pick_title(transcript: str, max_words: int = 9) -> str:
    """Heuristic hook title from the transcript: first question, else punchiest line.

    Zero-dependency default. (Optional Ollama upgrade is a later cycle.)
    """
    sentences = [s.strip() for s in transcript.replace("!", ".").replace("?", "?.").split(".")
                 if s.strip()]
    if not sentences:
        return ""
    question = next((s for s in sentences if s.endswith("?")), None)
    pick = question or max(sentences, key=len)
    words = pick.split()
    return " ".join(words[:max_words]) + ("…" if len(words) > max_words else "")
