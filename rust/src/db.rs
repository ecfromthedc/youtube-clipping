//! SQLite layer — schema, migrations, models, and queries. Mirrors src/ycp/db.py.
//! Same on-disk database as the Python version (data/clips.db), so they interoperate
//! during the port.
use std::path::Path;

use anyhow::Result;
use rusqlite::{params, Connection};

pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS source_videos (
    video_id      TEXT PRIMARY KEY,
    creator       TEXT NOT NULL,
    channel_id    TEXT,
    title         TEXT,
    url           TEXT NOT NULL,
    views         INTEGER,
    published_at  TEXT,
    view_velocity REAL,
    lane          TEXT NOT NULL,
    status        TEXT DEFAULT 'queued',
    sourced_at    TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS clips (
    clip_id         TEXT PRIMARY KEY,
    source_video_id TEXT,
    source_creator  TEXT,
    channel         TEXT NOT NULL,
    platform        TEXT NOT NULL,
    lane            TEXT NOT NULL,
    fmt             TEXT,
    hook_type       TEXT,
    length_sec      INTEGER,
    status          TEXT DEFAULT 'pending_qc',
    post_title      TEXT,
    post_id         TEXT,
    experiment_id   TEXT,
    variant         TEXT,
    post_url        TEXT,
    posted_at       TEXT,
    slack_ts        TEXT,
    created_at      TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS metrics (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    clip_id       TEXT NOT NULL,
    captured_at   TEXT NOT NULL,
    views         INTEGER DEFAULT 0,
    retention_pct REAL,
    swipe_away_pct REAL,
    rpm           REAL,
    ad_revenue    REAL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS qc_log (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    clip_id    TEXT NOT NULL,
    reviewer   TEXT,
    decision   TEXT NOT NULL,
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
"#;

/// A clip joined to its most recent metrics snapshot (mirrors clips_with_latest_metrics).
#[derive(Debug, Clone, Default)]
pub struct ClipRow {
    pub clip_id: String,
    pub source_creator: Option<String>,
    pub channel: Option<String>,
    pub platform: Option<String>,
    pub fmt: Option<String>,
    pub hook_type: Option<String>,
    pub length_sec: Option<i64>,
    pub status: Option<String>,
    pub post_title: Option<String>,
    pub post_id: Option<String>,
    pub experiment_id: Option<String>,
    pub variant: Option<String>,
    pub post_url: Option<String>,
    pub posted_at: Option<String>,
    pub views: i64,
    pub retention_pct: Option<f64>,
    pub swipe_away_pct: Option<f64>,
    pub ad_revenue: f64,
}

pub fn now() -> String {
    // UTC, second precision (matches the Python isoformat(timespec="seconds")).
    crate::util::utc_now_iso()
}

pub fn open(path: &Path) -> Result<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON")?;
    init(&conn)?;
    Ok(conn)
}

pub fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA)?;
    // Idempotent migrations for columns added after a db already existed.
    for col in ["post_title TEXT", "post_id TEXT", "experiment_id TEXT", "variant TEXT"] {
        let _ = conn.execute(&format!("ALTER TABLE clips ADD COLUMN {col}"), []);
    }
    Ok(())
}

const CLIP_SELECT: &str = "
    SELECT c.clip_id, c.source_creator, c.channel, c.platform, c.fmt, c.hook_type,
           c.length_sec, c.status, c.post_title, c.post_id, c.experiment_id, c.variant,
           c.post_url, c.posted_at,
           COALESCE(m.views, 0), m.retention_pct, m.swipe_away_pct, COALESCE(m.ad_revenue, 0)
    FROM clips c
    LEFT JOIN (
        SELECT t.* FROM metrics t
        JOIN (SELECT clip_id, MAX(id) mid FROM metrics GROUP BY clip_id) latest
          ON t.id = latest.mid
    ) m ON m.clip_id = c.clip_id";

fn row_to_clip(r: &rusqlite::Row) -> rusqlite::Result<ClipRow> {
    Ok(ClipRow {
        clip_id: r.get(0)?,
        source_creator: r.get(1)?,
        channel: r.get(2)?,
        platform: r.get(3)?,
        fmt: r.get(4)?,
        hook_type: r.get(5)?,
        length_sec: r.get(6)?,
        status: r.get(7)?,
        post_title: r.get(8)?,
        post_id: r.get(9)?,
        experiment_id: r.get(10)?,
        variant: r.get(11)?,
        post_url: r.get(12)?,
        posted_at: r.get(13)?,
        views: r.get(14)?,
        retention_pct: r.get(15)?,
        swipe_away_pct: r.get(16)?,
        ad_revenue: r.get(17)?,
    })
}

/// One row per clip joined to its latest metrics snapshot.
pub fn clips_with_latest_metrics(conn: &Connection) -> Result<Vec<ClipRow>> {
    let mut stmt = conn.prepare(CLIP_SELECT)?;
    let rows = stmt.query_map([], row_to_clip)?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Clips approved by QC and not yet posted — the distribution work queue.
/// Parity with db.py `approved_clips`: status = 'approved' only.
pub fn approved_clips(conn: &Connection) -> Result<Vec<ClipRow>> {
    clips_by_status(conn, "approved")
}

/// Clips awaiting manual/auto QC (mirrors db.py `pending_qc_clips`).
pub fn pending_qc_clips(conn: &Connection) -> Result<Vec<ClipRow>> {
    clips_by_status(conn, "pending_qc")
}

/// All clips in one status, joined to latest metrics. (Python does `SELECT *`; the metrics
/// join is harmless here — consumers read only clip columns.)
fn clips_by_status(conn: &Connection, status: &str) -> Result<Vec<ClipRow>> {
    let sql = format!("{CLIP_SELECT} WHERE c.status = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([status], row_to_clip)?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Update a clip's status, optionally setting extra TEXT columns (post_url, posted_at,
/// post_id). Field names are static identifiers, not user input. Mirrors db.py `set_clip_status`.
pub fn set_clip_status(
    conn: &Connection,
    clip_id: &str,
    status: &str,
    fields: &[(&str, &str)],
) -> Result<()> {
    let mut sets = String::from("status = ?");
    let mut vals: Vec<&str> = vec![status];
    for (k, v) in fields {
        sets.push_str(&format!(", {k} = ?"));
        vals.push(v);
    }
    vals.push(clip_id);
    let sql = format!("UPDATE clips SET {sets} WHERE clip_id = ?");
    conn.execute(&sql, rusqlite::params_from_iter(vals))?;
    Ok(())
}

/// Log a QC decision and flip the clip's status accordingly (mirrors db.py `record_qc`).
pub fn record_qc(
    conn: &Connection,
    clip_id: &str,
    decision: &str,
    reviewer: Option<&str>,
    note: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO qc_log (clip_id, reviewer, decision, note, decided_at) VALUES (?1,?2,?3,?4,?5)",
        params![clip_id, reviewer, decision, note, now()],
    )?;
    let status = if decision == "approve" { "approved" } else { "rejected" };
    set_clip_status(conn, clip_id, status, &[])
}

/// A sourcing-queue row (mirrors the dict `upsert_source_video` writes). status=None → 'queued'.
#[derive(Debug, Clone, Default)]
pub struct SourceVideoRow {
    pub video_id: String,
    pub creator: String,
    pub channel_id: Option<String>,
    pub title: Option<String>,
    pub url: String,
    pub views: i64,
    pub published_at: Option<String>,
    pub view_velocity: f64,
    pub lane: String,
    pub status: Option<String>,
}

/// Insert/refresh a sourced video (mirrors db.py `upsert_source_video`): on conflict only
/// views/view_velocity/sourced_at are refreshed, exactly like the Python ON CONFLICT clause.
pub fn upsert_source_video(conn: &Connection, r: &SourceVideoRow) -> Result<()> {
    conn.execute(
        "INSERT INTO source_videos
             (video_id, creator, channel_id, title, url, views, published_at,
              view_velocity, lane, status, sourced_at)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, COALESCE(?10, 'queued'), ?11)
           ON CONFLICT(video_id) DO UPDATE SET
             views=excluded.views,
             view_velocity=excluded.view_velocity,
             sourced_at=excluded.sourced_at",
        params![
            r.video_id, r.creator, r.channel_id, r.title, r.url, r.views,
            r.published_at, r.view_velocity, r.lane, r.status, now()
        ],
    )?;
    Ok(())
}

/// One performance snapshot for a clip (mirrors the db.py `insert_metric` row dict).
#[derive(Debug, Clone, Default)]
pub struct MetricRow {
    pub clip_id: String,
    pub views: i64,
    pub retention_pct: Option<f64>,
    pub swipe_away_pct: Option<f64>,
    pub rpm: Option<f64>,
    pub ad_revenue: f64,
}

pub fn insert_metric(conn: &Connection, m: &MetricRow) -> Result<()> {
    conn.execute(
        "INSERT INTO metrics (clip_id, captured_at, views, retention_pct, swipe_away_pct, rpm, ad_revenue)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![m.clip_id, now(), m.views, m.retention_pct, m.swipe_away_pct, m.rpm, m.ad_revenue],
    )?;
    Ok(())
}
