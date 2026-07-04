"""Clip pipeline tests — pure planning/scoring + a real ffmpeg reframe smoke."""
from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

import pytest

from ycp import clip
from ycp.clip import Candidate, plan_clips, score_candidate
from ycp.srt import Segment


def test_score_rewards_hooks_and_sweet_spot_duration():
    hooky = score_candidate("Why did nobody tell you the truth about 3 mistakes?", 30)
    bland = score_candidate("and then we walked over there slowly", 30)
    assert hooky > bland
    # same text, bad duration scores lower than sweet-spot duration
    assert score_candidate("a normal sentence", 30) > score_candidate("a normal sentence", 90)


def test_plan_clips_windows_and_ranks():
    # 0-12s, 12-24s, 24-36s segments -> windows within [15,60]
    segs = [
        Segment(0, 12, "intro rambling here"),
        Segment(12, 24, "why is this the biggest mistake? 5 reasons!"),
        Segment(24, 36, "calm outro"),
    ]
    cands = plan_clips(segs, min_len=15, max_len=60)
    assert cands, "should produce at least one candidate"
    assert all(15 <= c.duration <= 60 for c in cands)
    # ranked best-first
    assert cands == sorted(cands, key=lambda c: c.score, reverse=True)


def test_plan_clips_respects_max_len():
    segs = [Segment(i * 10, (i + 1) * 10, f"seg {i}") for i in range(10)]  # 100s total
    cands = plan_clips(segs, min_len=15, max_len=40)
    assert all(c.duration <= 40 for c in cands)


def test_vision_candidates_clamp_and_floor(monkeypatch, tmp_path: Path):
    """A moment whose start sits near the end of the footage must be dropped, not cut to a stub
    (the 0.31s bug). Good moments survive, clamped to MAX_CLIP_SEC."""
    from ycp import captions, vision
    from ycp.vision import Moment

    moments = [
        Moment(50.0, 80.0, 0.95, "good"),    # 30s — kept (≤38)
        Moment(170.0, 200.0, 0.9, "tail"),   # clamps to 180 → 10s < MIN_CLIP_SEC → dropped
        Moment(10.0, 90.0, 0.8, "too long"),  # clamps to 10+38=48 → 38s kept
    ]
    monkeypatch.setattr(vision, "enabled", lambda: True)
    monkeypatch.setattr(vision, "rank_moments", lambda *a, **k: moments)
    monkeypatch.setattr(captions, "_probe_duration", lambda p: 180.0)
    cands = clip._vision_candidates(Path("ignored.mp4"), [], max_clips=6)
    assert len(cands) == 2  # tail moment dropped
    assert all(c.duration >= clip.MIN_CLIP_SEC for c in cands)
    assert all(c.duration <= clip.MAX_CLIP_SEC for c in cands)


@pytest.mark.skipif(
    not shutil.which("ffmpeg") or not shutil.which("ffprobe"),
    reason="ffmpeg/ffprobe not installed")
def test_cut_vertical_produces_1080x1920(tmp_path: Path):
    # synthetic 6s landscape test video; cut_vertical is now pure scale/crop (no libass)
    src = tmp_path / "source.mp4"
    subprocess.run(
        ["ffmpeg", "-y", "-f", "lavfi", "-i", "testsrc=duration=6:size=640x480:rate=15",
         "-f", "lavfi", "-i", "sine=frequency=440:duration=6",
         "-c:v", "libx264", "-c:a", "aac", "-pix_fmt", "yuv420p", str(src)],
        capture_output=True, check=True,
    )
    cand = Candidate(0.0, 4.0, "test caption", 2.0)
    out = tmp_path / "out.mp4"
    clip.cut_vertical(src, cand, out, tmp_path)
    assert out.exists()
    dims = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0",
         "-show_entries", "stream=width,height", "-of", "csv=p=0", str(out)],
        capture_output=True, text=True, check=True,
    ).stdout.strip()
    assert dims == "1080,1920"  # reframed to 9:16


def test_section_builder():
    """_section: start offset + window → yt-dlp section string; no window → to end."""
    from ycp.clip import _section
    assert _section(0, 30) == "*0-30"
    assert _section(2520, 480) == "*2520-3000"   # --start 42 --window 8 (minutes→sec)
    assert _section(600, None) == "*600-inf"
    assert _section(0, None) == "*0-inf"


def test_run_bounded_returns_output_on_success():
    r = clip._run_bounded(["sh", "-c", "printf hi"], timeout=10)
    assert r.returncode == 0 and r.stdout == "hi"


def test_run_bounded_kills_whole_tree_on_timeout():
    """The download hang was subprocess.run(timeout=) blocking on a pipe held by yt-dlp's orphaned
    ffmpeg child. _run_bounded must return PROMPTLY on timeout (group-kill), not wait out the child."""
    import time
    t0 = time.monotonic()
    with pytest.raises(subprocess.TimeoutExpired):
        clip._run_bounded(["sh", "-c", "sleep 30"], timeout=1)
    assert time.monotonic() - t0 < 15   # didn't deadlock on the child's pipe
