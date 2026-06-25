//! Stage 5 (capture half) — pull performance into the DB. Parity port of `src/ycp/capture.py`.
//!
//! Two sources, in order of how much they need:
//!   • public views   – yt-dlp on each posted clip URL. No creds. Works today.
//!   • full analytics  – retention %, RPM, ad revenue. Needs YouTube Analytics OAuth.
//!
//! The OAuth + Postiz + yt-dlp paths shell/REST out exactly as the Python does; they're
//! no-ops without creds (returns 0). The pure `analyze_retention` + `video_id` are cross-checked.
#![allow(dead_code)] // consumed by the autopilot orchestrator (last port row)

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::Result;
use chrono::Duration as ChronoDuration;
use rusqlite::Connection;
use serde_json::Value;

use crate::db::{self, MetricRow};
use crate::{config, util};

const YT_WATCH: &str = "youtube.com/watch?v=";
const HOOK_WINDOW: f64 = 0.2; // first 20% of a Short ≈ the hook window

/// Current public view count for one video URL (YouTube/TikTok/IG). Shells yt-dlp.
pub fn ytdlp_views(url: &str) -> Option<i64> {
    let out = Command::new("yt-dlp")
        .args(["--dump-json", "--skip-download", url])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let v: Value = serde_json::from_slice(&out.stdout).ok()?;
    // Python: int(json.get("view_count") or 0)
    Some(v.get("view_count").and_then(Value::as_i64).unwrap_or(0))
}

/// Resolve posted clips' Postiz post_id → the published YouTube URL (releaseURL). Safe no-op
/// without a Postiz token. Returns how many clips got resolved. Mirrors `resolve_published`.
pub fn resolve_published(conn: &Connection, root: &Path) -> Result<i64> {
    let token = match config::env_var(root, "POSTIZ_API_TOKEN") {
        Some(t) => t,
        None => return Ok(0),
    };
    let pending: Vec<(String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT clip_id, post_id FROM clips WHERE status='posted' AND post_id IS NOT NULL \
             AND (post_url IS NULL OR post_url NOT LIKE '%youtube.com/watch%')",
        )?;
        let v = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
        v
    };
    if pending.is_empty() {
        return Ok(0);
    }
    let settings = config::load_settings(root)?;
    let api = settings["distribution"]["postiz"]["api_url"].as_str().unwrap_or("").trim_end_matches('/').to_string();
    let today = chrono::Utc::now().date_naive();
    let start = format!("{}T00:00:00Z", today - ChronoDuration::days(10));
    let end = format!("{}T00:00:00Z", today + ChronoDuration::days(2));
    let client = match reqwest::blocking::Client::builder().timeout(Duration::from_secs(30)).build() {
        Ok(c) => c,
        Err(_) => return Ok(0),
    };
    let data: Value = match client
        .get(format!("{api}/posts"))
        .header("Authorization", &token)
        .query(&[("startDate", &start), ("endDate", &end)])
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
    {
        Ok(d) => d,
        Err(_) => return Ok(0),
    };
    let posts: Vec<Value> = if let Some(arr) = data.as_array() {
        arr.clone()
    } else {
        data.get("posts")
            .or_else(|| data.get("data"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    // id → (releaseURL, publishDate) for PUBLISHED posts that carry a releaseURL.
    let mut info: HashMap<String, (String, Option<String>)> = HashMap::new();
    for p in &posts {
        if p.get("state").and_then(Value::as_str) != Some("PUBLISHED") {
            continue;
        }
        let url = match p.get("releaseURL").and_then(Value::as_str).filter(|u| !u.is_empty()) {
            Some(u) => u,
            None => continue,
        };
        if let Some(id) = p.get("id").and_then(Value::as_str) {
            let published = p.get("publishDate").and_then(Value::as_str).map(String::from);
            info.insert(id.to_string(), (url.to_string(), published));
        }
    }
    let mut n = 0;
    for (clip_id, post_id) in pending {
        if let Some((url, published)) = info.get(&post_id) {
            // Python: published or db.now()
            let posted_at = published.as_deref().filter(|s| !s.is_empty()).map(String::from).unwrap_or_else(db::now);
            db::set_clip_status(
                conn,
                &clip_id,
                "posted",
                &[("post_url", url.as_str()), ("posted_at", posted_at.as_str())],
            )?;
            n += 1;
        }
    }
    Ok(n)
}

/// Extract a YouTube video id from a watch URL (else None). Mirrors `_video_id`.
pub fn video_id(url: Option<&str>) -> Option<String> {
    let url = url?;
    if url.contains(YT_WATCH) {
        let after = url.split("watch?v=").nth(1)?;
        return Some(after.split('&').next().unwrap_or(after).to_string());
    }
    None
}

/// Snapshot public views for every posted clip that has a URL. Returns count.
pub fn capture_public(conn: &Connection, root: &Path) -> Result<i64> {
    capture_public_with(conn, root, ytdlp_views)
}

/// `capture_public` with an injectable view fetcher (the seam the Python tests monkeypatch).
pub fn capture_public_with(
    conn: &Connection,
    root: &Path,
    fetch: impl Fn(&str) -> Option<i64>,
) -> Result<i64> {
    resolve_published(conn, root)?; // turn Postiz post_ids into YouTube URLs first
    let rows: Vec<(String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT clip_id, post_url FROM clips WHERE status = 'posted' \
             AND post_url LIKE '%youtube.com/watch%'",
        )?;
        let v = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
        v
    };
    let mut n = 0;
    for (clip_id, post_url) in rows {
        match fetch(&post_url) {
            None => println!("  ! no views for {clip_id} ({post_url})"),
            Some(views) => {
                db::insert_metric(conn, &MetricRow { clip_id, views, ..Default::default() })?;
                n += 1;
            }
        }
    }
    Ok(n)
}

/// Hook/drop-off signals from an audience-retention curve [(elapsedRatio, watchRatio), ...].
/// Pure. None if the curve is unreadably short. Mirrors `analyze_retention`.
#[derive(Debug, Clone, PartialEq)]
pub struct RetentionSignals {
    pub hook_retention: f64,
    pub swipe_away_pct: f64,
    pub biggest_drop_pct: f64,
    pub biggest_drop_at: f64,
}

pub fn analyze_retention(curve: &[(f64, f64)]) -> Option<RetentionSignals> {
    let mut pts: Vec<(f64, f64)> = curve.to_vec();
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.partial_cmp(&b.1).unwrap()));
    if pts.len() < 3 {
        return None;
    }
    // hook_ret = last watch-ratio within the hook window, else the very first point.
    let hook_ret =
        pts.iter().filter(|(e, _)| *e <= HOOK_WINDOW).map(|(_, w)| *w).next_back().unwrap_or(pts[0].1);
    // biggest single drop; Python `max(..., key=drop)` keeps the FIRST on ties.
    let mut biggest_drop = pts[0].1 - pts[1].1;
    let mut at = pts[1].0;
    for i in 1..pts.len() - 1 {
        let drop = pts[i].1 - pts[i + 1].1;
        if drop > biggest_drop {
            biggest_drop = drop;
            at = pts[i + 1].0;
        }
    }
    Some(RetentionSignals {
        hook_retention: util::round_to(hook_ret, 3),
        swipe_away_pct: util::round_to((1.0 - hook_ret).max(0.0) * 100.0, 1),
        biggest_drop_pct: util::round_to(biggest_drop.max(0.0) * 100.0, 1),
        biggest_drop_at: util::round_to(at, 2),
    })
}

// ── full analytics (YouTube Analytics OAuth — no-op without creds) ─────────────

/// Refresh a YouTube Analytics access token from .env creds (mirrors `_yt_creds` + the
/// implicit googleapiclient refresh). None when creds are absent or refresh fails.
fn yt_access_token(root: &Path) -> Option<String> {
    let cid = config::env_var(root, "YT_CLIENT_ID")?;
    let secret = config::env_var(root, "YT_CLIENT_SECRET")?;
    let refresh = config::env_var(root, "YT_REFRESH_TOKEN")?;
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(30)).build().ok()?;
    let resp = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", cid.as_str()),
            ("client_secret", secret.as_str()),
            ("refresh_token", refresh.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .ok()?
        .error_for_status()
        .ok()?;
    let v: Value = resp.json().ok()?;
    v.get("access_token").and_then(Value::as_str).map(String::from)
}

fn yt_report(
    client: &reqwest::blocking::Client,
    token: &str,
    start: &str,
    end: &str,
    metrics: &str,
    dimensions: &str,
    filters: &str,
) -> Option<Value> {
    client
        .get("https://youtubeanalytics.googleapis.com/v2/reports")
        .header("Authorization", format!("Bearer {token}"))
        .query(&[
            ("ids", "channel==MINE"),
            ("startDate", start),
            ("endDate", end),
            ("metrics", metrics),
            ("dimensions", dimensions),
            ("filters", filters),
        ])
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .ok()
}

fn fetch_retention(
    client: &reqwest::blocking::Client,
    token: &str,
    vid: &str,
    start: &str,
    end: &str,
) -> Option<Vec<(f64, f64)>> {
    let rep = yt_report(
        client,
        token,
        start,
        end,
        "audienceWatchRatio",
        "elapsedVideoTimeRatio",
        &format!("video=={vid}"),
    )?;
    let rows = rep.get("rows").and_then(Value::as_array)?;
    if rows.is_empty() {
        return None;
    }
    Some(
        rows.iter()
            .filter_map(|r| {
                let a = r.as_array()?;
                Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
            })
            .collect(),
    )
}

/// Per-clip retention % + ad revenue from the YouTube Analytics API. Safe no-op without creds.
/// Returns updates. Mirrors `capture_full_analytics`.
pub fn capture_full_analytics(conn: &Connection, root: &Path) -> Result<i64> {
    let token = match yt_access_token(root) {
        Some(t) => t,
        None => return Ok(0),
    };
    resolve_published(conn, root)?;
    let rows: Vec<(String, String, Option<String>)> = {
        let mut stmt = conn.prepare(
            "SELECT clip_id, post_url, posted_at FROM clips WHERE status='posted' \
             AND post_url LIKE '%youtube.com/watch%'",
        )?;
        let v = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<rusqlite::Result<_>>()?;
        v
    };
    if rows.is_empty() {
        return Ok(0);
    }
    let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(30)).build()?;
    let today = util::today_iso();
    let mut n = 0;
    for (clip_id, post_url, posted_at) in rows {
        let vid = match video_id(Some(&post_url)) {
            Some(v) => v,
            None => continue,
        };
        let start_owned = posted_at.as_deref().filter(|s| !s.is_empty()).unwrap_or(&today).to_string();
        let start = &start_owned[..start_owned.len().min(10)]; // [:10]
        let report = match yt_report(
            &client,
            &token,
            start,
            &today,
            "views,averageViewPercentage,estimatedRevenue",
            "video",
            &format!("video=={vid}"),
        ) {
            Some(r) => r,
            None => continue,
        };
        let srows = match report.get("rows").and_then(Value::as_array) {
            Some(s) if !s.is_empty() => s.clone(),
            _ => continue, // no owned data yet — don't overwrite the public snapshot with zeros
        };
        let row = &srows[0]; // [video, views, avgViewPct, estRevenue]
        let ret = analyze_retention(&fetch_retention(&client, &token, &vid, start, &today).unwrap_or_default());
        db::insert_metric(
            conn,
            &MetricRow {
                clip_id,
                views: num_at(row, 1).map(|f| f as i64).unwrap_or(0),
                retention_pct: Some(num_at(row, 2).unwrap_or(0.0)),
                ad_revenue: num_at(row, 3).unwrap_or(0.0),
                swipe_away_pct: ret.map(|r| r.swipe_away_pct),
                ..Default::default()
            },
        )?;
        n += 1;
    }
    Ok(n)
}

/// A numeric cell from an analytics row (the API may send numbers as strings).
fn num_at(row: &Value, i: usize) -> Option<f64> {
    let cell = row.get(i)?;
    cell.as_f64().or_else(|| cell.as_str().and_then(|s| s.parse().ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_retention_reads_hook_dropoff() {
        // Mirrors test_capture.py: 40% gone by the hook's end; cliff at the 10% mark.
        let curve = [(0.0, 1.0), (0.1, 0.6), (0.2, 0.6), (0.5, 0.5), (1.0, 0.4)];
        let r = analyze_retention(&curve).unwrap();
        assert_eq!(r.swipe_away_pct, 40.0);
        assert_eq!(r.biggest_drop_at, 0.1);
        assert_eq!(r.biggest_drop_pct, 40.0);
    }

    #[test]
    fn analyze_retention_none_for_short_curve() {
        assert!(analyze_retention(&[(0.0, 1.0)]).is_none());
    }

    #[test]
    fn video_id_extracts_watch_param() {
        assert_eq!(video_id(Some("https://www.youtube.com/watch?v=abc123&t=5")).as_deref(), Some("abc123"));
        assert_eq!(video_id(Some("https://tiktok.com/@x/video/1")), None);
        assert_eq!(video_id(None), None);
    }

    #[test]
    fn capture_public_snapshots_only_clips_with_urls() {
        // Mirrors test_capture.py::test_capture_public_snapshots_posted_clips.
        let conn = Connection::open_in_memory().unwrap();
        db::init(&conn).unwrap();
        conn.execute(
            "INSERT INTO clips (clip_id, channel, platform, lane, status, post_url, created_at)
             VALUES ('c1','ch','youtube','owned','posted','https://www.youtube.com/watch?v=abc123','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO clips (clip_id, channel, platform, lane, status, created_at)
             VALUES ('c2','ch','youtube','owned','posted','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        // resolve_published is a no-op here (no POSTIZ token / no pending post_ids).
        let n = capture_public_with(&conn, Path::new("/nonexistent-ycp"), |_| Some(4321)).unwrap();
        assert_eq!(n, 1); // only the clip with a post_url
        let views: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(views),0) FROM metrics WHERE clip_id='c1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(views, 4321);
        let c2: i64 = conn.query_row("SELECT COUNT(*) FROM metrics WHERE clip_id='c2'", [], |r| r.get(0)).unwrap();
        assert_eq!(c2, 0); // no URL → never captured
    }

    #[test]
    fn capture_public_skips_when_no_views() {
        let conn = Connection::open_in_memory().unwrap();
        db::init(&conn).unwrap();
        conn.execute(
            "INSERT INTO clips (clip_id, channel, platform, lane, status, post_url, created_at)
             VALUES ('c1','ch','youtube','owned','posted','https://www.youtube.com/watch?v=abc123','2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
        let n = capture_public_with(&conn, Path::new("/nonexistent-ycp"), |_| None).unwrap();
        assert_eq!(n, 0); // no metric written when views can't be fetched
    }
}
