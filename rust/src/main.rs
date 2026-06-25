//! `ycp` — YouTube clipping closed-loop ops (Rust port, in progress).
//! The Python in ../src/ycp stays the live system until this reaches parity.
mod archive;
mod brief;
mod capture;
mod captions;
mod clip;
mod config;
mod db;
mod distribute;
mod enhance;
mod experiment;
mod guardrails;
mod hooks;
mod optimize;
mod reframe;
mod scoreboard;
mod scoring;
mod sourcing;
mod srt;
mod transcribe;
mod util;

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ycp", about = "YouTube clipping closed-loop ops")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create the database.
    Init,
    /// Clip counts by status + total views (reads the same data/clips.db).
    Status,
    /// Deterministic scoring rollup — top creators by virality (cross-checks scoring.py).
    Scoring,
    /// Gamified Race-to-$15K scoreboard markdown (cross-checks scoreboard.py).
    Scoreboard {
        /// Fixed as-of date for reproducible diffing against Python.
        #[arg(long, default_value = "AOD")]
        as_of: String,
    },
    /// Weekly Double-Down Brief markdown (cross-checks brief.py).
    Brief {
        /// Fixed week label for reproducible diffing against Python.
        #[arg(long, default_value = "WK")]
        week: String,
    },
    /// Learned source weights + creative preferences (cross-checks optimize.py).
    Optimize,
    /// A/B hook winners — top posted variant per experiment (cross-checks experiment.py).
    Experiment,
    /// Hook agent — deterministic score/safety/best on a moment (cross-checks hooks.py).
    /// Generation needs DEEPSEEK_API_KEY; with none, `best` is the heuristic fallback.
    Hook {
        /// Transcript of the clip moment (also scored directly as a candidate hook).
        moment: String,
        /// Channel angle (e.g. finance, debate, agitation); biases scoring.
        #[arg(default_value = "")]
        angle: String,
    },
    /// SRT slice + caption chunking on a file (cross-checks srt.py + captions.py).
    Captions {
        /// Path to an SRT file (e.g. whisper output).
        srt: PathBuf,
        /// Clip window start, seconds.
        #[arg(default_value_t = 0.0)]
        start: f64,
        /// Clip window end, seconds.
        #[arg(default_value_t = 1e9)]
        end: f64,
    },
    /// Posting-slot assignment (cross-checks distribute.assign_slots). Pure.
    Slots {
        /// How many slots to assign.
        n: usize,
        /// IANA timezone (e.g. America/New_York).
        tz: String,
        /// Start instant, RFC-3339 (e.g. 2026-06-24T07:00:00-04:00).
        start: String,
        /// Posting times, comma-separated HH:MM (e.g. 06:00,12:30,19:00).
        times: String,
    },
    /// Retention-curve signals (cross-checks capture.analyze_retention). Pure.
    Retention {
        /// Curve as comma-separated elapsed:watch pairs (e.g. 0:1,0.1:0.6,0.2:0.6,1:0.4).
        curve: String,
    },
    /// Clip planning — windows + heuristic scores from an SRT (cross-checks clip.plan_clips). Pure.
    Plan {
        /// Path to an SRT file (e.g. whisper output).
        srt: PathBuf,
        #[arg(default_value_t = 15.0)]
        min_len: f64,
        #[arg(default_value_t = 60.0)]
        max_len: f64,
        #[arg(long)]
        top: Option<usize>,
    },
    /// Hook heuristic score for one text+duration (cross-checks clip.score_candidate). Pure.
    ScoreCand {
        text: String,
        duration: f64,
    },
    /// Parse + rank a yt-dlp `--print` meta dump (cross-checks sourcing.parse_entries+rank). Pure.
    SrcRank {
        /// File of tab-delimited yt-dlp meta lines (id\tviews\tts\tchannel\ttitle).
        meta_file: PathBuf,
        creator: String,
        lane: String,
        /// "now" as epoch seconds (fixed for reproducible velocity vs Python).
        now_epoch: f64,
        #[arg(long, default_value_t = 50_000)]
        min_views: i64,
    },
    /// Face-pan crop-x expression from a track (cross-checks reframe.crop_x_expr). Pure.
    CropX {
        /// Track as comma-separated t:frac pairs (e.g. 0:0.2,0.3:0.2,6:0.85).
        track: String,
        scaled_w: i64,
        #[arg(default_value_t = 1080)]
        crop_w: i64,
        #[arg(default_value_t = 0.05)]
        jump: f64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = config::find_root()?;
    let conn = db::open(&config::db_path(&root))?;
    match cli.cmd {
        Cmd::Init => {
            println!("✓ database ready at {}", config::db_path(&root).display());
        }
        Cmd::Status => {
            let clips = db::clips_with_latest_metrics(&conn)?;
            let posted = clips
                .iter()
                .filter(|c| c.status.as_deref() == Some("posted"))
                .count();
            let views: i64 = clips.iter().map(|c| c.views).sum();
            println!("ycp (rust) · {}", root.display());
            println!(
                "clips: {} total · {} posted · {} views",
                clips.len(),
                posted,
                views
            );
            let mut by: BTreeMap<String, usize> = BTreeMap::new();
            for c in &clips {
                *by.entry(c.status.clone().unwrap_or_default()).or_default() += 1;
            }
            for (status, n) in by {
                println!("  {status:<12} {n}");
            }
        }
        Cmd::Scoring => {
            let settings = config::load_settings(&root)?;
            let cfg = scoring::ScoreCfg::from_settings(&settings);
            let clips = db::clips_with_latest_metrics(&conn)?;
            let a = scoring::analyze(&clips, &cfg);
            println!(
                "scored {} clips · top creators by virality:",
                a.scored.len()
            );
            for r in a.by_creator.iter().take(5) {
                println!(
                    "  {:<26} score {:>5.1} · {:>9.0} views · n={}",
                    r.key, r.avg_score, r.avg_views, r.n
                );
            }
        }
        Cmd::Scoreboard { as_of } => {
            let clips = db::clips_with_latest_metrics(&conn)?;
            print!("{}", scoreboard::build(&clips, &as_of));
        }
        Cmd::Brief { week } => {
            let settings = config::load_settings(&root)?;
            let cfg = scoring::ScoreCfg::from_settings(&settings);
            let top_n = settings["brief"]["top_n"].as_u64().unwrap_or(5) as usize;
            let clips = db::clips_with_latest_metrics(&conn)?;
            let a = scoring::analyze(&clips, &cfg);
            print!("{}", brief::build(&a, clips.len(), top_n, &week));
        }
        Cmd::Optimize => {
            let settings = config::load_settings(&root)?;
            let cfg = scoring::ScoreCfg::from_settings(&settings);
            let factors = optimize::Factors::from_settings(&settings);
            let clips = db::clips_with_latest_metrics(&conn)?;
            let a = scoring::analyze(&clips, &cfg);
            let weights = optimize::creator_weights(&a, &factors, cfg.scale_q, cfg.kill_q);
            let (prefer_hooks, prefer_length) = optimize::creative_prefs(&a);
            println!("learned creator weights: {weights:?}");
            println!("prefer_hooks: {prefer_hooks:?} · prefer_length: {prefer_length:?}");
        }
        Cmd::Experiment => {
            let settings = config::load_settings(&root)?;
            let min_views = settings["ab"]["min_views"].as_i64().unwrap_or(1000);
            let clips = db::clips_with_latest_metrics(&conn)?;
            for w in experiment::winners(&clips, min_views) {
                // pipe-delimited for byte-diffing against experiment.py.
                println!(
                    "{}|{}|{}|{}|{:.1}",
                    w.experiment, w.winning_hook, w.winning_views, w.variants, w.margin
                );
            }
        }
        Cmd::Hook { moment, angle } => {
            // Pipe-delimited, deterministic lines for byte-diffing against hooks.py.
            println!("score|{:.3}", hooks::score_hook(&moment, &angle));
            println!("safe|{}", hooks::looks_safe(&moment));
            let b = hooks::best(&root, &moment, &angle, 6, 10, None, &[]);
            println!("best_text|{}", b.text);
            println!("best_type|{}", b.typ);
        }
        Cmd::Captions { srt, start, end } => {
            let text = std::fs::read_to_string(&srt)?;
            let sliced = srt::slice_and_shift(&srt::parse_srt(&text), start, end);
            print!("{}", srt::to_srt(&sliced));
            // chunks, pipe-delimited, for byte-diffing against captions.py.
            for ch in captions::build_chunks(&sliced, captions::MAX_WORDS, captions::MIN_DWELL) {
                println!("{:.3}|{:.3}|{}", ch.start, ch.end, ch.text());
            }
        }
        Cmd::Slots { n, tz, start, times } => {
            let start = chrono::DateTime::parse_from_rfc3339(&start)?;
            let times: Vec<String> = times.split(',').map(|s| s.trim().to_string()).collect();
            for slot in distribute::assign_slots(n, &times, &tz, start) {
                println!("{slot}");
            }
        }
        Cmd::Retention { curve } => {
            let pts: Vec<(f64, f64)> = curve
                .split(',')
                .filter_map(|pair| {
                    let (e, w) = pair.trim().split_once(':')?;
                    Some((e.trim().parse().ok()?, w.trim().parse().ok()?))
                })
                .collect();
            match capture::analyze_retention(&pts) {
                Some(r) => println!(
                    "{:.3}|{:.1}|{:.1}|{:.2}",
                    r.hook_retention, r.swipe_away_pct, r.biggest_drop_pct, r.biggest_drop_at
                ),
                None => println!("none"),
            }
        }
        Cmd::Plan { srt, min_len, max_len, top } => {
            let text = std::fs::read_to_string(&srt)?;
            let segs = srt::parse_srt(&text);
            // Pipe-delimited (start|end|duration|score|text) for byte-diffing against clip.plan_clips.
            for c in clip::plan_clips(&segs, min_len, max_len, top) {
                println!("{:.3}|{:.3}|{:.2}|{:.3}|{}", c.start, c.end, c.duration(), c.score, c.text);
            }
        }
        Cmd::ScoreCand { text, duration } => {
            println!("{:.3}", clip::score_candidate(&text, duration));
        }
        Cmd::SrcRank { meta_file, creator, lane, now_epoch, min_views } => {
            let stdout = std::fs::read_to_string(&meta_file)?;
            let raw = sourcing::parse_meta_lines(&stdout);
            let cands = sourcing::parse_entries(&raw, &creator, &lane, now_epoch);
            // Pipe-delimited rows for byte-diffing against sourcing.rank(parse_entries(...)).
            for r in sourcing::rank(&cands, min_views) {
                println!(
                    "{}|{}|{:.1}|{}|{}",
                    r.video_id,
                    r.views,
                    r.view_velocity,
                    r.published_at.unwrap_or_default(),
                    r.title.unwrap_or_default()
                );
            }
        }
        Cmd::CropX { track, scaled_w, crop_w, jump } => {
            let pts: Vec<(f64, f64)> = track
                .split(',')
                .filter_map(|pair| {
                    let (t, f) = pair.trim().split_once(':')?;
                    Some((t.trim().parse().ok()?, f.trim().parse().ok()?))
                })
                .collect();
            match reframe::crop_x_expr(&pts, scaled_w, crop_w, jump) {
                Some(e) => println!("{e}"),
                None => println!("none"),
            }
        }
    }
    Ok(())
}
