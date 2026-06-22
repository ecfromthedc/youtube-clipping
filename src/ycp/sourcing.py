"""Stage 1 — SOURCE. Build a ranked daily queue of videos worth clipping.

Uses yt-dlp (no API key required) to pull each source creator's recent uploads
with view counts, computes view-velocity (views per hour since publish), ranks,
and writes the top N per creator into the DB. A YouTube Data API key only makes
this faster; it is never required.

Parsing is isolated in `parse_entries` (pure) so ranking logic is unit-tested
without touching the network.
"""
from __future__ import annotations

import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import yaml

from . import db
from .config import ROOT, settings

NICHES_PATH = ROOT / "config" / "niches.yaml"


def _now_epoch() -> float:
    return datetime.now(timezone.utc).timestamp()


def _ytdlp_json(channel_url: str, limit: int) -> list[dict[str, Any]]:
    """Flat-dump a channel's recent uploads. Fast: no per-video download."""
    cmd = [
        "yt-dlp", "--flat-playlist", "--dump-json",
        "--playlist-items", f"1-{limit}", channel_url,
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    if proc.returncode != 0:
        raise RuntimeError(f"yt-dlp failed for {channel_url}: {proc.stderr.strip()[:300]}")
    return [json.loads(line) for line in proc.stdout.splitlines() if line.strip()]


def parse_entries(raw: list[dict[str, Any]], creator: str, lane: str,
                  now_epoch: float | None = None) -> list[dict[str, Any]]:
    """Normalize raw yt-dlp entries -> candidate rows with view_velocity. Pure."""
    now_epoch = now_epoch or _now_epoch()
    out: list[dict[str, Any]] = []
    for e in raw:
        vid = e.get("id")
        if not vid:
            continue
        views = int(e.get("view_count") or 0)
        ts = e.get("timestamp") or e.get("release_timestamp")
        if ts:
            hours = max((now_epoch - float(ts)) / 3600.0, 1.0)
            velocity = round(views / hours, 1)
            published = datetime.fromtimestamp(float(ts), tz=timezone.utc).isoformat()
        else:
            velocity = float(views)  # recency-ordered proxy when no timestamp
            published = None
        out.append({
            "video_id": vid,
            "creator": creator,
            "channel_id": e.get("channel_id"),
            "title": e.get("title"),
            "url": e.get("url") or f"https://www.youtube.com/watch?v={vid}",
            "views": views,
            "published_at": published,
            "view_velocity": velocity,
            "lane": lane,
        })
    return out


def rank(candidates: list[dict[str, Any]], cfg: dict | None = None) -> list[dict[str, Any]]:
    """Filter by min_views and sort by view_velocity desc. Pure."""
    cfg = cfg or settings()["sourcing"]
    keep = [c for c in candidates if c["views"] >= cfg["min_views"]]
    return sorted(keep, key=lambda c: c["view_velocity"], reverse=True)


def load_creators(niches_path: Path | None = None) -> list[dict[str, Any]]:
    path = niches_path or NICHES_PATH
    if not path.exists():
        raise FileNotFoundError(
            f"{path} not found. Create it from config/niches.example.yaml "
            "(or run after niche research)."
        )
    data = yaml.safe_load(path.open())
    creators: list[dict[str, Any]] = []
    for niche in data.get("niches", []):
        for c in niche.get("creators", []):
            creators.append({
                "name": c["name"],
                "url": c["handle"] if c["handle"].startswith("http")
                       else f"https://www.youtube.com/{c['handle']}/videos",
                "lane": c.get("lane", niche.get("lane_default", "whop")),
                "niche": niche["name"],
            })
    return creators


def run(niches_path: Path | None = None, db_path: Path | None = None) -> list[dict[str, Any]]:
    """Source all creators -> write top N each to DB -> return the day's queue."""
    cfg = settings()["sourcing"]
    db.init_db(db_path)
    queue: list[dict[str, Any]] = []
    for creator in load_creators(niches_path):
        try:
            raw = _ytdlp_json(creator["url"], cfg["lookback_days"] * 3)
        except (RuntimeError, subprocess.TimeoutExpired) as exc:
            print(f"  ! skip {creator['name']}: {exc}")
            continue
        candidates = parse_entries(raw, creator["name"], creator["lane"])
        top = rank(candidates, cfg)[: cfg["per_creator"]]
        for row in top:
            db.upsert_source_video(row, db_path)
            queue.append({**row, "niche": creator["niche"]})
    return sorted(queue, key=lambda r: r["view_velocity"], reverse=True)


def render_queue_md(queue: list[dict[str, Any]]) -> str:
    if not queue:
        return "# Daily Source Queue\n\n_(empty — check creator handles in niches.yaml)_\n"
    lines = ["# Daily Source Queue", "", "| velocity/hr | views | lane | creator | title | url |",
             "|---|---|---|---|---|---|"]
    for r in queue:
        title = (r["title"] or "")[:60].replace("|", "/")
        lines.append(
            f"| {r['view_velocity']:,.0f} | {r['views']:,} | {r['lane']} | "
            f"{r['creator']} | {title} | {r['url']} |"
        )
    return "\n".join(lines) + "\n"
