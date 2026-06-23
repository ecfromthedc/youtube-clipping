"""Hybrid clip pipeline — the free, uncapped volume engine.

Download with yt-dlp, transcribe with Whisper, slice into candidate moments (ranked by
a cheap heuristic), cut a clean vertical 9:16 clip with ffmpeg, then composite the
hook title + opus-style word-by-word captions on top (see `captions.py`). Output lands
as `pending_qc` clips in the DB + mp4s in data/clips/, ready for the Slack QC board.

v1 reframe is center-crop (no face-tracking yet) and ranking is a transcript heuristic
(no AI scoring yet) — both noted as future loop cycles. Captions are rendered with
Pillow + the ffmpeg `overlay` filter because this ffmpeg has no libass/freetype.

Pure logic (`plan_clips`, `score_candidate`) is unit-tested; the subprocess steps
(download/transcribe/cut/overlay) are thin wrappers verified on a real clip.
"""
from __future__ import annotations

import hashlib
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path

from . import captions, db, enhance, hooks, reframe, vision
from .config import ROOT, settings
from .srt import Segment, slice_and_shift
from .transcribe import transcribe

CLIPS_DIR = ROOT / "data" / "clips"
MAX_CLIP_SEC = 45.0  # hard cap on a clip window (Gemini sometimes returns longer)
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


def _window_text(segments: list[Segment], start: float, end: float) -> str:
    """Transcript text overlapping [start, end] — the hook + caption source for a window."""
    return " ".join(s.text for s in segments if s.end > start and s.start < end).strip()


# -- subprocess steps (thin wrappers) -----------------------------------------

def download(url: str, workdir: Path, window_sec: int | None = None) -> Path:
    out = workdir / "source.mp4"
    cmd = ["yt-dlp", "-f", "mp4/best", "-o", str(out)]
    if window_sec:
        # Bound long sources (podcasts): grab only the first window_sec seconds.
        cmd += ["--download-sections", f"*0-{int(window_sec)}", "--force-keyframes-at-cuts"]
    cmd.append(url)
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=900)
    if out.exists():
        return out
    cands = list(workdir.glob("source*.mp4")) + list(workdir.glob("source*.mkv"))
    if cands:
        return cands[0]
    raise RuntimeError(f"download failed: {proc.stderr.strip()[:300]}")


# transcribe() now lives in transcribe.py (whisper.cpp default, openai-whisper fallback)


def cut_vertical(video: Path, cand: Candidate, out_path: Path, workdir: Path) -> Path:
    """Trim the candidate window, then reframe to a 9:16 vertical that follows the speaker
    (OpenCV face-pan, falling back to a center crop). Captions are composited afterward by
    `captions.burn_captions` (this ffmpeg has no libass/freetype text filters)."""
    trimmed = workdir / "trim.mp4"
    cmd = ["ffmpeg", "-y", "-i", str(video), "-ss", str(cand.start), "-t", str(cand.duration),
           "-c:v", "libx264", "-c:a", "aac", "-preset", "veryfast", str(trimmed)]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600, cwd=workdir)
    if proc.returncode != 0 or not trimmed.exists():
        raise RuntimeError(f"ffmpeg trim failed: {proc.stderr.strip()[-400:]}")
    mode = settings().get("reframe", {}).get("mode", "face")
    return reframe.reframe(trimmed, out_path, workdir, mode=mode)


def run(url: str, max_clips: int = 6, lane: str = "owned",
        source_creator: str = "unknown", channel: str = "clips",
        hook_cta: bool = True, title: str | None = None, cta: str = "Subscribe for more",
        gameplay: Path | None = None, source_video_id: str | None = None,
        angle: str = "", window_sec: int | None = None,
        db_path: Path | None = None) -> list[dict]:
    """Full pipeline: url -> ranked vertical clips with hook title + captions, registered for QC.

    Every clip gets the DeepSeek hook title (the highest-leverage lever) and opus-style
    captions burned on (`captions.burn_captions`). `title` overrides the per-clip hook for
    all clips; `gameplay` stacks each clip over a looping file (split-screen retention).
    Caption failure is non-fatal — the plain vertical clip still ships.
    """
    db.init_db(db_path)
    created: list[dict] = []
    vid_hash = hashlib.sha1(url.encode()).hexdigest()[:8]
    with tempfile.TemporaryDirectory(prefix="ycp-clip-") as tmp:
        workdir = Path(tmp)
        video = download(url, workdir, window_sec=window_sec)
        segments = transcribe(video, workdir)
        moments = vision.rank_moments(video, n=max_clips) if vision.enabled() else []
        if moments:
            print(f"  · Gemini vision picked {len(moments)} moment(s)")
            candidates = [Candidate(m.start, min(m.end, m.start + MAX_CLIP_SEC),
                                    _window_text(segments, m.start,
                                                 min(m.end, m.start + MAX_CLIP_SEC)),
                                    m.score) for m in moments]
        else:
            candidates = plan_clips(segments, top=max_clips)
        for i, cand in enumerate(candidates):
            clip_id = f"{vid_hash}-{i:02d}"
            chunks = captions.build_chunks(slice_and_shift(segments, cand.start, cand.end))
            staged = workdir / f"{clip_id}.mp4"
            try:
                cut_vertical(video, cand, staged, workdir)
                cur = staged
                clip_title = title or hooks.best_hook(cand.text, angle=angle)
                try:
                    cur = captions.burn_captions(
                        staged, chunks, workdir / f"{clip_id}_cap.mp4", workdir,
                        title=clip_title)
                except RuntimeError as exc:
                    print(f"  · captions failed ({exc}); shipping plain clip")
                if gameplay:
                    cur = enhance.stack_gameplay(cur, gameplay, workdir / f"{clip_id}_gp.mp4")
            except (RuntimeError, FileNotFoundError) as exc:
                print(f"  ! skip {clip_id}: {exc}")
                continue
            out = CLIPS_DIR / f"{clip_id}.mp4"
            out.parent.mkdir(parents=True, exist_ok=True)
            shutil.move(str(cur), str(out))
            db.insert_clip({
                "clip_id": clip_id, "source_video_id": source_video_id,
                "source_creator": source_creator, "channel": channel,
                "platform": "youtube", "lane": lane, "fmt": "auto-clip",
                "hook_type": "tbd", "length_sec": int(cand.duration), "status": "pending_qc",
                "post_url": str(out),  # local preview path until posted
            }, db_path)
            created.append({"clip_id": clip_id, "file": str(out),
                            "score": cand.score, "len": cand.duration,
                            "preview": cand.text[:80]})
    return created
