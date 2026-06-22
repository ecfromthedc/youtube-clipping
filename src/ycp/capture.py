"""Stage 5 (capture half) — pull performance into the DB.

Three sources, in order of how much they need:
  • public views   – yt-dlp on each posted clip URL. No creds. Works today.
  • Whop payouts    – import a CSV exported from the Whop/Content Rewards dashboard.
  • full analytics  – retention %, RPM, ad revenue. Needs YouTube Analytics OAuth
                      per owned channel (see capture_full_analytics docstring).

Public views + Whop CSV already close the loop on the two numbers that matter
most early: how many views, and how much Whop paid.
"""
from __future__ import annotations

import json
import subprocess
from pathlib import Path

import pandas as pd

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


def _pick(cols: list[str], *candidates: str) -> str | None:
    low = {c.lower().strip(): c for c in cols}
    for cand in candidates:
        if cand in low:
            return low[cand]
    return None


def import_whop_csv(csv_path: str | Path, db_path: Path | None = None) -> int:
    """Attribute Whop payouts to clips from a dashboard CSV export.

    Flexible column detection: matches on a clip id column if present, else on
    the posted clip URL. Recognized payout columns: payout/earnings/amount/paid.
    """
    db.init_db(db_path)
    df = pd.read_csv(csv_path)
    cols = list(df.columns)
    id_col = _pick(cols, "clip_id", "clipid", "id")
    url_col = _pick(cols, "clip_url", "url", "link", "post_url")
    pay_col = _pick(cols, "payout", "earnings", "amount", "paid", "reward")
    view_col = _pick(cols, "views", "view_count")
    if pay_col is None or (id_col is None and url_col is None):
        raise ValueError(
            f"CSV needs a payout column and a clip_id or url column. Saw: {cols}"
        )

    # Known clips + URL->id map, so we only attribute payouts to real clips
    # (skip unknown rows instead of hitting a foreign-key error).
    with connect(db_path) as conn:
        rows = conn.execute("SELECT clip_id, post_url FROM clips").fetchall()
    known = {r["clip_id"] for r in rows}
    url_map = {r["post_url"]: r["clip_id"] for r in rows if r["post_url"]}

    n = skipped = 0
    for _, row in df.iterrows():
        if id_col:
            clip_id = str(row[id_col]).strip() if pd.notna(row[id_col]) else None
        else:
            clip_id = url_map.get(str(row[url_col]).strip()) if pd.notna(row[url_col]) else None
        if not clip_id or clip_id not in known:
            skipped += 1
            continue
        payout = float(row[pay_col]) if pd.notna(row[pay_col]) else 0.0
        metric = {"clip_id": clip_id, "whop_payout": payout}
        if view_col and pd.notna(row[view_col]):
            metric["views"] = int(row[view_col])
        db.insert_metric(metric, db_path)
        n += 1
    if skipped:
        print(f"  · skipped {skipped} CSV row(s) with unknown/blank clip id")
    return n


def capture_full_analytics(db_path: Path | None = None) -> int:
    """Retention %, RPM, ad revenue per owned channel via YouTube Analytics API.

    Requires OAuth per channel (YT_OAUTH_CLIENT_SECRET_JSON in .env). This is a
    deliberate stub — wiring real OAuth is a setup task, not something to fake.
    To enable: install google-api-python-client + google-auth-oauthlib, run the
    one-time consent flow per channel, then query the 'reports' endpoint for
    estimatedRevenue, averageViewPercentage, and views grouped by video.
    """
    raise NotImplementedError(
        "Full analytics needs YouTube Analytics OAuth per channel. "
        "Public views (capture_public) + Whop CSV cover the early loop; "
        "wire OAuth when owned channels approach YPP. See docstring."
    )
