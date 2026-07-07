//! Analytics surface — channel rollups + per-video metrics from YouTube Analytics.
//!
//! Reuses the existing OAuth path (capture::yt_access_token + capture::yt_report) — no new
//! auth infrastructure. Wraps it in 1h caching (YouTube Analytics has tight quotas) and
//! shapes the output for the editor's /analytics dashboard.
//!
//! The loop this closes: render → publish → MEASURE → tune → render the next one.
//! Kellan's entire tuning process is reading these numbers and adjusting the next video.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use serde_json::{json, Value};

use crate::{capture, config};

const CACHE_TTL_SECS: u64 = 3600; // 1h — YT Analytics quotas are tight

/// Where the cache lives.
fn cache_path(root: &Path) -> PathBuf {
    config::data_dir(root).join("analytics-cache.json")
}

/// Read the cached payload for `key` if it's < 1h old.
fn read_cache(root: &Path, key: &str) -> Option<Value> {
    let path = cache_path(root);
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
    let entry = v.get(key)?;
    let cached_at = entry.get("cached_at").and_then(Value::as_f64)?;
    let age = now_secs() - cached_at;
    if age > CACHE_TTL_SECS as f64 {
        return None;
    }
    entry.get("data").cloned()
}

fn write_cache(root: &Path, key: &str, data: &Value) {
    let path = cache_path(root);
    let mut full: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}));
    if !full.is_object() {
        full = json!({});
    }
    if let Some(map) = full.as_object_mut() {
        map.insert(
            key.to_string(),
            json!({ "cached_at": now_secs(), "data": data }),
        );
    }
    if let Some(p) = path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    let _ = std::fs::write(
        &path,
        serde_json::to_string_pretty(&full).unwrap_or_default(),
    );
}

fn now_secs() -> f64 {
    chrono::Utc::now().timestamp() as f64
}

fn http_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("reqwest client")
}

/// Is YouTube Analytics OAuth configured? Probes by attempting a token refresh
/// (connected-channels store first, .env single-channel fallback).
pub fn configured(root: &Path) -> bool {
    capture::yt_access_token_for(root, None).is_some()
}

/// Cache-key segment isolating each channel's numbers (multi-channel toggle).
fn chan_key(channel: Option<&str>) -> &str {
    channel.unwrap_or("env")
}

/// Channel rollup: total views, subs, est revenue, avg retention, avg swipe, top video.
/// Returns a JSON object shaped for the dashboard. Empty result on any failure.
pub fn channel_rollup(root: &Path, days: u32, channel: Option<&str>) -> Result<Value> {
    let cache_key = format!("rollup_{days}_{}", chan_key(channel));
    if let Some(c) = read_cache(root, &cache_key) {
        return Ok(c);
    }
    let token = match capture::yt_access_token_for(root, channel) {
        Some(t) => t,
        None => return Ok(json!({ "configured": false })),
    };
    let client = http_client();
    let today = chrono::Utc::now().date_naive();
    let start = format!("{}", today - chrono::Duration::days(days as i64));
    let end = format!("{}", today);

    // Single batched query for views/watchMinutes/subsGained/estimatedRevenue.
    let totals = capture::yt_report(
        &client,
        &token,
        &start,
        &end,
        "views,averagePercentageWatched,subscribersGained,estimatedRevenue,estimatedAdRevenue",
        "", // no dimension → totals row
        "",
    )
    .unwrap_or_else(|| json!({ "rows": [[0, 0, 0, 0, 0]] }));
    let row = totals
        .get("rows")
        .and_then(Value::as_array)
        .and_then(|r| r.first());
    let num = |i: usize| {
        row.and_then(|r| r.get(i))
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    };

    let rollup = json!({
        "configured": true,
        "range_days": days,
        "start": start,
        "end": end,
        "views": num(0) as i64,
        "avg_watch_pct": num(1),
        "subs_gained": num(2) as i64,
        "est_revenue": num(3),
        "est_ad_revenue": num(4),
        "hook_retention_pct": num(1), // approx — proper hook retention needs the curve
    });
    write_cache(root, &cache_key, &rollup);
    Ok(rollup)
}

/// Top-N videos by views over the window, with per-video metrics + health classification.
pub fn top_videos(root: &Path, days: u32, limit: usize, channel: Option<&str>) -> Result<Value> {
    let cache_key = format!("top_{days}_{limit}_{}", chan_key(channel));
    if let Some(c) = read_cache(root, &cache_key) {
        return Ok(c);
    }
    let token = match capture::yt_access_token_for(root, channel) {
        Some(t) => t,
        None => return Ok(json!({ "configured": false, "videos": [] })),
    };
    let client = http_client();
    let today = chrono::Utc::now().date_naive();
    let start = format!("{}", today - chrono::Duration::days(days as i64));
    let end = format!("{}", today);

    let rep = capture::yt_report(
        &client,
        &token,
        &start,
        &end,
        "views,averagePercentageWatched,subscribersGained,estimatedRevenue",
        "video",
        "",
    )
    .unwrap_or_else(|| json!({ "rows": [] }));

    let cols = rep
        .get("columnHeaders")
        .and_then(Value::as_array)
        .map(|h| {
            h.iter()
                .map(|x| x["name"].as_str().unwrap_or("").to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut videos: Vec<Value> = rep
        .get("rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| {
                    let arr = r.as_array()?;
                    let mut obj = serde_json::Map::new();
                    for (i, v) in arr.iter().enumerate() {
                        let key = cols.get(i).cloned().unwrap_or_else(|| format!("col{i}"));
                        obj.insert(key, v.clone());
                    }
                    // health classification — green if avg watch > 70%
                    let watch = obj
                        .get("averagePercentageWatched")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    let health = if watch >= 70.0 {
                        "green"
                    } else if watch >= 50.0 {
                        "yellow"
                    } else {
                        "red"
                    };
                    obj.insert("health".to_string(), json!(health));
                    Some(Value::Object(obj))
                })
                .collect()
        })
        .unwrap_or_default();

    // Sort by views desc, take top N.
    videos.sort_by(|a, b| {
        let av = a.get("views").and_then(Value::as_i64).unwrap_or(0);
        let bv = b.get("views").and_then(Value::as_i64).unwrap_or(0);
        bv.cmp(&av)
    });
    videos.truncate(limit);

    // Decorate each with its video URL.
    for v in videos.iter_mut() {
        if let Some(vid) = v.get("video").and_then(Value::as_str) {
            v["url"] = json!(format!("https://youtube.com/watch?v={vid}"));
        }
    }

    let out = json!({ "configured": true, "videos": videos, "range_days": days });
    write_cache(root, &cache_key, &out);
    Ok(out)
}

/// 7-day daily views + revenue sparkline. Returns `{dates: [], views: [], revenue: []}`.
pub fn daily_series(root: &Path, days: u32, channel: Option<&str>) -> Result<Value> {
    let cache_key = format!("daily_{days}_{}", chan_key(channel));
    if let Some(c) = read_cache(root, &cache_key) {
        return Ok(c);
    }
    let token = match capture::yt_access_token_for(root, channel) {
        Some(t) => t,
        None => return Ok(json!({ "configured": false, "dates": [], "views": [], "revenue": [] })),
    };
    let client = http_client();
    let today = chrono::Utc::now().date_naive();
    let start = format!("{}", today - chrono::Duration::days(days as i64));
    let end = format!("{}", today);
    let rep = capture::yt_report(
        &client,
        &token,
        &start,
        &end,
        "views,estimatedRevenue",
        "day",
        "",
    )
    .unwrap_or_else(|| json!({ "rows": [] }));

    let mut dates = Vec::new();
    let mut views = Vec::new();
    let mut revenue = Vec::new();
    if let Some(rows) = rep.get("rows").and_then(Value::as_array) {
        for r in rows {
            let arr = match r.as_array() {
                Some(a) => a,
                None => continue,
            };
            dates.push(arr.first().cloned().unwrap_or(json!("")));
            views.push(arr.get(1).and_then(Value::as_i64).unwrap_or(0));
            revenue.push(arr.get(2).and_then(Value::as_f64).unwrap_or(0.0));
        }
    }
    let out = json!({ "configured": true, "dates": dates, "views": views, "revenue": revenue });
    write_cache(root, &cache_key, &out);
    Ok(out)
}

/// Per-video retention curve for the retention popover. `vid` is the YouTube video id.
pub fn retention_curve(root: &Path, vid: &str, channel: Option<&str>) -> Result<Value> {
    let token = match capture::yt_access_token_for(root, channel) {
        Some(t) => t,
        None => return Ok(json!({ "configured": false, "curve": [] })),
    };
    let client = http_client();
    let today = chrono::Utc::now().date_naive();
    let start = format!("{}", today - chrono::Duration::days(28));
    let end = format!("{}", today);
    let curve = capture::fetch_retention(&client, &token, vid, &start, &end).unwrap_or_default();
    let points: Vec<Value> = curve.iter().map(|(t, w)| json!([t, w])).collect();
    let signals = capture::analyze_retention(&curve);
    Ok(json!({
        "configured": true,
        "video_id": vid,
        "curve": points,
        "signals": signals.map(|s| json!({
            "hook_retention": s.hook_retention,
            "swipe_away_pct": s.swipe_away_pct,
            "biggest_drop_pct": s.biggest_drop_pct,
            "biggest_drop_at": s.biggest_drop_at,
        })),
    }))
}

/// "What's working" recommendations — top format/hook-type/length from captured data,
/// wrapped in concrete copy the team can act on.
pub fn recommendations(root: &Path) -> Result<Value> {
    // This pulls from the clips DB (autopilot-captured metrics). If the team hasn't
    // run autopilot yet, return an honest empty state.
    let db_path = config::db_path(root);
    if !db_path.exists() {
        return Ok(json!({
            "configured": true,
            "ready": false,
            "note": "No clips in the database yet. Run autopilot (or publish a few renders) to populate this."
        }));
    }
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(_) => {
            return Ok(json!({ "configured": true, "ready": false, "note": "DB unreadable" }))
        }
    };
    use crate::db;
    let clips = db::clips_with_latest_metrics(&conn).unwrap_or_default();
    if clips.is_empty() {
        return Ok(json!({
            "configured": true,
            "ready": false,
            "note": "No clips posted yet. Publish a few renders and the recommendations will populate."
        }));
    }

    // Aggregate by format (fmt) and hook_type.
    use std::collections::HashMap;
    let mut by_fmt: HashMap<String, (i64, f64, i64)> = HashMap::new(); // (views_sum, rev_sum, count)
    let mut by_hook: HashMap<String, (i64, f64, i64)> = HashMap::new();
    for c in &clips {
        if c.status.as_deref() != Some("posted") {
            continue;
        }
        if let Some(f) = c.fmt.as_ref() {
            let e = by_fmt.entry(f.clone()).or_insert((0, 0.0, 0));
            e.0 += c.views;
            e.1 += c.ad_revenue;
            e.2 += 1;
        }
        if let Some(h) = c.hook_type.as_ref() {
            let e = by_hook.entry(h.clone()).or_insert((0, 0.0, 0));
            e.0 += c.views;
            e.1 += c.ad_revenue;
            e.2 += 1;
        }
    }
    let mut fmt_sorted: Vec<(String, f64)> = by_fmt
        .iter()
        .map(|(k, (v, _, n))| (k.clone(), *v as f64 / *n as f64))
        .collect();
    fmt_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let best_fmt = fmt_sorted.first();
    let recommendations = match best_fmt {
        Some((fmt, avg_views)) if *avg_views > 0.0 => {
            let mut other_avg = 0.0;
            if fmt_sorted.len() > 1 {
                other_avg = fmt_sorted[1..].iter().map(|(_, v)| *v).sum::<f64>()
                    / (fmt_sorted.len() - 1) as f64;
            }
            let multiple = if other_avg > 0.0 {
                avg_views / other_avg
            } else {
                0.0
            };
            vec![format!(
                "Your {} videos average {:.0} views each{} — make more of those.",
                fmt,
                avg_views,
                if multiple > 1.2 {
                    format!(" ({:.1}× the next-best format)", multiple)
                } else {
                    String::new()
                }
            )]
        }
        _ => vec!["Not enough posted videos to compare formats yet.".to_string()],
    };

    Ok(json!({
        "configured": true,
        "ready": true,
        "format_breakdown": by_fmt.iter().map(|(k, (v, r, n))| {
            json!({ "format": k, "total_views": v, "revenue": r, "count": n, "avg_views": (*v as f64 / *n as f64) })
        }).collect::<Vec<_>>(),
        "hook_breakdown": by_hook.iter().map(|(k, (v, r, n))| {
            json!({ "hook_type": k, "total_views": v, "revenue": r, "count": n, "avg_views": (*v as f64 / *n as f64) })
        }).collect::<Vec<_>>(),
        "recommendations": recommendations,
    }))
}
