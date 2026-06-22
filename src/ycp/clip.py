"""Hybrid clip pipeline — the free, uncapped volume engine.

Breaks the Ssemble credit ceiling (~7/day): download with yt-dlp, transcribe with
Whisper, slice into candidate moments (ranked by a cheap heuristic), and cut
vertical captioned 9:16 clips with ffmpeg. Output lands as `pending_qc` clips in
the DB + mp4s in data/clips/, ready for the Slack QC board.

Reserve Ssemble credits for AI moment-detection on heroes + auto-posting; use this
for raw volume. v1 reframe is center-crop (no face-tracking yet) and ranking is a
transcript heuristic (no AI scoring yet) — both noted as future loop cycles.

Pure logic (`plan_clips`, `score_candidate`) is unit-tested; the subprocess steps
(download/transcribe/cut) are thin wrappers verified via a synthetic ffmpeg smoke.
"""
from __future__ import annotations

import hashlib
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path

from . import db, enhance
from .config import ROOT
from .srt import Segment, slice_and_shift, to_srt
from .transcribe import transcribe

CLIPS_DIR = ROOT / "data" / "clips"
HOOK_WORDS = {"why", "how", "never", "secret", "nobody", "actually", "truth",
              "mistake", "stop", "biggest", "worst", "best", "everyone", "wrong"}


@dataclass(frozen=True)
class Candidate:
    start: float
    end: float
    text: str
    score: float

    @property
    def duration(self) -> float:
        return round(self.end - self.start, 2)


def score_candidate(text: str, duration: float) -> float:
    """Cheap, deterministic 'is this a hook?' heuristic. Higher = more promising."""
    t = text.lower()
    score = 1.0
    score += t.count("?") * 0.6
    score += t.count("!") * 0.3
    score += 0.4 if any(c.isdigit() for c in t) else 0.0
    score += 0.3 * sum(1 for w in HOOK_WORDS if w in t.split())
    # duration sweet spot ~30s; taper outside [18, 50]
    if 18 <= duration <= 50:
        score += 1.0
    else:
        score -= min(abs(duration - 32) / 20.0, 1.5)
    return round(score, 3)


def plan_clips(segments: list[Segment], min_len: float = 15, max_len: float = 60,
               top: int | None = None) -> list[Candidate]:
    """Group consecutive transcript segments into candidate windows at sentence
    boundaries, score each, return ranked (best first). Pure + testable."""
    candidates: list[Candidate] = []
    buf: list[Segment] = []

    def flush() -> None:
        if not buf:
            return
        start, end = buf[0].start, buf[-1].end
        if end - start >= min_len:
            text = " ".join(s.text for s in buf).strip()
            candidates.append(Candidate(start, end, text, score_candidate(text, end - start)))

    for seg in segments:
        if buf and (seg.end - buf[0].start) > max_len:
            flush()
            buf = []
        buf.append(seg)
    flush()

    ranked = sorted(candidates, key=lambda c: c.score, reverse=True)
    return ranked[:top] if top else ranked


# ── subprocess steps (thin wrappers) ─────────────────────────────────────────

def download(url: str, workdir: Path) -> Path:
    out = workdir / "source.mp4"
    cmd = ["yt-dlp", "-f", "mp4/best", "-o", str(out), url]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    if proc.returncode != 0 or not out.exists():
        raise RuntimeError(f"download failed: {proc.stderr.strip()[:300]}")
    return out


# transcribe() now lives in transcribe.py (whisper.cpp default, openai-whisper fallback)


def cut_vertical(video: Path, cand: Candidate, srt_text: str, out_path: Path,
                 workdir: Path) -> Path:
    """ffmpeg: trim -> scale/center-crop to 1080x1920 -> burn captions."""
    sub = workdir / "cap.srt"
    sub.write_text(srt_text)
    vf = ("scale=1080:1920:force_original_aspect_ratio=increase,"
          "crop=1080:1920,"
          "subtitles=cap.srt:force_style='Alignment=2,Fontsize=18,Outline=2,MarginV=80'")
    tmp_out = workdir / "out.mp4"
    cmd = ["ffmpeg", "-y", "-i", str(video), "-ss", str(cand.start),
           "-t", str(cand.duration), "-vf", vf, "-c:v", "libx264",
           "-c:a", "aac", "-preset", "veryfast", str(tmp_out)]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600, cwd=workdir)
    if proc.returncode != 0 or not tmp_out.exists():
        raise RuntimeError(f"ffmpeg cut failed: {proc.stderr.strip()[-400:]}")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(str(tmp_out), str(out_path))
    return out_path


def run(url: str, max_clips: int = 6, lane: str = "whop",
        source_creator: str = "unknown", channel: str = "clips",
        hook_cta: bool = False, title: str | None = None, cta: str = "Subscribe for more",
        gameplay: Path | None = None, db_path: Path | None = None) -> list[dict]:
    """Full pipeline: url -> ranked vertical captioned clips registered for QC.

    Optional owned enhancements (Ssemble parity): `hook_cta` burns a top title
    (auto-picked from the transcript unless `title` given) + a CTA banner; `gameplay`
    stacks each clip over a looping gameplay file (split-screen retention).
    """
    db.init_db(db_path)
    created: list[dict] = []
    vid_hash = hashlib.sha1(url.encode()).hexdigest()[:8]
    with tempfile.TemporaryDirectory(prefix="ycp-clip-") as tmp:
        workdir = Path(tmp)
        video = download(url, workdir)
        segments = transcribe(video, workdir)
        candidates = plan_clips(segments, top=max_clips)
        for i, cand in enumerate(candidates):
            clip_id = f"{vid_hash}-{i:02d}"
            sub = to_srt(slice_and_shift(segments, cand.start, cand.end))
            staged = workdir / f"{clip_id}.mp4"
            try:
                cut_vertical(video, cand, sub, staged, workdir)
                cur = staged
                if hook_cta:
                    cur = enhance.apply_overlay(cur, workdir / f"{clip_id}_ov.mp4",
                                                title=title or enhance.pick_title(cand.text),
                                                cta=cta)
                if gameplay:
                    cur = enhance.stack_gameplay(cur, gameplay, workdir / f"{clip_id}_gp.mp4")
            except (RuntimeError, FileNotFoundError) as exc:
                print(f"  ! skip {clip_id}: {exc}")
                continue
            out = CLIPS_DIR / f"{clip_id}.mp4"
            out.parent.mkdir(parents=True, exist_ok=True)
            shutil.move(str(cur), str(out))
            db.insert_clip({
                "clip_id": clip_id, "source_creator": source_creator, "channel": channel,
                "platform": "youtube", "lane": lane, "fmt": "auto-clip",
                "hook_type": "tbd", "length_sec": int(cand.duration), "status": "pending_qc",
                "post_url": str(out),  # local preview path until posted
            }, db_path)
            created.append({"clip_id": clip_id, "file": str(out),
                            "score": cand.score, "len": cand.duration,
                            "preview": cand.text[:80]})
    return created
