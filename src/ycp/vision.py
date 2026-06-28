"""Stage 2.1 — VISION MOMENT SELECTOR (Gemini). Picks clippable windows by WATCHING the video.

The transcript heuristic (clip.score_candidate) is blind to the footage. A video-native model
(Gemini 3.5 Flash) watches the actual video and reasons about which windows are most clippable —
emotional peak, visual payoff, reaction, quotable AND watchable. This is the biggest upgrade to
clip selection: the moment IS the clip; captions/reframe can't save a boring 40 seconds.

  upload video (Gemini Files API) -> ask for the N best 18-45s windows as JSON -> return ranked.

Flag- and key-gated: with vision.enabled false or no GEMINI_API_KEY, rank_moments() returns []
and the caller falls back to the transcript heuristic (same never-hard-break philosophy as
hooks.py / transcribe.py). qc_screen() is an optional visual-compliance pass that fails OPEN.
"""
from __future__ import annotations

import json
import time
from dataclasses import dataclass

from .config import env, settings

DEFAULT_MODEL = "gemini-3.5-flash"

_MOMENT_PROMPT = """You are an expert short-form editor for a TALKING-HEAD clip channel.
Watch this video and pick the {n} BEST standalone moments to cut as vertical Shorts.

CRITICAL — the on-camera subject MUST be a PERSON (a speaker / talking head). For the WHOLE window
you choose, a person must be clearly on screen as the main subject. REJECT and never pick windows
that are slides, charts, graphs, screen-shares, code, diagrams, title cards, product shots, logos,
or any b-roll where no person is the focus. A clip with no person on camera is unusable to us — if
a great quote happens over a slide, pick a nearby window where the speaker is actually shown talking.

A great moment also: hooks in the first 1-2 seconds, has a clear payoff or emotional/visual peak, is
quotable, and stands alone. Prefer TIGHT 20-35s windows (the retention sweet spot); never exceed 38s.
Avoid intros, ad reads, dead air, and anything that needs setup.

Return ONLY JSON:
{{"moments":[{{"start_sec": <number>, "end_sec": <number>, "score": <0-1 how viral>,
"person_on_camera": <true|false — is a person the on-camera subject for the WHOLE window?>,
"reason": "<one line: why this clips>"}}]}}
Only include moments where person_on_camera is true. Best first. Timestamps are seconds from the
start of THIS video."""

_REVIEW_PROMPT = """You are a STRICT quality-control reviewer for a faceless TALKING-HEAD clip
channel. Watch this FINISHED vertical Short and decide if it is USABLE to post. Be harsh — when in
doubt, reject. A clip is UNUSABLE if ANY of these is true:
- SUBJECT IS NOT A PERSON: the shot is a slide, chart, graph, screen-share, code, diagram, title
  card, product shot, logo, or b-roll for a meaningful part of the clip.
- WIDE / TINY SPEAKER: the speaker is a small figure in a wide shot (e.g. a far stage shot) rather
  than a head-and-shoulders / upper-body framing.
- BAD FRAMING: the face is cut off, off to the edge, or the crop is centred on the background.
- DOESN'T OPEN ON THE SPEAKER: the first ~1 second is an establishing/wide/title/slide shot, not
  the person talking.
- CUT OFF: the clip ends before the speaker finishes the key sentence/point (the thought is
  incomplete or it stops mid-sentence).
- WRONG SUBJECT: the framed person appears to be an interviewer/host reacting rather than the main
  speaker delivering the point.

Return ONLY JSON:
{"usable": <bool>, "subject": "person"|"slide_or_chart"|"wide_tiny"|"b_roll"|"mixed",
 "opens_on_speaker": <bool>, "complete_thought": <bool>, "well_framed": <bool>,
 "issues": ["<short issue>", ...], "confidence": <0-1>}"""

_QC_PROMPT = """You are a YouTube monetization compliance checker for a faceless clip channel.
Watch this clip and flag anything that risks demonetization or a strike: copyrighted music,
licensed footage (film/TV/sports), on-screen brand logos, or a public figure risky to clip.
Return ONLY JSON: {"ok": <true if safe to post>, "flags": ["<risk>", ...]}."""


@dataclass(frozen=True)
class Moment:
    start: float
    end: float
    score: float
    reason: str

    @property
    def duration(self) -> float:
        return round(self.end - self.start, 2)


def _cfg() -> dict:
    return settings().get("vision", {})


def _api_key() -> str | None:
    return env().get("gemini_api_key")


def vision_available() -> bool:
    return _api_key() is not None


def enabled() -> bool:
    return bool(_cfg().get("enabled", False)) and vision_available()


def _client():
    from google import genai
    return genai.Client(api_key=_api_key())


def _upload_active(client, video_path, timeout: int = 180):
    """Upload a video and wait until the Files API marks it ACTIVE (None on FAILED/timeout)."""
    f = client.files.upload(file=str(video_path))
    waited = 0
    while getattr(f.state, "name", "") != "ACTIVE":
        if getattr(f.state, "name", "") == "FAILED" or waited >= timeout:
            return None
        time.sleep(5)
        waited += 5
        f = client.files.get(name=f.name)
    return f


def _parse_moments(raw) -> list[Moment]:
    out: list[Moment] = []
    for m in raw if isinstance(raw, list) else []:
        try:
            s, e = float(m["start_sec"]), float(m["end_sec"])
        except (KeyError, TypeError, ValueError):
            continue
        if e <= s:
            continue
        if m.get("person_on_camera") is False:   # Gemini says no person on screen → skip (charts/slides)
            continue
        try:
            score = max(0.0, min(1.0, float(m.get("score", 0.5))))
        except (TypeError, ValueError):
            score = 0.5
        out.append(Moment(round(s, 2), round(e, 2), score, str(m.get("reason", ""))[:200]))
    return sorted(out, key=lambda x: x.score, reverse=True)


def rank_moments(video_path, n: int = 6, model: str | None = None) -> list[Moment]:
    """Gemini picks the N most clippable windows. Returns [] if disabled/unavailable/any failure
    (the caller falls back to the transcript heuristic)."""
    if not enabled():
        return []
    try:
        from google.genai import types
        client = _client()
        f = _upload_active(client, video_path)
        if f is None:
            return []
        resp = client.models.generate_content(
            model=model or _cfg().get("model", DEFAULT_MODEL),
            contents=[f, _MOMENT_PROMPT.format(n=n)],
            config=types.GenerateContentConfig(
                response_mime_type="application/json", temperature=0.4),
        )
        moments = _parse_moments(json.loads(resp.text).get("moments", []))
        try:
            client.files.delete(name=f.name)
        except Exception:  # noqa: BLE001
            pass
        return moments[:n]
    except Exception:  # noqa: BLE001  (best-effort; caller falls back to the heuristic)
        return []


def review_clip(video_path, model: str | None = None) -> dict:
    """Strict QC of a FINISHED clip — Gemini watches the rendered Short and judges usability
    (subject is a person, well framed, opens on the speaker, complete thought). Returns a dict
    with 'usable' + the breakdown + 'issues'. Fails OPEN ({'usable': True, 'reviewed': False})
    when Gemini is off/unavailable, so it never silently blocks when we can't actually check."""
    if not enabled():
        return {"usable": True, "reviewed": False, "issues": []}
    try:
        from google.genai import types
        client = _client()
        f = _upload_active(client, video_path)
        if f is None:
            return {"usable": True, "reviewed": False, "issues": []}
        resp = client.models.generate_content(
            model=model or _cfg().get("model", DEFAULT_MODEL),
            contents=[f, _REVIEW_PROMPT],
            config=types.GenerateContentConfig(response_mime_type="application/json"),
        )
        data = json.loads(resp.text)
        try:
            client.files.delete(name=f.name)
        except Exception:  # noqa: BLE001
            pass
        subject = str(data.get("subject", ""))
        opens = bool(data.get("opens_on_speaker", False))
        complete = bool(data.get("complete_thought", False))
        framed = bool(data.get("well_framed", False))
        # usable = subject is a person (or mostly-person) AND it opens on the speaker AND it's
        # well framed. The framing gate is NON-NEGOTIABLE (2026-06-28): without it, a crop centred
        # on the background/wall (the 16:9→9:16 pan missing the speaker) shipped as "usable" because
        # well_framed was ignored — that's how empty-room clips reached the review pile. Cut-off /
        # incomplete-thought stay WARNINGS (Gemini over-flags those), but bad framing now rejects.
        usable = subject in ("person", "mixed") and opens and framed
        warnings = list(data.get("issues") or [])
        if not complete:
            warnings.append("may cut off / incomplete thought")
        if not framed:
            warnings.append("framing could be tighter")
        return {
            "usable": usable, "reviewed": True, "subject": subject,
            "opens_on_speaker": opens, "complete_thought": complete, "well_framed": framed,
            "issues": [str(x) for x in warnings][:8],
            "confidence": float(data.get("confidence", 0.0) or 0.0),
        }
    except Exception:  # noqa: BLE001
        return {"usable": True, "reviewed": False, "issues": []}


def qc_screen(video_path, model: str | None = None) -> dict:
    """Optional visual-compliance pass. {'ok': bool, 'flags': [...]}. Fails OPEN (ok=True) when
    disabled/unavailable so it never blocks the human Slack QC — it's defense-in-depth only."""
    if not enabled():
        return {"ok": True, "flags": []}
    try:
        from google.genai import types
        client = _client()
        f = _upload_active(client, video_path)
        if f is None:
            return {"ok": True, "flags": []}
        resp = client.models.generate_content(
            model=model or _cfg().get("model", DEFAULT_MODEL),
            contents=[f, _QC_PROMPT],
            config=types.GenerateContentConfig(response_mime_type="application/json"),
        )
        data = json.loads(resp.text)
        try:
            client.files.delete(name=f.name)
        except Exception:  # noqa: BLE001
            pass
        return {"ok": bool(data.get("ok", True)), "flags": list(data.get("flags", []))[:8]}
    except Exception:  # noqa: BLE001
        return {"ok": True, "flags": []}
