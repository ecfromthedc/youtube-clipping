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

_MOMENT_PROMPT = """You are an expert short-form editor for faceless YouTube/TikTok clip channels.
Watch this video and pick the {n} BEST standalone moments to cut as vertical Shorts.

A great moment: hooks in the first 1-2 seconds, has a clear payoff or emotional/visual peak, is
quotable, and stands alone without prior context. Prefer 18-45s windows. Avoid intros, ad reads,
dead air, and anything that needs setup.

Return ONLY JSON:
{{"moments":[{{"start_sec": <number>, "end_sec": <number>, "score": <0-1 how viral>,
"reason": "<one line: why this clips>"}}]}}
Best first. Timestamps are seconds from the start of THIS video."""

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
