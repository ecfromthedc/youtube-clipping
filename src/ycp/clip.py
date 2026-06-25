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

from . import archive, captions, db, enhance, hooks, reframe, vision
from .config import ROOT, settings
from .srt import Segment, slice_and_shift
from .transcribe import transcribe

CLIPS_DIR = ROOT / "data" / "clips"
MAX_CLIP_SEC = 38.0  # hard cap on a clip window. 2026 health-Shorts data: 20-35s is the
# retention sweet spot, >45s drops off hard — so we cap short and let the vision picker target 20-35s.
MIN_CLIP_SEC = 12.0  # floor — a sub-12s clip has no room for hook+payoff. Gemini is told 20-35s, so
# this only catches pathologies (e.g. a moment whose start sits near the end of a windowed source).
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


def _vision_candidates(video: Path, segments: list[Segment], max_clips: int) -> list[Candidate]:
    """Gemini-picked moments, clamped to real footage and floored at MIN_CLIP_SEC.

    A moment whose start sits near the end of the (possibly windowed) source would otherwise cut a
    near-empty clip — clamp the window to the actual duration and drop anything that can't reach
    MIN_CLIP_SEC. Empty result → the caller falls back to the transcript heuristic.
    """
    if not vision.enabled():
        return []
    moments = vision.rank_moments(video, n=max_clips)
    if not moments:
        return []
    src_dur = captions._probe_duration(video)  # 0.0 if probe fails → skip the upper clamp
    out: list[Candidate] = []
    for m in moments:
        start = max(0.0, m.start)
        end = min(m.end, start + MAX_CLIP_SEC)
        if src_dur:
            end = min(end, src_dur)
        if end - start < MIN_CLIP_SEC:
            continue  # too short once clamped to actual footage — a malformed clip, skip it
        out.append(Candidate(start, end, _window_text(segments, start, end), m.score))
    if out:
        print(f"  · Gemini vision picked {len(out)} moment(s)")
    return out


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
        angle: str = "", window_sec: int | None = None, captions_on: bool = True,
        db_path: Path | None = None) -> list[dict]:
    """Full pipeline: url -> ranked vertical clips with hook title + captions, registered for QC.

    Every clip gets the DeepSeek hook title (the highest-leverage lever) and opus-style
    captions burned on (`captions.burn_captions`). `title` overrides the per-clip hook for
    all clips; `gameplay` stacks each clip over a looping file (split-screen retention).
    Caption failure is non-fatal — the plain vertical clip still ships.

    RULE #1: never two sets of subtitles. `captions_on=False` (set for sources that already
    burn their own captions) skips our word-by-word track — the hook still renders — so we
    defer to the source's captions instead of stacking a second set.
    """
    db.init_db(db_path)
    created: list[dict] = []
    vid_hash = hashlib.sha1(url.encode()).hexdigest()[:8]
    with tempfile.TemporaryDirectory(prefix="ycp-clip-") as tmp:
        workdir = Path(tmp)
        video = download(url, workdir, window_sec=window_sec)
        segments = transcribe(video, workdir)
        candidates = _vision_candidates(video, segments, max_clips)
        if not candidates:  # vision off/unavailable, or every moment too short once clamped
            # Cap the heuristic fallback at MAX_CLIP_SEC too — the vision path clamps to it, so the
            # fallback must as well, else a 50-60s window slips through to QC.
            candidates = plan_clips(segments, max_len=MAX_CLIP_SEC, top=max_clips)
        from . import optimize
        prefer = optimize.preferred_hooks()  # hook styles the loop has learned are winning
        ab = settings().get("ab", {})
        # A/B only the SINGLE best moment per source (not every hero) — A/B'ing every moment
        # explodes variant count + floods the post schedule. The top moment earns the test.
        top_idx = max(range(len(candidates)), key=lambda j: candidates[j].score) if candidates else -1
        for i, cand in enumerate(candidates):
            clip_id = f"{vid_hash}-{i:02d}"
            # captions_on=False → no chunks → hook renders alone (defer to source's captions).
            chunks = (captions.build_chunks(slice_and_shift(segments, cand.start, cand.end))
                      if captions_on else [])
            staged = workdir / f"{clip_id}.mp4"
            try:
                cut_vertical(video, cand, staged, workdir)
            except (RuntimeError, FileNotFoundError) as exc:
                print(f"  ! skip {clip_id}: {exc}")
                continue

            # Pick the hook set: a manual title, an A/B hero set (distinct angles on the same
            # clip), or a single best hook. Hero = a top-scoring moment worth testing angles on.
            if title:
                hook_set, exp_id = [{"text": title, "type": "manual"}], None
            elif (ab.get("enabled", True) and i == top_idx
                  and cand.score >= ab.get("hero_score", 0.9)):
                hook_set = hooks.variants(cand.text, angle=angle, prefer_types=prefer,
                                          k=int(ab.get("variants", 3)))
                exp_id = f"{vid_hash}-{i:02d}-ab" if len(hook_set) > 1 else None
            else:
                hook_set, exp_id = [hooks.best(cand.text, angle=angle, prefer_types=prefer)], None
            if exp_id:
                print(f"  · hero moment (score {cand.score:.2f}) → A/B {len(hook_set)} hook angles")

            for vi, hook in enumerate(hook_set):
                variant_id = clip_id if len(hook_set) == 1 else f"{clip_id}-v{vi}"
                cur = staged
                try:
                    cur = captions.burn_captions(
                        staged, chunks, workdir / f"{variant_id}_cap.mp4", workdir,
                        title=hook["text"])
                except RuntimeError as exc:
                    print(f"  · captions failed ({exc}); shipping plain clip")
                if gameplay:
                    cur = enhance.stack_gameplay(cur, gameplay, workdir / f"{variant_id}_gp.mp4")
                out = CLIPS_DIR / f"{variant_id}.mp4"
                out.parent.mkdir(parents=True, exist_ok=True)
                # Copy (not move) when the hook burn fell back to `staged`, so the shared base
                # survives for the other variants in this A/B set.
                if cur == staged:
                    shutil.copy2(str(staged), str(out))
                else:
                    shutil.move(str(cur), str(out))
                db.insert_clip({
                    "clip_id": variant_id, "source_video_id": source_video_id,
                    "source_creator": source_creator, "channel": channel,
                    "platform": "youtube", "lane": lane, "fmt": "auto-clip",
                    "hook_type": hook["type"], "length_sec": int(cand.duration),
                    "score": float(cand.score), "status": "pending_qc", "post_title": hook["text"],
                    "experiment_id": exp_id, "variant": hook["type"] if exp_id else None,
                    "post_url": str(out),  # local preview path until posted
                }, db_path)
                created.append({"clip_id": variant_id, "file": str(out),
                                "score": cand.score, "len": cand.duration,
                                "preview": cand.text[:80]})
                # Archive to the Phoenix Protocol drive (best-effort; never blocks posting).
                dest = archive.archive_clip(out, {
                    "clip_id": variant_id, "channel": channel, "hook": hook["text"],
                    "hook_type": hook["type"], "source_creator": source_creator,
                    "score": cand.score, "length_sec": int(cand.duration),
                    "experiment_id": exp_id})
                if dest:
                    print(f"  · archived → {dest}")
    return created
