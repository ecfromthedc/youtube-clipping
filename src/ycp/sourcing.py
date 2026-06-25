"""Stage 1 — SOURCE. Build a ranked daily queue of videos worth clipping.

Uses yt-dlp (no API key required) to pull each source creator's recent uploads
with view counts, computes view-velocity (views per hour since publish), ranks,
and writes the top N per creator into the DB. A YouTube Data API key only makes
this faster; it is never required.

Parsing is isolated in `parse_entries` (pure) so ranking logic is unit-tested
without touching the network.
"""
from __future__ import annotations

import subprocess
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import yaml

from . import db, guardrails
from .config import ROOT, settings

NICHES_PATH = ROOT / "config" / "niches.yaml"


def _now_epoch() -> float:
    return datetime.now(timezone.utc).timestamp()


# Tab-delimited per-video metadata. title is LAST so any tabs inside it survive
# `split("\t", 4)`. timestamp falls back to release_timestamp (both epoch seconds).
_META_FORMAT = "%(id)s\t%(view_count)s\t%(timestamp,release_timestamp)s\t%(channel_id)s\t%(title)s"


def _na_int(v: str) -> int | None:
    return None if v in ("NA", "") else int(v)


def _na_float(v: str) -> float | None:
    return None if v in ("NA", "") else float(v)


def _parse_meta_lines(stdout: str) -> list[dict[str, Any]]:
    """Parse `yt-dlp --print _META_FORMAT` output into raw entries. Pure.

    yt-dlp prints the literal string 'NA' for any missing field, so view_count /
    timestamp / channel_id are normalized back to None here.
    """
    rows: list[dict[str, Any]] = []
    for line in stdout.splitlines():
        if not line.strip():
            continue
        parts = line.split("\t", 4)
        if len(parts) < 5:
            continue
        vid, views, ts, channel_id, title = parts
        rows.append({
            "id": vid,
            "view_count": _na_int(views),
            "timestamp": _na_float(ts),
            "channel_id": None if channel_id == "NA" else channel_id,
            "title": title,
        })
    return rows


def _ytdlp_flat_ids(channel_url: str, limit: int) -> list[str]:
    """Fast flat enumerate of a channel's most-recent video IDs (no metadata).

    Flat mode is cheap (one page, no per-video extraction) but reports
    view_count=None on channel /videos tabs — so we use it ONLY to get IDs,
    then re-fetch real view_count/timestamp with `_ytdlp_meta`.
    """
    cmd = [
        "yt-dlp", "--flat-playlist", "--no-warnings", "--print", "%(id)s",
        "--playlist-items", f"1-{limit}", channel_url,
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    if proc.returncode != 0:
        raise RuntimeError(
            f"yt-dlp flat enumerate failed for {channel_url}: {proc.stderr.strip()[:300]}"
        )
    return [ln.strip() for ln in proc.stdout.splitlines() if ln.strip()]


def _ytdlp_meta(video_ids: list[str]) -> list[dict[str, Any]]:
    """Real (non-flat) metadata for specific videos in ONE yt-dlp call.

    This is the fix for the empty-queue bug: flat playlist mode reports
    view_count=None for channel uploads, so velocity ranking dropped everything
    at min_views. We pay the non-flat cost only on the bounded recent-candidate
    set. Partial per-video failures (private/deleted) are skipped, not fatal.
    """
    if not video_ids:
        return []
    urls = [f"https://www.youtube.com/watch?v={vid}" for vid in video_ids]
    cmd = ["yt-dlp", "--no-warnings", "--print", _META_FORMAT, *urls]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
    rows = _parse_meta_lines(proc.stdout)
    if not rows and proc.returncode != 0:
        raise RuntimeError(f"yt-dlp meta fetch failed: {proc.stderr.strip()[:300]}")
    return rows


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
                "lane": c.get("lane", niche.get("lane_default", "owned")),
                "niche": niche["name"],
            })
    return creators


def _source_creator(creator: dict[str, Any], cfg: dict) -> list[dict[str, Any]]:
    """Fetch + rank ONE creator's top videos (network only; no DB write).

    flat enumerate (cheap) → non-flat metadata on the recent candidate set →
    parse → rank → top per_creator. Isolated so creators can run concurrently.
    """
    flat_limit = cfg["lookback_days"] * 3
    meta_limit = cfg.get("meta_fetch", 8)
    ids = _ytdlp_flat_ids(creator["url"], flat_limit)[:meta_limit]
    raw = _ytdlp_meta(ids)
    candidates = parse_entries(raw, creator["name"], creator["lane"])
    # Avoid-list gate (per-video): drop music/casino/licensed-IP titles BEFORE ranking.
    candidates = [c for c in candidates if guardrails.source_allowed(c.get("title", ""))[0]]
    top = rank(candidates, cfg)[: cfg["per_creator"]]
    return [{**row, "niche": creator["niche"]} for row in top]


def run(niches_path: Path | None = None, db_path: Path | None = None) -> list[dict[str, Any]]:
    """Source all creators concurrently -> write top N each to DB -> day's queue.

    Network fetches run in a thread pool (I/O-bound subprocess work, ~Nx faster);
    SQLite writes stay on the main thread to keep the connection single-writer.
    """
    cfg = settings()["sourcing"]
    db.init_db(db_path)
    creators, dropped = guardrails.filter_creators(load_creators(niches_path))
    if dropped:
        print(f"  ⚠ avoid-list gate dropped {len(dropped)}: {', '.join(dropped)}")
    workers = max(1, min(int(cfg.get("concurrency", 8)), len(creators) or 1))
    rows: list[dict[str, Any]] = []
    with ThreadPoolExecutor(max_workers=workers) as ex:
        futures = {ex.submit(_source_creator, c, cfg): c for c in creators}
        for fut in as_completed(futures):
            creator = futures[fut]
            try:
                rows.extend(fut.result())
            except (RuntimeError, subprocess.TimeoutExpired) as exc:
                print(f"  ! skip {creator['name']}: {exc}")
    for row in rows:  # DB writes on the main thread (sqlite single-writer)
        db.upsert_source_video(row, db_path)
    # Double-down: bias the queue by what's winning. optimize.run() writes per-creator
    # multipliers from the scoreboard each cycle; winners rise, losers sink. Default 1.0.
    from . import optimize
    weights = optimize.load_weights()
    return sorted(rows, key=lambda r: r["view_velocity"] * weights.get(r["creator"], 1.0),
                  reverse=True)


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
