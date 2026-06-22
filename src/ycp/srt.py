"""Tiny SRT toolkit — parse Whisper output, then slice/shift it per clip.

Pure functions (no I/O beyond explicit read/write helpers) so the timing logic
is unit-tested without invoking Whisper or ffmpeg.
"""
from __future__ import annotations

import re
from dataclasses import dataclass

_TS = re.compile(r"(\d\d):(\d\d):(\d\d)[,.](\d\d\d)")


@dataclass(frozen=True)
class Segment:
    start: float  # seconds
    end: float
    text: str


def _parse_ts(s: str) -> float:
    m = _TS.search(s)
    if not m:
        return 0.0
    h, mm, ss, ms = (int(x) for x in m.groups())
    return h * 3600 + mm * 60 + ss + ms / 1000.0


def _fmt_ts(t: float) -> str:
    t = max(t, 0.0)
    h = int(t // 3600)
    m = int((t % 3600) // 60)
    s = int(t % 60)
    ms = int(round((t - int(t)) * 1000))
    return f"{h:02d}:{m:02d}:{s:02d},{ms:03d}"


def parse_srt(text: str) -> list[Segment]:
    """Parse an SRT string into ordered Segments."""
    blocks = re.split(r"\n\s*\n", text.strip())
    out: list[Segment] = []
    for block in blocks:
        lines = [ln for ln in block.splitlines() if ln.strip()]
        if len(lines) < 2:
            continue
        ts_line = next((ln for ln in lines if "-->" in ln), None)
        if not ts_line:
            continue
        left, right = ts_line.split("-->")
        body = " ".join(lines[lines.index(ts_line) + 1:]).strip()
        out.append(Segment(_parse_ts(left), _parse_ts(right), body))
    return out


def slice_and_shift(segments: list[Segment], start: float, end: float) -> list[Segment]:
    """Return segments overlapping [start, end], retimed to begin at 0."""
    out: list[Segment] = []
    for seg in segments:
        if seg.end <= start or seg.start >= end:
            continue
        out.append(Segment(max(seg.start, start) - start,
                            min(seg.end, end) - start, seg.text))
    return out


def to_srt(segments: list[Segment]) -> str:
    """Render Segments back to an SRT string."""
    parts = []
    for i, seg in enumerate(segments, 1):
        parts.append(f"{i}\n{_fmt_ts(seg.start)} --> {_fmt_ts(seg.end)}\n{seg.text}")
    return "\n\n".join(parts) + "\n"
