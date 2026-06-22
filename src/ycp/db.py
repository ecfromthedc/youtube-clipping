"""SQLite Performance DB — the single source of truth for the closed loop.

Schema (one job each):
  source_videos  – sourcing queue output (what to clip, ranked by velocity)
  clips          – every produced clip and its lifecycle state
  metrics        – daily performance snapshots per clip (views/retention/$$)
  qc_log         – approval decisions (audit trail)
  briefs         – generated weekly Double-Down Briefs

All writes are parameterized. Connections are short-lived; callers use the
`connect()` context manager or the typed helpers below.
"""
from __future__ import annotations

import sqlite3
from contextlib import contextmanager
from collections.abc import Iterator
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import pandas as pd

from .config import DB_PATH, ensure_data_dir

SCHEMA = """
CREATE TABLE IF NOT EXISTS source_videos (
    video_id      TEXT PRIMARY KEY,
    creator       TEXT NOT NULL,
    channel_id    TEXT,
    title         TEXT,
    url           TEXT,
    views         INTEGER DEFAULT 0,
    published_at  TEXT,
    view_velocity REAL DEFAULT 0,     -- views per hour since publish
    lane          TEXT,               -- whop | owned
    status        TEXT DEFAULT 'queued',  -- queued | clipped | skipped
    sourced_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS clips (
    clip_id         TEXT PRIMARY KEY,
    source_video_id TEXT REFERENCES source_videos(video_id),
    source_creator  TEXT,
    channel         TEXT NOT NULL,    -- our posting channel
    platform        TEXT NOT NULL,    -- youtube | tiktok | instagram
    lane            TEXT NOT NULL,    -- whop | owned
    fmt             TEXT,             -- debate-moment | story-payoff | list | reaction ...
    hook_type       TEXT,             -- question | bold-claim | cliffhanger | pattern-interrupt
    length_sec      INTEGER,
    status          TEXT DEFAULT 'pending_qc',  -- pending_qc|approved|rejected|scheduled|posted
    post_url        TEXT,
    posted_at       TEXT,
    slack_ts        TEXT,             -- Slack message ts for QC reaction routing
    created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS metrics (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    clip_id      TEXT NOT NULL REFERENCES clips(clip_id),
    captured_at  TEXT NOT NULL,
    views        INTEGER DEFAULT 0,
    retention_pct REAL,
    swipe_away_pct REAL,
    rpm          REAL,               -- $ per 1000 views (owned/YPP)
    ad_revenue   REAL DEFAULT 0,
    whop_payout  REAL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS qc_log (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    clip_id    TEXT NOT NULL REFERENCES clips(clip_id),
    reviewer   TEXT,
    decision   TEXT NOT NULL,        -- approve | reject
    note       TEXT,
    decided_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS briefs (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    week_start TEXT NOT NULL,
    content    TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_metrics_clip ON metrics(clip_id);
CREATE INDEX IF NOT EXISTS idx_clips_status ON clips(status);
"""


def now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


@contextmanager
def connect(db_path: Path | None = None) -> Iterator[sqlite3.Connection]:
    ensure_data_dir()
    conn = sqlite3.connect(db_path or DB_PATH)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON")
    try:
        yield conn
        conn.commit()
    finally:
        conn.close()


def init_db(db_path: Path | None = None) -> None:
    with connect(db_path) as conn:
        conn.executescript(SCHEMA)


# ── writers ──────────────────────────────────────────────────────────────────

def upsert_source_video(row: dict[str, Any], db_path: Path | None = None) -> None:
    with connect(db_path) as conn:
        conn.execute(
            """INSERT INTO source_videos
                 (video_id, creator, channel_id, title, url, views,
                  published_at, view_velocity, lane, status, sourced_at)
               VALUES (:video_id, :creator, :channel_id, :title, :url, :views,
                       :published_at, :view_velocity, :lane,
                       COALESCE(:status,'queued'), :sourced_at)
               ON CONFLICT(video_id) DO UPDATE SET
                 views=excluded.views,
                 view_velocity=excluded.view_velocity,
                 sourced_at=excluded.sourced_at""",
            {"status": None, **row, "sourced_at": now()},
        )


def insert_clip(row: dict[str, Any], db_path: Path | None = None) -> None:
    with connect(db_path) as conn:
        conn.execute(
            """INSERT INTO clips
                 (clip_id, source_video_id, source_creator, channel, platform,
                  lane, fmt, hook_type, length_sec, status, post_url, posted_at,
                  slack_ts, created_at)
               VALUES (:clip_id, :source_video_id, :source_creator, :channel,
                       :platform, :lane, :fmt, :hook_type, :length_sec,
                       COALESCE(:status,'pending_qc'), :post_url, :posted_at,
                       :slack_ts, :created_at)
               ON CONFLICT(clip_id) DO NOTHING""",
            {
                "status": None, "source_video_id": None, "source_creator": None,
                "post_url": None, "posted_at": None, "slack_ts": None,
                "created_at": now(), **row,
            },
        )


def set_clip_status(clip_id: str, status: str, db_path: Path | None = None,
                    **fields: Any) -> None:
    sets = ["status = ?"]
    vals: list[Any] = [status]
    for k, v in fields.items():
        sets.append(f"{k} = ?")
        vals.append(v)
    vals.append(clip_id)
    with connect(db_path) as conn:
        conn.execute(f"UPDATE clips SET {', '.join(sets)} WHERE clip_id = ?", vals)


def record_qc(clip_id: str, decision: str, reviewer: str | None = None,
              note: str | None = None, db_path: Path | None = None) -> None:
    with connect(db_path) as conn:
        conn.execute(
            """INSERT INTO qc_log (clip_id, reviewer, decision, note, decided_at)
               VALUES (?, ?, ?, ?, ?)""",
            (clip_id, reviewer, decision, note, now()),
        )
    set_clip_status(clip_id, "approved" if decision == "approve" else "rejected",
                    db_path=db_path)


def insert_metric(row: dict[str, Any], db_path: Path | None = None) -> None:
    with connect(db_path) as conn:
        conn.execute(
            """INSERT INTO metrics
                 (clip_id, captured_at, views, retention_pct, swipe_away_pct,
                  rpm, ad_revenue, whop_payout)
               VALUES (:clip_id, :captured_at, :views, :retention_pct,
                       :swipe_away_pct, :rpm, :ad_revenue, :whop_payout)""",
            {
                "captured_at": now(), "retention_pct": None, "swipe_away_pct": None,
                "rpm": None, "ad_revenue": 0, "whop_payout": 0, "views": 0, **row,
            },
        )


def save_brief(week_start: str, content: str, db_path: Path | None = None) -> None:
    with connect(db_path) as conn:
        conn.execute(
            "INSERT INTO briefs (week_start, content, created_at) VALUES (?, ?, ?)",
            (week_start, content, now()),
        )


def clip_by_slack_ts(slack_ts: str, db_path: Path | None = None) -> str | None:
    with connect(db_path) as conn:
        cur = conn.execute("SELECT clip_id FROM clips WHERE slack_ts = ?", (slack_ts,))
        row = cur.fetchone()
        return row["clip_id"] if row else None


def pending_qc_clips(db_path: Path | None = None) -> list[dict[str, Any]]:
    with connect(db_path) as conn:
        cur = conn.execute("SELECT * FROM clips WHERE status = 'pending_qc'")
        return [dict(r) for r in cur.fetchall()]


# ── readers (pandas, for scoring) ────────────────────────────────────────────

def clips_with_latest_metrics(db_path: Path | None = None) -> pd.DataFrame:
    """One row per clip joined to its most recent metrics snapshot."""
    with connect(db_path) as conn:
        clips = pd.read_sql_query("SELECT * FROM clips", conn)
        metrics = pd.read_sql_query("SELECT * FROM metrics", conn)
    if clips.empty:
        return clips
    if metrics.empty:
        for col in ("views", "retention_pct", "rpm", "ad_revenue", "whop_payout"):
            clips[col] = 0.0
        return clips
    metrics = metrics.sort_values("captured_at").groupby("clip_id").tail(1)
    merged = clips.merge(
        metrics[["clip_id", "views", "retention_pct", "rpm", "ad_revenue", "whop_payout"]],
        on="clip_id", how="left",
    )
    return merged.fillna({"views": 0, "ad_revenue": 0, "whop_payout": 0})
