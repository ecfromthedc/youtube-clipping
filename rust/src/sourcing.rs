//! Stage 1 — SOURCE. Parity port of `src/ycp/sourcing.py`.
//!
//! Build a ranked daily queue of videos worth clipping: yt-dlp flat-enumerates each
//! creator's recent uploads (cheap, IDs only), re-fetches real view_count/timestamp on the
//! bounded candidate set (the empty-queue fix), computes view-velocity, screens titles
//! through the avoid-list gate, ranks, and upserts the top N per creator.
//!
//! The pure half — `parse_meta_lines`, `parse_entries`, `rank`, `load_creators`,
//! `render_queue_md` — is cross-checked byte-for-byte against the Python on the same inputs
//! (`ycp src-rank` harness). The yt-dlp shell-outs + the `run` orchestrator mirror the Python
//! structurally and are exercised by the autopilot row (no network here).
#![allow(dead_code)] // shell-outs + run() are wired by the autopilot orchestrator (last port row)

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Result};
use chrono::{SecondsFormat, TimeZone, Utc};

use crate::guardrails;
use crate::{config, db, optimize};

/// Tab-delimited per-video metadata. title is LAST so any tabs inside it survive split(4).
/// timestamp falls back to release_timestamp (both epoch seconds). Mirrors `_META_FORMAT`.
pub const META_FORMAT: &str =
    "%(id)s\t%(view_count)s\t%(timestamp,release_timestamp)s\t%(channel_id)s\t%(title)s";

/// Raw yt-dlp entry as parsed from a `--print META_FORMAT` line. 'NA'/'' → None.
#[derive(Debug, Clone, PartialEq)]
pub struct RawEntry {
    pub id: String,
    pub view_count: Option<i64>,
    pub timestamp: Option<f64>,
    pub channel_id: Option<String>,
    pub title: String,
}

/// A normalized candidate row with view-velocity (mirrors the dict parse_entries returns).
#[derive(Debug, Clone, PartialEq)]
pub struct SourceRow {
    pub video_id: String,
    pub creator: String,
    pub channel_id: Option<String>,
    pub title: Option<String>,
    pub url: String,
    pub views: i64,
    pub published_at: Option<String>,
    pub view_velocity: f64,
    pub lane: String,
    pub niche: Option<String>,
    pub has_captions: bool,
}

/// One creator to source (mirrors the load_creators dict).
#[derive(Debug, Clone)]
pub struct SourceCreator {
    pub name: String,
    pub url: String,
    pub lane: String,
    pub niche: String,
    pub has_captions: bool,
}

fn na_int(v: &str) -> Option<i64> {
    if v == "NA" || v.is_empty() { None } else { v.parse().ok() }
}

fn na_float(v: &str) -> Option<f64> {
    if v == "NA" || v.is_empty() { None } else { v.parse().ok() }
}

/// Parse `yt-dlp --print META_FORMAT` output into raw entries. Pure (mirrors `_parse_meta_lines`).
pub fn parse_meta_lines(stdout: &str) -> Vec<RawEntry> {
    let mut rows = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        // split("\t", 4) → at most 5 fields, the 5th keeps any tabs in the title.
        let parts: Vec<&str> = line.splitn(5, '\t').collect();
        if parts.len() < 5 {
            continue;
        }
        rows.push(RawEntry {
            id: parts[0].to_string(),
            view_count: na_int(parts[1]),
            timestamp: na_float(parts[2]),
            channel_id: if parts[3] == "NA" { None } else { Some(parts[3].to_string()) },
            title: parts[4].to_string(),
        });
    }
    rows
}

/// epoch-seconds → Python `datetime.fromtimestamp(ts, tz=utc).isoformat()` (e.g.
/// `2026-06-20T12:34:56+00:00`). yt-dlp timestamps are integer seconds → no fractional part.
fn iso_from_epoch(ts: f64) -> String {
    let secs = ts.trunc() as i64;
    let nanos = ((ts - secs as f64) * 1e9).round() as u32;
    let dt = Utc.timestamp_opt(secs, nanos).single().unwrap_or_else(|| Utc.timestamp_opt(secs, 0).unwrap());
    if nanos == 0 {
        dt.to_rfc3339_opts(SecondsFormat::Secs, false)
    } else {
        // Python isoformat uses microsecond precision when present.
        dt.to_rfc3339_opts(SecondsFormat::Micros, false)
    }
}

/// Normalize raw entries → candidate rows with view_velocity. Pure (mirrors `parse_entries`).
pub fn parse_entries(raw: &[RawEntry], creator: &str, lane: &str, now_epoch: f64) -> Vec<SourceRow> {
    let mut out = Vec::new();
    for e in raw {
        if e.id.is_empty() {
            continue;
        }
        let views = e.view_count.unwrap_or(0);
        let (velocity, published) = match e.timestamp {
            Some(ts) => {
                let hours = ((now_epoch - ts) / 3600.0).max(1.0);
                (crate::util::round_to(views as f64 / hours, 1), Some(iso_from_epoch(ts)))
            }
            None => (views as f64, None), // recency-ordered proxy when no timestamp
        };
        out.push(SourceRow {
            video_id: e.id.clone(),
            creator: creator.to_string(),
            channel_id: e.channel_id.clone(),
            title: Some(e.title.clone()),
            url: format!("https://www.youtube.com/watch?v={}", e.id),
            views,
            published_at: published,
            view_velocity: velocity,
            lane: lane.to_string(),
            niche: None,
            has_captions: false,
        });
    }
    out
}

/// Filter by min_views, then sort by view_velocity desc (stable). Pure (mirrors `rank`).
pub fn rank(candidates: &[SourceRow], min_views: i64) -> Vec<SourceRow> {
    let mut keep: Vec<SourceRow> = candidates.iter().filter(|c| c.views >= min_views).cloned().collect();
    // Python `sorted(..., reverse=True)` is stable; b.cmp(a) keeps equal-velocity order.
    keep.sort_by(|a, b| b.view_velocity.partial_cmp(&a.view_velocity).unwrap_or(std::cmp::Ordering::Equal));
    keep
}

/// Load creators from niches.yaml (mirrors `load_creators`). Errors if the file is missing.
pub fn load_creators(root: &Path, niches_path: Option<&Path>) -> Result<Vec<SourceCreator>> {
    let default = root.join("config").join("niches.yaml");
    let path = niches_path.unwrap_or(&default);
    if !path.is_file() {
        bail!(
            "{} not found. Create it from config/niches.example.yaml (or run after niche research).",
            path.display()
        );
    }
    let text = std::fs::read_to_string(path)?;
    let data: serde_yaml::Value = serde_yaml::from_str(&text)?;
    let mut creators = Vec::new();
    let niches = data.get("niches").and_then(|n| n.as_sequence()).cloned().unwrap_or_default();
    for niche in &niches {
        let niche_name = niche.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let lane_default =
            niche.get("lane_default").and_then(|v| v.as_str()).unwrap_or("owned").to_string();
        let list = niche.get("creators").and_then(|c| c.as_sequence()).cloned().unwrap_or_default();
        for c in &list {
            let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let handle = c.get("handle").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let url = if handle.starts_with("http") {
                handle.clone()
            } else {
                format!("https://www.youtube.com/{handle}/videos")
            };
            let lane =
                c.get("lane").and_then(|v| v.as_str()).map(String::from).unwrap_or_else(|| lane_default.clone());
            let has_captions = c.get("has_captions").and_then(|v| v.as_bool()).unwrap_or(false);
            creators.push(SourceCreator { name, url, lane, niche: niche_name.clone(), has_captions });
        }
    }
    Ok(creators)
}

// ── yt-dlp shell-outs (network; no DB write) ─────────────────────────────────

/// Fast flat enumerate of a channel's most-recent video IDs (no metadata). Mirrors `_ytdlp_flat_ids`.
fn ytdlp_flat_ids(channel_url: &str, limit: i64) -> Result<Vec<String>> {
    let out = Command::new("yt-dlp")
        .args([
            "--flat-playlist", "--no-warnings", "--print", "%(id)s",
            "--playlist-items", &format!("1-{limit}"), channel_url,
        ])
        .output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("yt-dlp flat enumerate failed for {channel_url}: {}", &err.trim()[..err.trim().len().min(300)]);
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

/// Real (non-flat) metadata for specific videos in ONE yt-dlp call. Mirrors `_ytdlp_meta`.
fn ytdlp_meta(video_ids: &[String]) -> Result<Vec<RawEntry>> {
    if video_ids.is_empty() {
        return Ok(Vec::new());
    }
    let urls: Vec<String> =
        video_ids.iter().map(|v| format!("https://www.youtube.com/watch?v={v}")).collect();
    let out = Command::new("yt-dlp")
        .args(["--no-warnings", "--print", META_FORMAT])
        .args(&urls)
        .output()?;
    let rows = parse_meta_lines(&String::from_utf8_lossy(&out.stdout));
    if rows.is_empty() && !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("yt-dlp meta fetch failed: {}", &err.trim()[..err.trim().len().min(300)]);
    }
    Ok(rows)
}

/// Fetch + rank ONE creator's top videos (network only). Mirrors `_source_creator`.
fn source_creator(c: &SourceCreator, cfg: &serde_yaml::Value) -> Result<Vec<SourceRow>> {
    let lookback = cfg["lookback_days"].as_i64().unwrap_or(14);
    let meta_limit = cfg["meta_fetch"].as_i64().unwrap_or(8) as usize;
    let per_creator = cfg["per_creator"].as_i64().unwrap_or(3) as usize;
    let min_views = cfg["min_views"].as_i64().unwrap_or(50_000);

    let mut ids = ytdlp_flat_ids(&c.url, lookback * 3)?;
    ids.truncate(meta_limit);
    let raw = ytdlp_meta(&ids)?;
    let now = Utc::now().timestamp() as f64;
    let candidates = parse_entries(&raw, &c.name, &c.lane, now);
    // Avoid-list gate (per-video): drop music/casino/licensed-IP titles BEFORE ranking.
    let allowed: Vec<SourceRow> = candidates
        .into_iter()
        .filter(|row| guardrails::source_allowed(row.title.as_deref().unwrap_or("")).0)
        .collect();
    let mut top = rank(&allowed, min_views);
    top.truncate(per_creator);
    for row in &mut top {
        row.niche = Some(c.niche.clone());
        row.has_captions = c.has_captions;
    }
    Ok(top)
}

/// Source all creators → upsert top N each → return the day's queue, weighted by learned
/// per-creator multipliers (double-down). Mirrors `run`. Creators run in bounded worker chunks.
pub fn run(root: &Path, niches_path: Option<&Path>) -> Result<Vec<SourceRow>> {
    let settings = config::load_settings(root)?;
    let cfg = settings["sourcing"].clone();
    let conn = db::open(&config::db_path(root))?;

    let all = load_creators(root, niches_path)?;
    let mut creators = Vec::new();
    let mut dropped = Vec::new();
    for c in all {
        if guardrails::creator_allowed(&c.name, &c.url) {
            creators.push(c);
        } else {
            dropped.push(c.name);
        }
    }
    if !dropped.is_empty() {
        println!("  ⚠ avoid-list gate dropped {}: {}", dropped.len(), dropped.join(", "));
    }

    let workers = (cfg["concurrency"].as_i64().unwrap_or(8) as usize).clamp(1, creators.len().max(1));
    let mut rows: Vec<SourceRow> = Vec::new();
    for chunk in creators.chunks(workers) {
        let results: Vec<(String, Result<Vec<SourceRow>>)> = std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|c| s.spawn(|| (c.name.clone(), source_creator(c, &cfg))))
                .collect();
            handles.into_iter().map(|h| h.join().expect("source thread panicked")).collect()
        });
        for (name, res) in results {
            match res {
                Ok(top) => rows.extend(top),
                Err(e) => println!("  ! skip {name}: {e}"),
            }
        }
    }

    for row in &rows {
        db::upsert_source_video(&conn, &to_db_row(row))?;
    }
    // Double-down: bias the queue by what's winning (default multiplier 1.0).
    let weights = optimize::load_weights(&optimize::Paths::new(root));
    rows.sort_by(|a, b| {
        let wa = a.view_velocity * weights.get(&a.creator).copied().unwrap_or(1.0);
        let wb = b.view_velocity * weights.get(&b.creator).copied().unwrap_or(1.0);
        wb.partial_cmp(&wa).unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(rows)
}

fn to_db_row(r: &SourceRow) -> db::SourceVideoRow {
    db::SourceVideoRow {
        video_id: r.video_id.clone(),
        creator: r.creator.clone(),
        channel_id: r.channel_id.clone(),
        title: r.title.clone(),
        url: r.url.clone(),
        views: r.views,
        published_at: r.published_at.clone(),
        view_velocity: r.view_velocity,
        lane: r.lane.clone(),
        status: None,
    }
}

/// Render the daily queue as markdown (mirrors `render_queue_md`). Pure.
pub fn render_queue_md(queue: &[SourceRow]) -> String {
    if queue.is_empty() {
        return "# Daily Source Queue\n\n_(empty — check creator handles in niches.yaml)_\n".to_string();
    }
    let mut lines = vec![
        "# Daily Source Queue".to_string(),
        String::new(),
        "| velocity/hr | views | lane | creator | title | url |".to_string(),
        "|---|---|---|---|---|---|".to_string(),
    ];
    for r in queue {
        // {view_velocity:,.0f}
        let vel = crate::util::comma(format!("{:.0}", r.view_velocity).parse::<i64>().unwrap_or(0));
        let title: String =
            r.title.clone().unwrap_or_default().chars().take(60).collect::<String>().replace('|', "/");
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            vel,
            crate::util::comma(r.views),
            r.lane,
            r.creator,
            title,
            r.url
        ));
    }
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_meta_lines_normalizes_na_and_keeps_title_tabs() {
        let stdout = "abc\t1000\t1700000000\tUCxyz\thello world\n\
                      def\tNA\tNA\tNA\ttitle\twith\ttab\n\
                      short\tline\n";
        let rows = parse_meta_lines(stdout);
        assert_eq!(rows.len(), 2); // the 2-field line is dropped
        assert_eq!(rows[0], RawEntry {
            id: "abc".into(),
            view_count: Some(1000),
            timestamp: Some(1_700_000_000.0),
            channel_id: Some("UCxyz".into()),
            title: "hello world".into(),
        });
        // NA → None; title keeps its embedded tabs (split at most 5 fields).
        assert_eq!(rows[1].view_count, None);
        assert_eq!(rows[1].timestamp, None);
        assert_eq!(rows[1].channel_id, None);
        assert_eq!(rows[1].title, "title\twith\ttab");
    }

    #[test]
    fn parse_entries_velocity_and_published() {
        let raw = vec![RawEntry {
            id: "v1".into(),
            view_count: Some(36000),
            timestamp: Some(1_700_000_000.0),
            channel_id: Some("UC1".into()),
            title: "a title".into(),
        }];
        // 10 hours after publish → 3600 views/hr.
        let now = 1_700_000_000.0 + 10.0 * 3600.0;
        let rows = parse_entries(&raw, "Creator", "owned", now);
        assert_eq!(rows[0].view_velocity, 3600.0);
        assert_eq!(rows[0].published_at.as_deref(), Some("2023-11-14T22:13:20+00:00"));
        assert_eq!(rows[0].url, "https://www.youtube.com/watch?v=v1");
    }

    #[test]
    fn parse_entries_no_timestamp_uses_views_proxy() {
        let raw = vec![RawEntry {
            id: "v2".into(),
            view_count: Some(50_000),
            timestamp: None,
            channel_id: None,
            title: "t".into(),
        }];
        let rows = parse_entries(&raw, "C", "owned", 0.0);
        assert_eq!(rows[0].view_velocity, 50_000.0);
        assert_eq!(rows[0].published_at, None);
    }

    #[test]
    fn rank_filters_min_views_and_sorts_desc_stable() {
        let mk = |id: &str, views: i64, vel: f64| SourceRow {
            video_id: id.into(), creator: "c".into(), channel_id: None, title: Some("t".into()),
            url: "u".into(), views, published_at: None, view_velocity: vel, lane: "owned".into(),
            niche: None, has_captions: false,
        };
        let cands = vec![mk("lo", 100, 9.0), mk("a", 60_000, 5.0), mk("b", 70_000, 9.0), mk("c", 80_000, 9.0)];
        let ranked = rank(&cands, 50_000);
        // "lo" dropped by min_views; ties (b,c at 9.0) keep input order; a last.
        let ids: Vec<&str> = ranked.iter().map(|r| r.video_id.as_str()).collect();
        assert_eq!(ids, vec!["b", "c", "a"]);
    }

    #[test]
    fn render_queue_md_empty_and_rows() {
        assert!(render_queue_md(&[]).contains("_(empty"));
        let row = SourceRow {
            video_id: "v".into(), creator: "Jubilee".into(), channel_id: None,
            title: Some("a | piped | title".into()), url: "http://x".into(), views: 51725,
            published_at: None, view_velocity: 51724.6, lane: "owned".into(), niche: None,
            has_captions: false,
        };
        let md = render_queue_md(&[row]);
        assert!(md.contains("| 51,725 | 51,725 | owned | Jubilee | a / piped / title | http://x |"));
    }
}
