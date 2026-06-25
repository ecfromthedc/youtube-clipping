"""Stage 5 (capture half) — pull performance into the DB.

Two sources, in order of how much they need:
  • public views   – yt-dlp on each posted clip URL. No creds. Works today.
  • full analytics  – retention %, RPM, ad revenue. Needs YouTube Analytics OAuth
                      per owned channel (see capture_full_analytics docstring).

Public views close the loop on the number that matters most early: how many
views each clip is pulling. Ad revenue follows once owned channels hit YPP.
"""
from __future__ import annotations

import json
import subprocess
from pathlib import Path

from . import db
from .db import connect


def _ytdlp_views(url: str) -> int | None:
    """Current public view count for one video URL (YouTube/TikTok/IG)."""
    cmd = ["yt-dlp", "--dump-json", "--skip-download", url]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=90)
    if proc.returncode != 0:
        return None
    try:
        return int(json.loads(proc.stdout).get("view_count") or 0)
    except (json.JSONDecodeError, ValueError):
        return None


def capture_public(db_path: Path | None = None) -> int:
    """Snapshot public views for every posted clip that has a URL. Returns count."""
    db.init_db(db_path)
    with connect(db_path) as conn:
        rows = conn.execute(
            "SELECT clip_id, post_url FROM clips "
            "WHERE status = 'posted' AND post_url IS NOT NULL"
        ).fetchall()
    n = 0
    for r in rows:
        views = _ytdlp_views(r["post_url"])
        if views is None:
            print(f"  ! no views for {r['clip_id']} ({r['post_url']})")
            continue
        db.insert_metric({"clip_id": r["clip_id"], "views": views}, db_path)
        n += 1
    return n


def capture_full_analytics(db_path: Path | None = None) -> int:
    """Retention %, RPM, ad revenue per owned channel via YouTube Analytics API.

    OAuth is now wired (scripts/yt_oauth.py wrote YT_CLIENT_ID/SECRET/REFRESH_TOKEN/
    CHANNEL_ID to .env, scopes incl. yt-analytics + monetary). The remaining gap is
    the clip→YouTube-videoId linkage: per-video analytics needs each Short's video id,
    which we only learn from the Postiz publish response on a real post. Once clips carry
    a `yt_video_id`, this queries reports(ids='channel==MINE', dimensions='video',
    metrics='views,averageViewPercentage,estimatedRevenue') and writes them per clip.

    Until the first posts flow and the linkage is stored, public views (capture_public)
    drive the loop — views is the dominant scoring weight, so learning works today.
    """
    raise NotImplementedError(
        "Owned analytics ready (OAuth wired) but needs the clip→videoId linkage from "
        "the first Postiz post. capture_public covers the loop until then. See docstring."
    )
