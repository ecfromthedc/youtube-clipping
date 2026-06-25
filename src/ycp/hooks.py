"""Stage 2.5 — HOOK OPTIMIZER. A specialist that writes viral hook titles.

The hook is the single highest-leverage lever on a clip's virality — the first
1-2 seconds decide whether someone keeps scrolling. So this routes to a strong
model via **DeepSeek**, not a weak local model.

  generate N candidates (DeepSeek) → score each (pure heuristic) → return the best

> Decision history: started on local Ollama (too weak for viral hooks — removed),
> briefly on Claude, now on **DeepSeek** per Eric (2026-06-22): strong enough for
> creative hooks and already paid for. The key lives in 1Password; the run reads
> it from the `DEEPSEEK_API_KEY` env var (inject via `op read` — never on disk).

Only generation hits the API; **scoring is pure and unit-tested**. With no key
configured it falls back to the transcript heuristic so the pipeline never
hard-breaks (same philosophy as transcribe.py's whisper fallback).

GUARDRAIL (HANDOFF §10): for debate/agitation angles the hook attacks the POSITION
or BEHAVIOR, never a protected group or person. The system prompt enforces this and
`looks_safe()` is a last-line code check before any hook is used.
"""
from __future__ import annotations

import functools
import json

import requests

from . import enhance
from .config import ROOT, env, settings

DEFAULT_MODEL = "deepseek-chat"
DEEPSEEK_URL = "https://api.deepseek.com/chat/completions"
PLAYBOOK_PATH = ROOT / "config" / "hook-playbook.md"

# Minimal fallback if the playbook file is missing — keeps the agent functional.
_FALLBACK_PLAYBOOK = (
    "You are a world-class short-form viral hook writer for faceless YouTube/TikTok "
    "clip channels. Write the on-screen TITLE hook that stops the scroll in the first "
    "second. Speed to value (the hook IS the value), tension in the first 5 words, open "
    "a curiosity gap, be specific, MAX 10 words, no emojis/hashtags/quotes, never "
    "clickbait you can't pay off. ALWAYS write the hook entirely in lowercase. Use "
    "punctuation to cue the payoff — a trailing colon to tease what's coming (e.g. "
    "\"when your friend doesn't know what's coming:\") and correct apostrophes. The hook "
    "MUST cue the specific thing that happens in THIS clip (no generic hooks). Use the 5 "
    "hook types — Contrarian, Labeling, Curiosity Gap, Reframe, Pattern Interrupt — pick "
    "the types most likely to succeed for THIS clip. Respond ONLY with JSON: "
    '{"hooks": [{"text": "...", "type": "Curiosity Gap", "fit": 0.0}]}.'
)

# Curiosity / stakes / specificity signals that correlate with stop-scroll hooks.
_CURIOSITY = {"why", "how", "what", "secret", "nobody", "actually", "truth",
              "really", "reason", "behind", "before", "until", "happens"}
_STAKES = {"never", "stop", "mistake", "wrong", "worst", "lost", "ruined",
           "destroyed", "exposed", "caught", "regret", "warning", "fired",
           "broke", "scam", "lie", "lying", "trap"}
_PERSONAL = {"you", "your", "you're", "youre"}

# Last-line safety net: hooks naming a protected class as the target get dropped.
# (Opinion about a POSITION is fine; this only catches a slur/group-as-target.)
_UNSAFE = {"retard", "retarded", "tranny", "faggot", "fag", "n-word", "nigger",
           "kike", "spic", "chink", "groomer"}


def score_hook(hook: str, angle: str = "") -> float:
    """Deterministic 'how stop-scroll is this title?' score. Higher = better. Pure."""
    h = hook.strip()
    if not h:
        return 0.0
    words = h.lower().split()
    n = len(words)
    score = 1.0
    wset = set(words)
    score += 0.8 if h.endswith("?") else 0.0
    score += 0.7 * len(wset & _CURIOSITY)
    score += 0.6 * len(wset & _STAKES)
    score += 0.5 if wset & _PERSONAL else 0.0
    score += 0.5 if any(c.isdigit() for c in h) else 0.0
    # Length: punchy 3-10 words is the sweet spot; taper outside it.
    if 3 <= n <= 10:
        score += 1.0
    else:
        score -= min(abs(n - 6) / 5.0, 1.5)
    if angle == "finance" and wset & {"money", "broke", "rich", "debt", "cash", "$"}:
        score += 0.4
    if angle in ("debate", "agitation") and wset & {"vs", "destroys", "owns", "wrong", "fight"}:
        score += 0.4
    return round(score, 3)


def looks_safe(hook: str) -> bool:
    """Last-line guardrail: reject hooks containing a slur / protected-group target."""
    low = hook.lower()
    return not any(bad in low for bad in _UNSAFE)


def _api_key() -> str | None:
    # env() loads .env (so a key dropped in .env works) and respects a real env var.
    return env()["deepseek_api_key"]


def deepseek_available() -> bool:
    return _api_key() is not None


@functools.lru_cache(maxsize=1)
def _playbook() -> str:
    """The viral-hook copywriting skill (system prompt). Loaded from the editable
    playbook file; falls back to a built-in minimal prompt if it's missing."""
    try:
        return PLAYBOOK_PATH.read_text().strip()
    except OSError:
        return _FALLBACK_PLAYBOOK


def _coerce_candidate(raw: object) -> dict | None:
    """Normalize one raw hook into {text, type, fit}. Accepts a dict (preferred) or
    a bare string (back-compat). Returns None if there's no usable text."""
    if isinstance(raw, str):
        text = raw.strip()
        return {"text": text, "type": "", "fit": 0.5} if text else None
    if isinstance(raw, dict):
        text = str(raw.get("text", "")).strip()
        if not text:
            return None
        try:
            fit = max(0.0, min(1.0, float(raw.get("fit", 0.5))))
        except (TypeError, ValueError):
            fit = 0.5
        return {"text": text, "type": str(raw.get("type", "")), "fit": fit}
    return None


def generate_candidates(moment: str, n: int = 6, angle: str = "",
                        model: str | None = None, timeout: int = 30) -> list[dict]:
    """Ask DeepSeek for N hook candidates with their type + self-rated context fit.

    Returns a list of {text, type, fit}; [] on any failure (caller falls back to the
    heuristic). API only — deterministic scoring/selection happens in `best_hook`.
    """
    key = _api_key()
    if not key:
        return []
    model = model or settings().get("hooks", {}).get("model", DEFAULT_MODEL)
    angle_line = f"This clip's angle: {angle}.\n" if angle else ""
    prompt = (f"{angle_line}Transcript of the clip moment:\n"
              f"\"\"\"{moment.strip()[:1500]}\"\"\"\n\n"
              f"Pick the hook types most likely to succeed for THIS clip, then write {n} "
              f"distinct hook titles as JSON (each with text, type, and a fit score).")
    try:
        resp = requests.post(
            DEEPSEEK_URL,
            headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
            json={
                "model": model,
                "messages": [{"role": "system", "content": _playbook()},
                             {"role": "user", "content": prompt}],
                "response_format": {"type": "json_object"},
                "temperature": 1.3,   # DeepSeek's recommended range for creative writing
                "max_tokens": 1024,
            },
            timeout=timeout,
        )
        resp.raise_for_status()
        content = resp.json()["choices"][0]["message"]["content"]
        raw_hooks = json.loads(content).get("hooks", [])
    except (requests.RequestException, ValueError, KeyError, IndexError):
        return []
    out = [_coerce_candidate(h) for h in raw_hooks]
    return [c for c in out if c]


def _combined_score(cand: dict, angle: str) -> float:
    """Blend the agent's context-fit likelihood (primary — it has the video context)
    with the deterministic stop-scroll heuristic (backstop). Pure."""
    heuristic = min(score_hook(cand["text"], angle) / 5.0, 1.0)  # normalize ~[0,1]
    return 0.6 * cand["fit"] + 0.4 * heuristic


def best_hook(moment: str, angle: str = "", n: int = 6, max_words: int = 10,
              model: str | None = None) -> str:
    """The hook agent's answer: the candidate with the best context-fit + score, else
    the heuristic. Always returns a usable, safe title.

    Selection is driven primarily by the agent's per-hook likelihood of success for
    THIS video (it read the transcript), with the deterministic scorer as a backstop.
    """
    candidates = [c for c in generate_candidates(moment, n, angle, model)
                  if looks_safe(c["text"])]
    if candidates:
        best = max(candidates, key=lambda c: _combined_score(c, angle))
        words = best["text"].split()
        return " ".join(words[:max_words]) + ("…" if len(words) > max_words else "")
    fallback = enhance.pick_title(moment, max_words=max_words)
    return fallback if looks_safe(fallback) else ""
