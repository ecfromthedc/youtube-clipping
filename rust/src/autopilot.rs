//! Stage 6 — AUTOPILOT. One command that chains the daily closed loop. Mirrors
//! src/ycp/autopilot.py.
//!
//!   source → clip (top-N queued, idempotent) → qc → capture → brief → scoreboard → optimize
//!   → distribute → cleanup
//!
//! Design goals (same as Python): idempotent/safe to re-run, fault-isolated (one stage failing
//! is logged and the chain continues), honest about gates (distribution/auto-QC reported as
//! their real state). The pure stage-selection logic (`select_unclipped`, `angle_for`,
//! `channel_for`, `connected_channels`) is byte-checkable; the chain is verified by a real
//! `--skip-source --no-clip` invocation cross-checked against Python.
use std::collections::BTreeSet;

use anyhow::Result;
use rusqlite::Connection;

use crate::{
    archive, brief, capture, config, db, diagnose, distribute, experiment, optimize, qc,
    scoreboard, scoring, sourcing, util,
};

/// Owned channels are the only lane the factory clips for (Whop cut 2026-06; owned-first).
pub const DEFAULT_LANES: &[&str] = &["owned"];

/// Map a niche label → a hook-agent angle (tunes the viral-hook prompt). Pure.
pub fn angle_for(niche: Option<&str>) -> &'static str {
    let n = niche.unwrap_or("").to_lowercase();
    if n.contains("debate") || n.contains("agitation") || n.contains("hot seat") {
        "agitation"
    } else if n.contains("finance") || n.contains("money") {
        "finance"
    } else {
        ""
    }
}

/// niche name (niches.yaml `name:`) → owned-channel slug (the Postiz routing key). Mirrors the
/// explicit CHANNEL_SLUGS map.
fn channel_slug(niche: Option<&str>) -> Option<&'static str> {
    match niche.unwrap_or("").to_lowercase().as_str() {
        "debate-agitation" => Some("hot-seat"),
        "finance-money" => Some("money-fights"),
        "comedy-crashout" => Some("crash-out"),
        "business-finance" => Some("boardroom"),
        _ => None,
    }
}

/// Map a niche label → its owned-channel slug. Unknown niches fall back to 'clips' (won't match
/// a configured Postiz integration → distribution raises rather than posting wrong). Pure.
pub fn channel_for(niche: Option<&str>) -> String {
    channel_slug(niche).unwrap_or("clips").to_string()
}

/// Channel slugs with a mapped Postiz integration id — the only ones that can post. Empty when
/// distribution is off, so dev/demo runs aren't gated. Mirrors `connected_channels`.
pub fn connected_channels(settings: &serde_yaml::Value) -> BTreeSet<String> {
    let d = &settings["distribution"];
    if !d["enabled"].as_bool().unwrap_or(false) {
        return BTreeSet::new();
    }
    d["postiz"]["channels"]
        .as_mapping()
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| {
                    let id = v.as_str().unwrap_or("");
                    if id.is_empty() {
                        None
                    } else {
                        Some(k.as_str()?.to_string())
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Pick the top `max_videos` queued rows not yet clipped, in allowed lanes. Pure. `queue` is
/// assumed already ranked (hottest first). Rows without a url, or whose video_id is in
/// `clipped_ids`, or whose lane isn't allowed, are skipped. Mirrors `select_unclipped`.
pub fn select_unclipped<'a>(
    queue: &'a [sourcing::SourceRow],
    clipped_ids: &std::collections::HashSet<String>,
    max_videos: usize,
    lanes: &[&str],
) -> Vec<&'a sourcing::SourceRow> {
    let mut picked: Vec<&sourcing::SourceRow> = Vec::new();
    for row in queue {
        if max_videos != 0 && picked.len() >= max_videos {
            break;
        }
        if !lanes.contains(&row.lane.as_str()) {
            continue;
        }
        if row.url.is_empty() {
            continue;
        }
        if clipped_ids.contains(&row.video_id) {
            continue;
        }
        picked.push(row);
    }
    picked
}

/// One stage's outcome (mirrors the frozen `StageResult` dataclass).
pub struct StageResult {
    pub name: &'static str,
    pub ok: bool,
    pub detail: String,
}

impl StageResult {
    fn line(&self) -> String {
        let mark = if self.ok { "✓" } else { "✗" };
        format!("  {mark} {:<11} {}", self.name, self.detail)
    }
}

/// Run one stage, capture ok/detail, never raise out of the chain. Logs its line on completion.
fn stage(name: &'static str, results: &mut Vec<StageResult>, fn_: impl FnOnce() -> Result<String>) {
    let res = match fn_() {
        Ok(detail) => StageResult {
            name,
            ok: true,
            detail,
        },
        Err(e) => {
            // Python logs `{ExcType}: {msg[:160]}`; anyhow has no exc-type, so the message
            // (truncated) stands in. Only seen on a stage failure (✗) — not on the happy path.
            let msg: String = format!("{e}").chars().take(160).collect();
            StageResult {
                name,
                ok: false,
                detail: msg,
            }
        }
    };
    println!("{}", res.line());
    results.push(res);
}

/// Knobs for one autopilot run (mirrors the keyword args of autopilot.py `run`).
pub struct RunOpts<'a> {
    pub max_videos: usize,
    pub skip_source: bool,
    pub do_clip: bool,
    pub hook_cta: bool,
    pub lanes: &'a [&'a str],
}

impl Default for RunOpts<'_> {
    fn default() -> Self {
        RunOpts {
            max_videos: 5,
            skip_source: false,
            do_clip: true,
            hook_cta: true,
            lanes: DEFAULT_LANES,
        }
    }
}

/// Convert a persisted source-queue row (skip-source path) into the richer SourceRow the live
/// path returns. The DB has no niche/has_captions columns → None/false, exactly like the Python
/// `dict(r).get("niche")` returning None.
fn db_row_to_source(r: db::SourceVideoRow) -> sourcing::SourceRow {
    sourcing::SourceRow {
        video_id: r.video_id,
        creator: r.creator,
        channel_id: r.channel_id,
        title: r.title,
        url: r.url,
        views: r.views,
        published_at: r.published_at,
        view_velocity: r.view_velocity,
        lane: r.lane,
        niche: None,
        has_captions: false,
    }
}

/// Chain the daily loop end-to-end. Returns a per-stage result list. Mirrors autopilot.py `run`.
pub fn run(conn: &Connection, root: &std::path::Path, opts: &RunOpts) -> Result<Vec<StageResult>> {
    let settings = config::load_settings(root)?;
    let cfg = scoring::ScoreCfg::from_settings(&settings);
    let mut results: Vec<StageResult> = Vec::new();
    println!("▶ autopilot: source → clip → qc → capture → brief → scoreboard");

    // 1 ─ SOURCE ──────────────────────────────────────────────────────────────
    let mut queue: Vec<sourcing::SourceRow> = Vec::new();
    stage("source", &mut results, || {
        if opts.skip_source {
            queue = db::source_queue(conn, None)?
                .into_iter()
                .map(db_row_to_source)
                .collect();
            Ok(format!("reused {} queued (skip-source)", queue.len()))
        } else {
            queue = sourcing::run(root, None)?;
            std::fs::write(
                root.join("data").join("source-queue.md"),
                sourcing::render_queue_md(&queue),
            )?;
            Ok(format!(
                "{} videos queued → data/source-queue.md",
                queue.len()
            ))
        }
    });

    // 2 ─ CLIP (idempotent) ──────────────────────────────────────────────────
    stage("clip", &mut results, || {
        if !opts.do_clip {
            return Ok("skipped (--no-clip)".to_string());
        }
        let clipped = db::clipped_source_ids(conn)?;
        // Only clip sources whose channel can actually post — else clips just park locally.
        let conn_chans = connected_channels(&settings);
        let pool: Vec<sourcing::SourceRow> = queue
            .iter()
            .filter(|r| {
                conn_chans.is_empty() || conn_chans.contains(&channel_for(r.niche.as_deref()))
            })
            .cloned()
            .collect();
        let todo = select_unclipped(&pool, &clipped, opts.max_videos, opts.lanes);
        if todo.is_empty() {
            return Ok(if conn_chans.is_empty() {
                "0 new sources to clip (all caught up)".to_string()
            } else {
                format!(
                    "0 to clip for connected channels: {}",
                    join_sorted(&conn_chans)
                )
            });
        }
        let mut made = 0usize;
        for row in &todo {
            let created = crate::clip::run(
                conn,
                root,
                &row.url,
                &crate::clip::RunOpts {
                    max_clips: 6,
                    lane: &row.lane,
                    source_creator: &row.creator,
                    channel: &channel_for(row.niche.as_deref()),
                    hook_cta: opts.hook_cta,
                    title: None,
                    gameplay: None,
                    source_video_id: Some(&row.video_id),
                    angle: angle_for(row.niche.as_deref()),
                    window_sec: None,
                    captions_on: !row.has_captions,
                },
            )?;
            made += created.len();
        }
        Ok(format!(
            "{made} clips from {} sources (pending_qc)",
            todo.len()
        ))
    });

    // 3 ─ QC ──────────────────────────────────────────────────────────────────
    // QC is MANUAL (qc.auto:false) or AUTO-guardrails (qc.auto:true) per settings.
    stage("qc", &mut results, || {
        if db::pending_qc_clips(conn)?.is_empty() {
            return Ok("no clips pending QC".to_string());
        }
        match qc::dispatch_pending(conn, root)? {
            qc::Dispatch::Human {
                channel,
                dispatched,
            } => Ok(format!("{dispatched} dispatched for review via {channel}")),
            qc::Dispatch::Auto { approved, rejected } => Ok(format!(
                "auto-QC: {approved} approved, {rejected} rejected (guardrails)"
            )),
        }
    });

    // 4 ─ CAPTURE ──────────────────────────────────────────────────────────────
    stage("capture", &mut results, || {
        let pub_n = capture::capture_public(conn, root)?;
        let full = capture::capture_full_analytics(conn, root)?;
        Ok(format!(
            "{pub_n} public-view + {full} owned-analytics snapshots"
        ))
    });

    // 5 ─ BRIEF ────────────────────────────────────────────────────────────────
    let top_n = settings["brief"]["top_n"].as_u64().unwrap_or(5) as usize;
    stage("brief", &mut results, || {
        let clips = db::clips_with_latest_metrics(conn)?;
        let a = scoring::analyze(&clips, &cfg);
        let today = util::today_iso();
        let mut md = brief::build(&a, clips.len(), top_n, &today);
        let why = diagnose::diagnose(root, &settings, &a); // None without DeepSeek/data
        if let Some(w) = &why {
            md.push_str(&format!("\n\n## 🧠 Why it's working (analyst)\n{w}\n"));
        }
        db::save_brief(conn, &today, &md)?;
        std::fs::write(root.join("data").join("latest-brief.md"), &md)?;
        Ok(if why.is_some() {
            "Double-Down Brief (+why)".to_string()
        } else {
            "Double-Down Brief → data/latest-brief.md".to_string()
        })
    });

    // 6 ─ SCOREBOARD ───────────────────────────────────────────────────────────
    stage("scoreboard", &mut results, || {
        let clips = db::clips_with_latest_metrics(conn)?;
        let md = scoreboard::build(&clips, &util::today_iso());
        std::fs::write(root.join("SCOREBOARD.md"), md)?;
        Ok("Race to $15K → SCOREBOARD.md".to_string())
    });

    // 6.5 ─ OPTIMIZE ───────────────────────────────────────────────────────────
    stage("optimize", &mut results, || {
        let r = optimize::run(conn, root, &util::today_iso())?;
        let min_views = settings["ab"]["min_views"].as_i64().unwrap_or(1000);
        let clips = db::clips_with_latest_metrics(conn)?;
        let ab = experiment::resolve(root, &clips, min_views)?;
        let ab_note = if ab.is_empty() {
            String::new()
        } else {
            format!(" · {} A/B winner(s) crowned", ab.len())
        };
        Ok(format!(
            "learned from {} clips → +{} boosted / -{} starved{ab_note} (→ IMPROVEMENT-LOG.md)",
            r.clips,
            r.boosted.len(),
            r.suppressed.len()
        ))
    });

    // 7 ─ DISTRIBUTE ───────────────────────────────────────────────────────────
    stage("distribute", &mut results, || {
        let r = distribute::run(conn, root)?;
        if !r.enabled {
            return Ok(format!(
                "OFF — {} approved clips waiting; {}",
                r.waiting, r.note
            ));
        }
        let prov = settings["distribution"]["provider"]
            .as_str()
            .unwrap_or("postiz");
        Ok(format!(
            "delivered {} via {prov} ({} parked [channel not connected], {} blocked, {} failed)",
            r.delivered, r.parked, r.blocked, r.failed
        ))
    });

    // 8 ─ CLEANUP ──────────────────────────────────────────────────────────────
    stage("cleanup", &mut results, || {
        Ok(format!(
            "{} local files pruned (posted → live + archived)",
            archive::prune_local(conn, root)?
        ))
    });

    let ok = results.iter().filter(|r| r.ok).count();
    println!("▶ autopilot done: {ok}/{} stages ok", results.len());
    Ok(results)
}

fn join_sorted(set: &BTreeSet<String>) -> String {
    set.iter().cloned().collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn row(video_id: &str, lane: &str, url: &str, niche: Option<&str>) -> sourcing::SourceRow {
        sourcing::SourceRow {
            video_id: video_id.into(),
            creator: "c".into(),
            channel_id: None,
            title: None,
            url: url.into(),
            views: 0,
            published_at: None,
            view_velocity: 0.0,
            lane: lane.into(),
            niche: niche.map(String::from),
            has_captions: false,
        }
    }

    #[test]
    fn angle_mapping() {
        assert_eq!(angle_for(Some("debate-agitation")), "agitation");
        assert_eq!(angle_for(Some("Hot Seat")), "agitation");
        assert_eq!(angle_for(Some("finance-money")), "finance");
        assert_eq!(angle_for(Some("comedy-crashout")), "");
        assert_eq!(angle_for(None), "");
    }

    #[test]
    fn channel_mapping_falls_back_to_clips() {
        assert_eq!(channel_for(Some("finance-money")), "money-fights");
        assert_eq!(channel_for(Some("unknown-niche")), "clips");
        assert_eq!(channel_for(None), "clips");
    }

    #[test]
    fn select_skips_clipped_wrong_lane_and_empty_url() {
        let queue = vec![
            row("a", "owned", "u1", None),
            row("b", "rented", "u2", None), // wrong lane
            row("c", "owned", "", None),    // no url
            row("d", "owned", "u4", None),  // already clipped
            row("e", "owned", "u5", None),
        ];
        let clipped: HashSet<String> = ["d".to_string()].into_iter().collect();
        let picked = select_unclipped(&queue, &clipped, 5, DEFAULT_LANES);
        let ids: Vec<&str> = picked.iter().map(|r| r.video_id.as_str()).collect();
        assert_eq!(ids, vec!["a", "e"]);
    }

    #[test]
    fn select_respects_max_videos() {
        let queue: Vec<sourcing::SourceRow> = (0..10)
            .map(|i| row(&format!("v{i}"), "owned", &format!("u{i}"), None))
            .collect();
        let picked = select_unclipped(&queue, &HashSet::new(), 3, DEFAULT_LANES);
        assert_eq!(picked.len(), 3);
    }
}
