"""Milestone watcher — alert when the channel crosses monetization thresholds.

The north star is AdSense → $15,000/month. This pulls owner stats (subs, trailing-90-day
views, trailing-30-day revenue run-rate) via the YouTube Data + Analytics APIs (reusing the
OAuth from scripts/yt_oauth.py), tracks which milestones already fired (idempotent), and posts
a Slack alert to the QC channel on each NEW crossing. Safe no-op without creds.

Ladders encode the 2026 YPP gates: 500 subs + 3M Shorts views/90d = ENTRY tier; 1k subs /
10M views/90d = full; and the goal: $15k/month.
"""
from __future__ import annotations

import datetime
import json
from typing import Any

from . import capture
from .config import ROOT

STATE_PATH = ROOT / "data" / "milestones.json"

SUBS = [100, 500, 1000, 5000, 10_000, 50_000, 100_000]
VIEWS_90D = [1_000_000, 3_000_000, 10_000_000, 50_000_000, 100_000_000]
REVENUE_MO = [1, 100, 1000, 5000, 10_000, 15_000, 25_000, 50_000]

_SPECIAL = {
    ("subs", 100): "100 subscribers — PREP monetization now (see steps): turn on 2-step verification + ready AdSense",
    ("subs", 500): "500 subscribers — YouTube Partner Program ENTRY tier (need 3M Shorts views/90d too)",
    ("subs", 1000): "1,000 subscribers — FULL YPP tier",
    ("views90d", 3_000_000): "3M Shorts views / 90 days — YPP entry monetization threshold",
    ("views90d", 10_000_000): "10M Shorts views / 90 days — full YPP Shorts path",
    ("rev", 15_000): "$15,000 / MONTH run-rate — 🏆 THE GOAL 🏆",
}

# What to actually DO once entry-eligible (2026 YPP onboarding). Fired in the alert.
MONETIZE_STEPS = (
    "💰 *ACTION — turn on monetization:*\n"
    "1. YouTube Studio → *Earn* → Apply (the option appears once eligible)\n"
    "2. Turn ON *2-step verification* on the channel's Google account (required before applying)\n"
    "3. Link an AdSense-for-YouTube account — *create it inside Studio's Earn flow*, not separately\n"
    "4. Accept the *YPP Terms* AND the *Shorts Monetization Module* (Shorts ad-rev only counts from the accept date)\n"
    "5. Submit → review (~1 month) · add *tax info* in AdSense · keep zero Community-Guidelines strikes\n"
    "⚠️ Only *transformed* clips earn (our captions/hooks/reframe) — raw reuploads = ineligible views. Guardrails already enforce this."
)


def _label(kind: str, t: int) -> str:
    if (kind, t) in _SPECIAL:
        return _SPECIAL[(kind, t)]
    if kind == "subs":
        return f"{t:,} subscribers"
    if kind == "views90d":
        return f"{t:,} views / 90 days"
    return f"${t:,}/month run-rate"


def crossed(current: float, ladder: list[int], fired: set[str], kind: str) -> list[str]:
    """Newly-crossed thresholds (adds them to `fired`). Pure given `fired`."""
    out = []
    for t in ladder:
        mk = f"{kind}:{t}"
        if current >= t and mk not in fired:
            fired.add(mk)
            out.append(_label(kind, t))
    return out


def _read_state() -> set[str]:
    try:
        return set(json.loads(STATE_PATH.read_text()))
    except (OSError, ValueError):
        return set()


def _save_state(fired: set[str]) -> None:
    STATE_PATH.parent.mkdir(parents=True, exist_ok=True)
    STATE_PATH.write_text(json.dumps(sorted(fired), indent=2))


def fetch_stats() -> dict[str, Any] | None:
    """subs + lifetime views (Data API) + trailing-90d views & 30d revenue (Analytics API).
    None without creds/libs (caller no-ops)."""
    creds = capture._yt_creds()
    if creds is None:
        return None
    try:
        from googleapiclient.discovery import build
    except ImportError:
        return None
    yt = build("youtube", "v3", credentials=creds)
    items = (yt.channels().list(part="statistics", mine=True).execute().get("items") or [])
    if not items:
        return None
    st = items[0]["statistics"]
    ya = build("youtubeAnalytics", "v2", credentials=creds)
    today = datetime.date.today()

    def metric(days: int, name: str) -> float:
        try:
            start = (today - datetime.timedelta(days=days)).isoformat()
            r = ya.reports().query(ids="channel==MINE", startDate=start,
                                   endDate=today.isoformat(), metrics=name).execute()
            return float((r.get("rows") or [[0]])[0][0])
        except Exception:  # noqa: BLE001 — pre-monetization revenue 404s/returns nothing
            return 0.0

    return {
        "subs": int(st.get("subscriberCount", 0)),
        "lifetime_views": int(st.get("viewCount", 0)),
        "views_90d": metric(90, "views"),
        "revenue_30d": metric(30, "estimatedRevenue"),  # ≈ monthly run-rate
    }


def progress_line(s: dict[str, Any]) -> str:
    return (f"subs {s['subs']:,}/500 (YPP) · 90d views {int(s['views_90d']):,}/3M · "
            f"run-rate ${s['revenue_30d']:,.0f}/mo of $15,000")


def check(post: bool = True) -> dict[str, Any]:
    """Pull stats, detect newly-crossed milestones, Slack-alert each. Idempotent."""
    stats = fetch_stats()
    if stats is None:
        return {"ok": False, "note": "no YouTube OAuth creds — run scripts/yt_oauth.py"}
    fired = _read_state()
    new = (crossed(stats["subs"], SUBS, fired, "subs")
           + crossed(stats["views_90d"], VIEWS_90D, fired, "views90d")
           + crossed(stats["revenue_30d"], REVENUE_MO, fired, "rev"))
    # Entry-tier eligibility = 500 subs AND 3M Shorts views/90d → time to actually apply.
    entry_eligible = (stats["subs"] >= 500 and stats["views_90d"] >= 3_000_000
                      and "eligible:entry" not in fired)
    if entry_eligible:
        fired.add("eligible:entry")
        new.append("🟢 YPP ENTRY ELIGIBLE — apply for monetization NOW")
    if new:
        _save_state(fired)
        if post:
            msg = ("🚨 *Phoenix Protocol — milestone hit!*\n"
                   + "\n".join(f"• {m}" for m in new)
                   + f"\n\n_Now: {progress_line(stats)}_")
            if entry_eligible:
                msg += "\n\n" + MONETIZE_STEPS
            try:
                from . import slack_qc
                slack_qc.post_message(msg)
            except Exception as exc:  # noqa: BLE001 — Slack optional; never break the watcher
                print(f"[milestone] slack unavailable ({exc}):\n{msg}")
    return {"ok": True, "stats": stats, "new_milestones": new, "entry_eligible": entry_eligible}
