"""Stage 1.5 — GOLDMINE. Find a video's most-rewatched moments from YouTube's own data.

YouTube publishes a "Most replayed" graph; yt-dlp exposes it as `heatmap` (peaks = the
moments viewers scrub back to = pre-validated clips). We pull the heatmap + the timed
subtitles WITHOUT downloading the video, map each rewatch peak to the words spoken there,
and emit ranked clip windows with ready-to-cut `--start/--window` values. This replaces
guesswork: the crowd tells us where the gold is, and the exact deep timestamp solves the
"the moment is 40 min into a 2-hour podcast" problem.

`peak_windows` (the ranking) is pure + unit-tested; the yt-dlp calls are thin wrappers.
No heatmap (small / brand-new videos) → returns [] and the caller falls back to Gemini.
"""
from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass
from pathlib import Path

from .srt import Segment, parse_srt

WINDOW_SEC = 40       # download window around a peak (Gemini/heuristic trims to the clip inside it)
LEAD_SEC = 8          # start this many seconds before the peak — moments build up to the payoff
MIN_GAP_SEC = 25      # don't pick two peaks closer than this (keeps clips distinct)


@dataclass(frozen=True)
class Peak:
    start: float          # download start (peak − lead, clamped ≥ 0)
    end: float            # download end
    peak_t: float         # the rewatch-peak timestamp itself
    intensity: float      # 0–1 replay intensity (1.0 = the single most-replayed moment)
    quote: str            # words spoken across the window

    @property
    def start_min(self) -> float:
        return round(self.start / 60.0, 3)

    @property
    def window_sec(self) -> int:
        return int(round(self.end - self.start))


def peak_windows(heatmap: list[dict], top: int = 5, window: float = WINDOW_SEC,
                 lead: float = LEAD_SEC, min_gap: float = MIN_GAP_SEC) -> list[dict]:
    """Pick the top rewatch peaks, greedily spaced ≥ min_gap apart, each as a download window
    [peak − lead, peak − lead + window]. Pure (no I/O), so it unit-tests. Returns dicts with
    start/end/peak_t/intensity, ranked by intensity."""
    pts = sorted((h for h in heatmap if "value" in h and "start_time" in h),
                 key=lambda h: h["value"], reverse=True)
    chosen: list[dict] = []
    for h in pts:
        pt = float(h["start_time"])
        if any(abs(pt - c["peak_t"]) < min_gap for c in chosen):
            continue
        start = max(0.0, pt - lead)
        chosen.append({"start": round(start, 2), "end": round(start + window, 2),
                       "peak_t": round(pt, 2), "intensity": round(float(h["value"]), 3)})
        if len(chosen) >= top:
            break
    return chosen


def _dedupe(text: str) -> str:
    """YouTube auto-subs repeat phrases as the caption scrolls. Collapse consecutive dup words."""
    out: list[str] = []
    for w in text.split():
        if not (out and out[-1].lower() == w.lower()):
            out.append(w)
    # also collapse an immediately-repeated 2-3 word run (…the age the age… → …the age…)
    cleaned: list[str] = []
    for w in out:
        cleaned.append(w)
        for n in range(8, 1, -1):   # collapse an immediately-repeated run of n words (YT scroll-dup)
            if len(cleaned) >= 2 * n and [c.lower() for c in cleaned[-n:]] == \
                    [c.lower() for c in cleaned[-2 * n:-n]]:
                del cleaned[-n:]
                break
    return " ".join(cleaned)


def quote_at(segments: list[Segment], start: float, end: float) -> str:
    """The (de-duplicated) words spoken across [start, end]."""
    txt = " ".join(s.text for s in segments if s.end > start and s.start < end).strip()
    return _dedupe(txt)


# -- thin yt-dlp wrappers (no video download) ---------------------------------

def _fetch_heatmap(url: str) -> tuple[list[dict], str]:
    """(heatmap, title) via a metadata-only yt-dlp dump. ([], '') on any failure."""
    try:
        out = subprocess.run(
            ["yt-dlp", "--no-warnings", "--skip-download", "-J", url],
            capture_output=True, text=True, timeout=120)
        d = json.loads(out.stdout)
        return (d.get("heatmap") or []), (d.get("title") or "")
    except (OSError, subprocess.SubprocessError, json.JSONDecodeError, ValueError):
        return [], ""


def _fetch_subs(url: str, workdir: Path) -> list[Segment]:
    """Timed transcript (manual subs if present, else auto) as Segments, no video download."""
    subprocess.run(
        ["yt-dlp", "--no-warnings", "--skip-download", "--write-subs", "--write-auto-subs",
         "--sub-lang", "en.*", "--convert-subs", "srt", "-o", str(workdir / "%(id)s.%(ext)s"), url],
        capture_output=True, text=True, timeout=180)
    srts = sorted(workdir.glob("*.srt"), key=lambda p: ("auto" in p.name, len(p.name)))
    for srt in srts:                       # prefer a manual (shorter-named) track over auto
        try:
            return parse_srt(srt.read_text())
        except (OSError, ValueError):
            continue
    return []


def run(url: str, top: int = 5) -> tuple[list[Peak], str]:
    """Pull heatmap + subs, map peaks to quotes, return (ranked Peaks, video title).
    Empty list if the video has no heatmap (too small/new) — caller falls back to Gemini."""
    import tempfile
    heatmap, title = _fetch_heatmap(url)
    if not heatmap:
        return [], title
    with tempfile.TemporaryDirectory(prefix="ycp-gold-") as tmp:
        segments = _fetch_subs(url, Path(tmp))
    peaks = []
    for w in peak_windows(heatmap, top=top):
        quote = quote_at(segments, w["start"], w["end"]) if segments else ""
        peaks.append(Peak(w["start"], w["end"], w["peak_t"], w["intensity"], quote))
    return peaks, title


def render_md(url: str, peaks: list[Peak], title: str) -> str:
    """Operator-readable goldmine report with ready-to-cut commands."""
    if not peaks:
        return (f"# 🪙 Goldmine — «{title}»\n\nNo rewatch heatmap (video too small or new). "
                "Fall back to `ycp clip` (Gemini picks the moment).\n")
    lines = [f"# 🪙 Goldmine — «{title}»", "",
             "Most-rewatched moments (YouTube 'Most replayed') → ready-to-cut windows. "
             "Intensity 1.0 = the single most-replayed second.", ""]
    for i, p in enumerate(peaks, 1):
        m, s = int(p.peak_t) // 60, int(p.peak_t) % 60
        bar = "█" * max(1, round(p.intensity * 12))
        lines += [f"## {i}. {m}:{s:02d}  ·  intensity {p.intensity:.2f}  {bar}",
                  f"> “{p.quote[:200]}”" if p.quote else "> _(no transcript)_",
                  f"`ycp clip \"{url}\" --start {p.start_min:g} --window {p.window_sec / 60:.2g} "
                  f"--max 1 --title \"...\"`  _(--start/--window are MINUTES)_", ""]
    return "\n".join(lines)


if __name__ == "__main__":  # tiny self-check (pure logic, no network)
    hm = [{"start_time": float(t), "end_time": t + 5, "value": v}
          for t, v in [(0, 0.2), (30, 1.0), (33, 0.9), (120, 0.6), (600, 0.8)]]
    w = peak_windows(hm, top=3)
    assert [round(x["peak_t"]) for x in w] == [30, 600, 120], w  # 33 dropped (too near 30)
    assert w[0]["start"] == 22.0 and w[0]["end"] == 62.0, w       # peak30 − 8 lead, +40 window
    assert _dedupe("the age the age of scaling") == "the age of scaling"
    print("goldmine self-check OK")
