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
import os
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

def _section(start_sec: int, window_sec: int | None) -> str:
    """yt-dlp --download-sections value. Pure + tested. `*START-END`, or `*START-inf` to the
    end when no window is given. The gold in a 90-min interview is deep in the episode, so we
    must be able to start past the cold-open montage, not only grab the first N seconds."""
    end = f"{start_sec + window_sec}" if window_sec else "inf"
    return f"*{start_sec}-{end}"


def download(url: str, workdir: Path, window_sec: int | None = None,
             start_sec: int = 0) -> Path:
    out = workdir / "source.mp4"
    # `mp4/best` grabs YouTube's *progressive* single-file mp4 — capped at 360p/720p — so every
    # clip starts life upscaled. Ask for the best separate video+audio (DASH, up to 1080p) and
    # merge to mp4: real source resolution → no pixelation from upscaling a low-res start.
    cmd = ["yt-dlp",
           "-f", "bestvideo[height<=1080][ext=mp4]+bestaudio[ext=m4a]/bestvideo[height<=1080]+bestaudio/best",
           "--merge-output-format", "mp4", "-o", str(out)]
    if window_sec or start_sec:
        # Bound long sources (podcasts): grab [start_sec, start_sec+window_sec]. The downloaded
        # file starts at ~0, so transcript/vision/cut timestamps stay chunk-relative downstream.
        cmd += ["--download-sections", _section(start_sec, window_sec), "--force-keyframes-at-cuts"]
    cmd.append(url)
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=900)
    if out.exists():
        return out
    cands = list(workdir.glob("source*.mp4")) + list(workdir.glob("source*.mkv"))
    if cands:
        return cands[0]
    raise RuntimeError(f"download failed: {proc.stderr.strip()[:300]}")


# transcribe() now lives in transcribe.py (whisper.cpp default, openai-whisper fallback)


def cut_vertical(video: Path, cand: Candidate, out_path: Path, workdir: Path,
                 segments: list[Segment] | None = None) -> Path:
    """Trim the candidate window, then reframe to a 9:16 vertical that follows the speaker
    (OpenCV face-pan, falling back to a center crop). Captions are composited afterward by
    `captions.burn_captions` (this ffmpeg has no libass/freetype text filters).

    When `segments` (the full transcript) is supplied, the window is sliced + shifted to clip-local
    time and handed to reframe so face-tracking can lock onto the ACTIVE SPEAKER (mouth-motion vs
    speech), not just the most-present face. Optional — omit it for the prior most-present behavior."""
    trimmed = workdir / "trim.mp4"
    cmd = ["ffmpeg", "-y", "-i", str(video), "-ss", str(cand.start), "-t", str(cand.duration),
           "-c:v", "libx264", "-crf", "18", "-c:a", "aac", "-preset", "veryfast", str(trimmed)]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600, cwd=workdir)
    if proc.returncode != 0 or not trimmed.exists():
        raise RuntimeError(f"ffmpeg trim failed: {proc.stderr.strip()[-400:]}")
    mode = settings().get("reframe", {}).get("mode", "face")
    local_segs = slice_and_shift(segments, cand.start, cand.end) if segments else None
    return reframe.reframe(trimmed, out_path, workdir, mode=mode, segments=local_segs)


def _prefer_on_camera(video: Path, candidates: list[Candidate], keep: int,
                      floor: float = 0.34) -> list[Candidate]:
    """Keep the best `keep` candidates whose window actually shows an adequately-sized speaker
    (face_coverage ≥ floor). Drops slide/chart/b-roll AND wide-tiny-speaker windows. Returns []
    when NOTHING is well-framed — better to skip a source than ship a clip with no real subject.
    Order preserves virality ranking; only poorly-framed windows are filtered out."""
    if settings().get("reframe", {}).get("mode", "face") != "face":
        return candidates[:keep]
    good = [c for c in candidates if reframe.face_coverage(video, c.start, c.end) >= floor]
    return good[:keep]


def _extend_to_sentence(cand: Candidate, segments: list[Segment]) -> Candidate:
    """If the clip currently ends MID-sentence, extend its end to the next sentence boundary
    (so it doesn't cut off before the speaker finishes the point), capped at MAX_CLIP_SEC."""
    prior = [s for s in segments if s.end <= cand.end + 0.3]
    if prior and prior[-1].text.rstrip().endswith((".", "!", "?", '"', "…")):
        return cand                                  # already ends on a complete thought
    limit = cand.start + MAX_CLIP_SEC
    new_end = cand.end
    for s in segments:
        if s.start < cand.end or s.start > limit:
            if s.start > limit:
                break
            continue
        new_end = min(s.end, limit)
        if s.text.rstrip().endswith((".", "!", "?", '"', "…")):
            break
    if new_end > cand.end + 0.2:
        return Candidate(cand.start, round(new_end, 2),
                         _window_text(segments, cand.start, new_end), cand.score)
    return cand


def _trim_to_speaker(video: Path, cand: Candidate, segments: list[Segment]) -> Candidate:
    """Advance the clip start to where the speaker first appears, so it never opens on a
    speaker-less establishing/wide/slide shot. No-op if the speaker is already there or trimming
    would push the clip under MIN_CLIP_SEC."""
    if settings().get("reframe", {}).get("mode", "face") != "face":
        return cand
    # Require a CLOSE face (≥12% of width) so we skip the wide establishing shot — a tiny speaker
    # on the edge is what let clips open on the room/wall. Look up to 6s in for the close-up cut.
    ft = reframe.first_face_time(video, cand.start, cand.end, max_skip=6.0, min_face_frac=0.12)
    if ft - cand.start > 0.3 and cand.end - ft >= MIN_CLIP_SEC:
        return Candidate(round(ft, 2), cand.end, _window_text(segments, ft, cand.end), cand.score)
    return cand


def _already_produced(vid_hash: str, channel: str, creator: str,
                      title: str | None, db_path: Path | None) -> bool:
    """True if this clip already exists — by source+moment id (files in any clips/ subfolder)
    or by creator+title in the DB. Stops the pipeline from regenerating clips already produced
    (incl. ones the operator already reviewed/used)."""
    if list(CLIPS_DIR.glob(f"**/{vid_hash}-*.mp4")):
        return True
    if title:
        from .db import connect
        try:
            with connect(db_path) as c:
                row = c.execute(
                    "SELECT 1 FROM clips WHERE channel=? AND source_creator=? AND post_title=? LIMIT 1",
                    (channel, creator, title)).fetchone()
            return row is not None
        except Exception:  # noqa: BLE001  (dedup is best-effort; never block a cut on a db hiccup)
            return False
    return False


def run(url: str, max_clips: int = 6, lane: str = "owned",
        source_creator: str = "unknown", channel: str = "clips",
        hook_cta: bool = True, title: str | None = None, cta: str = "Subscribe for more",
        gameplay: Path | None = None, source_video_id: str | None = None,
        angle: str = "", window_sec: int | None = None, start_sec: int = 0,
        captions_on: bool = True, exact: tuple[float, float] | None = None,
        force: bool = False, db_path: Path | None = None) -> list[dict]:
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
    # REFINEMENT (exact) mode: re-cut a precise [in, out] of the source — no moment-pick, honour
    # the bounds. Each refine gets a fresh id and bypasses the dedup (re-cutting is intentional).
    if exact:
        start_sec = int(exact[0])
        window_sec = max(1, int(round(exact[1] - exact[0])))
        vid_hash = hashlib.sha1(f"{url}@{exact}@{title}@{os.urandom(4).hex()}".encode()).hexdigest()[:8]
    elif force:
        # FORCE — a fresh cut even if we made one here before (unique id, no dedup gate).
        vid_hash = hashlib.sha1(f"{url}@{start_sec}@{os.urandom(4).hex()}".encode()).hexdigest()[:8]
    else:
        # Include the start offset so two DIFFERENT moments cut from the SAME video get distinct
        # ids (else clip_id collides and the second silently overwrites the first).
        vid_hash = hashlib.sha1(f"{url}@{start_sec}".encode()).hexdigest()[:8]
        # DEDUP — never remake a clip we already have.
        if _already_produced(vid_hash, channel, source_creator, title, db_path):
            print(f"  ⟳ skip — already produced: {source_creator} “{title or '(auto-hook)'}”")
            return []
    with tempfile.TemporaryDirectory(prefix="ycp-clip-") as tmp:
        workdir = Path(tmp)
        video = download(url, workdir, window_sec=window_sec, start_sec=start_sec)
        segments = transcribe(video, workdir)
        if exact:
            # The whole downloaded segment IS the clip — no moment-pick, no on-camera gate, no
            # trim/extend. The operator (refinement request) defined these exact bounds.
            dur = captions._probe_duration(video) or float(window_sec or 0)
            candidates = [Candidate(0.0, round(dur, 2), _window_text(segments, 0.0, dur), 1.0)]
        else:
            # Over-generate candidates, then prefer windows where the SPEAKER is on camera — a
            # talk that cut to a full-screen slide (or b-roll) has no face to follow and reframes
            # to the slide. Gate added 2026-06-27 after a Karpathy clip sat on a slide.
            n_pick = max(max_clips, 6)
            candidates = _vision_candidates(video, segments, n_pick)
            if not candidates:  # vision off/unavailable, or every moment too short once clamped
                candidates = plan_clips(segments, max_len=MAX_CLIP_SEC, top=n_pick)
            candidates = _prefer_on_camera(video, candidates, max_clips)
            candidates = [_trim_to_speaker(video, c, segments) for c in candidates]
            candidates = [_extend_to_sentence(c, segments) for c in candidates]
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
                cut_vertical(video, cand, staged, workdir, segments=segments)
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
                # ---- hardened QC gate: Gemini reviews the FINISHED clip and routes it ----
                # usable → unreviewed/ (for human review); not usable → unusable/ (auto-reject,
                # so charts/wide-tiny/cut-off clips never reach the human queue). Fails OPEN
                # (usable) when Gemini is unavailable, so it can't silently swallow everything.
                # THE GATE: Gemini visually signs off on the WHOLE clip (contact sheet of every ~3s)
                # before it can reach the human pile. FAILS CLOSED — no explicit pass ⇒ unusable.
                # Runs on every clip including refinements; an empty-room/wall opening is junk either
                # way. This is the hard rule: nothing hits unreviewed/ without a visual sign-off.
                review = vision.gate(cur)
                usable = review.get("usable", False)   # fail closed — no explicit pass ⇒ rejected
                status = "pending_qc" if usable else "rejected"
                out = CLIPS_DIR / ("unreviewed" if usable else "unusable") / f"{variant_id}.mp4"
                out.parent.mkdir(parents=True, exist_ok=True)
                # Copy (not move) when the hook burn fell back to `staged`, so the shared base
                # survives for the other variants in this A/B set.
                if cur == staged:
                    shutil.copy2(str(staged), str(out))
                else:
                    shutil.move(str(cur), str(out))
                if review.get("reviewed") and not usable:
                    print(f"  ✗ QC rejected → unusable/ [{review.get('subject','?')}] "
                          f"{'; '.join(review.get('issues', [])[:3])}")
                db.insert_clip({
                    "clip_id": variant_id, "source_video_id": source_video_id,
                    "source_creator": source_creator, "channel": channel,
                    "platform": "youtube", "lane": lane, "fmt": "auto-clip",
                    "hook_type": hook["type"], "length_sec": int(cand.duration),
                    "score": float(cand.score), "status": status, "post_title": hook["text"],
                    "experiment_id": exp_id, "variant": hook["type"] if exp_id else None,
                    "post_url": str(out),  # local preview path until posted
                    # provenance: the EXACT source + absolute in/out, so refinement ops can re-cut
                    # this same moment (download offset + the picked window within it).
                    "source_url": url,
                    "clip_start": round(float(start_sec) + cand.start, 2),
                    "clip_end": round(float(start_sec) + cand.end, 2),
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
