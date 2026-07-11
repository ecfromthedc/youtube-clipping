"""Owned Ssemble-parity enhancements — pure ffmpeg, no app, no ceiling.

Replaces Ssemble's "Hook Title & CTA" and "Game Video" features with local ffmpeg.
The filter/command *builders* are pure functions (unit-tested without running
ffmpeg); the thin `apply_*` wrappers execute them.

Title/CTA text is passed to ffmpeg via `textfile=` (ffmpeg reads it from a file),
which sidesteps drawtext's fragile inline-text escaping — so titles with
apostrophes/colons (e.g. "Don't sleep on 5:00") render correctly. The font is the
vendored TikTok Sans Overlay cut (no spaces in the path), falling back to a stock
macOS font when the asset is missing.

See SSEMBLE-PARITY.md for the full map.
"""
from __future__ import annotations

import functools
import subprocess
from pathlib import Path

# Official font: TikTok Sans Overlay cut (wght 650), vendored in assets/ (path has
# no spaces — drawtext-safe). Falls back to stock macOS Helvetica if assets are absent.
_TIKTOK_OVERLAY = (Path(__file__).resolve().parents[2]
                   / "assets" / "tiktok-font" / "fonts" / "TikTokSans-Overlay.ttf")
DEFAULT_FONT = (str(_TIKTOK_OVERLAY) if _TIKTOK_OVERLAY.is_file()
                else "/System/Library/Fonts/Helvetica.ttc")


@functools.lru_cache(maxsize=None)
def ffmpeg_has_filter(name: str) -> bool:
    """True if this ffmpeg build exposes `name` (e.g. 'drawtext', 'subtitles').

    Some ffmpeg builds ship without libass/libfreetype, so drawtext/subtitles are
    absent. We detect once and let callers degrade gracefully (uncaptioned clip)
    instead of hard-failing — captions/hooks render automatically once ffmpeg has
    the text libs. See SSEMBLE-PARITY.md / setup notes for the libass install.
    """
    try:
        out = subprocess.run(["ffmpeg", "-hide_banner", "-filters"],
                             capture_output=True, text=True, timeout=30).stdout
    except (OSError, subprocess.SubprocessError):
        return False
    return any(line.split()[1:2] == [name] for line in out.splitlines() if line.strip())


def title_filter(textfile: str, font: str = DEFAULT_FONT, fontsize: int = 56) -> str:
    """Top hook-title banner (whole clip). `textfile` is a path ffmpeg reads."""
    return (f"drawtext=fontfile='{font}':textfile='{textfile}':fontcolor=white:"
            f"fontsize={fontsize}:box=1:boxcolor=black@0.55:boxborderw=18:"
            f"x=(w-text_w)/2:y=90")


def cta_filter(textfile: str, start: float, end: float, font: str = DEFAULT_FONT,
               fontsize: int = 48) -> str:
    """Timed bottom CTA banner (e.g. 'Subscribe for more'), shown start→end."""
    return (f"drawtext=fontfile='{font}':textfile='{textfile}':fontcolor=black:"
            f"fontsize={fontsize}:box=1:boxcolor=yellow@0.95:boxborderw=20:"
            f"x=(w-text_w)/2:y=h-text_h-160:enable='between(t,{start},{end})'")


def hook_cta_vf(title_file: str | None, cta_file: str | None,
                cta_window: tuple[float, float], font: str = DEFAULT_FONT) -> str:
    """Compose the title + CTA drawtext chain into one -vf string."""
    parts: list[str] = []
    if title_file:
        parts.append(title_filter(title_file, font))
    if cta_file:
        parts.append(cta_filter(cta_file, cta_window[0], cta_window[1], font))
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

def _run(cmd: list[str], what: str, cwd: Path | None = None) -> None:
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600, cwd=cwd)
    if proc.returncode != 0:
        raise RuntimeError(f"{what} failed: {proc.stderr.strip()[-400:]}")


def apply_overlay(video: Path, out: Path, title: str | None = None, cta: str | None = None,
                  cta_window: tuple[float, float] = (2.0, 7.0), font: str = DEFAULT_FONT) -> Path:
    """Burn a hook title + CTA banner. Text is written to files next to `out` and
    referenced via textfile= (escaping-proof). Runs ffmpeg with cwd=out.parent."""
    if not ffmpeg_has_filter("drawtext"):
        print("  ⚠ ffmpeg lacks drawtext (no libfreetype) — skipping hook/CTA overlay; "
              "reinstall ffmpeg with libfreetype/libass to burn titles")
        return video
    workdir = out.parent
    title_file = cta_file = None
    if title:
        (workdir / "title.txt").write_text(title)
        title_file = "title.txt"
    if cta:
        (workdir / "cta.txt").write_text(cta)
        cta_file = "cta.txt"
    vf = hook_cta_vf(title_file, cta_file, cta_window, font)
    if not vf:
        return video
    _run(["ffmpeg", "-y", "-i", str(video), "-vf", vf, "-c:v", "libx264",
          "-c:a", "copy", "-preset", "veryfast", str(out)], "overlay", cwd=workdir)
    return out


def stack_gameplay(clip: Path, gameplay: Path, out: Path) -> Path:
    if not gameplay.exists():
        raise FileNotFoundError(f"gameplay loop not found: {gameplay}")
    _run(vstack_cmd(clip, gameplay, out), "gameplay vstack")
    return out


def pick_title(transcript: str, max_words: int = 9) -> str:
    """Heuristic hook title from the transcript: first question, else punchiest line.

    Zero-dependency fallback used when the DeepSeek hook agent (hooks.best_hook)
    is unavailable — e.g. no DEEPSEEK_API_KEY configured for the run.
    """
    sentences = [s.strip() for s in transcript.replace("!", ".").replace("?", "?.").split(".")
                 if s.strip()]
    if not sentences:
        return ""
    question = next((s for s in sentences if s.endswith("?")), None)
    pick = question or max(sentences, key=len)
    words = pick.split()
    return " ".join(words[:max_words]) + ("…" if len(words) > max_words else "")
