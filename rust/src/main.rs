//! `ycp` — YouTube clipping closed-loop ops (Rust port, in progress).
//! The Python in ../src/ycp stays the live system until this reaches parity.
mod actions;
mod analytics;
mod archive;
mod autopilot;
mod brief;
mod captions;
mod capture;
mod channels;
mod clip;
mod commentary;
mod config;
mod db;
mod diagnose;
mod distribute;
mod enhance;
mod experiment;
mod formats;
mod guardrails;
mod hooks;
mod listicle;
mod optimize;
mod qc;
mod reframe;
mod scoreboard;
mod scoring;
mod server;
mod sourcing;
mod srt;
mod story;
mod transcribe;
mod util;
mod vision;
mod voice;

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
    ScoreCand { text: String, duration: f64 },
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
    /// Caption render — frame schedule + rasterize to PNGs (cross-checks captions.render_overlay).
    /// Schedule lines (cfg|, frames|, per-frame title/chunk) are byte-checkable; the pixels
    /// (ab_glyph vs Pillow) are visually-equivalent, not byte-identical.
    Caprender {
        /// Path to an SRT file (e.g. whisper output).
        srt: PathBuf,
        /// Clip duration, seconds.
        #[arg(default_value_t = 3.0)]
        duration: f64,
        /// Hook title (empty = no title).
        #[arg(default_value = "")]
        title: String,
        /// Where to write PNG frames (default: a temp dir).
        #[arg(long)]
        out: Option<PathBuf>,
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
    /// Chain the daily loop: source→clip→qc→capture→brief→scoreboard→optimize→distribute→cleanup
    /// (cross-checks autopilot.py). Exit 1 if any stage failed.
    Autopilot {
        /// Max new source videos to clip this run.
        #[arg(long, default_value_t = 5)]
        max_videos: usize,
        /// Reuse the existing DB queue instead of re-fetching (fast).
        #[arg(long)]
        skip_source: bool,
        /// Run the chain but skip the (slow) clip stage.
        #[arg(long)]
        no_clip: bool,
        /// Don't burn the hook title + CTA overlay.
        #[arg(long)]
        no_hook: bool,
    },
    /// Launch the Tides Tiller editor — a browser UI over the clip pipeline.
    /// Internal team tool: upload footage → see ranked clip moments → render a
    /// captioned 9:16 MP4. No DB/auth/billing; projects live in data/editor/.
    Serve {
        /// Port to bind (default 8787).
        #[arg(short, long, default_value_t = 8787)]
        port: u16,
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
        Cmd::Slots {
            n,
            tz,
            start,
            times,
        } => {
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
        Cmd::Plan {
            srt,
            min_len,
            max_len,
            top,
        } => {
            let text = std::fs::read_to_string(&srt)?;
            let segs = srt::parse_srt(&text);
            // Pipe-delimited (start|end|duration|score|text) for byte-diffing against clip.plan_clips.
            for c in clip::plan_clips(&segs, min_len, max_len, top) {
                println!(
                    "{:.3}|{:.3}|{:.2}|{:.3}|{}",
                    c.start,
                    c.end,
                    c.duration(),
                    c.score,
                    c.text
                );
            }
        }
        Cmd::ScoreCand { text, duration } => {
            println!("{:.3}", clip::score_candidate(&text, duration));
        }
        Cmd::SrcRank {
            meta_file,
            creator,
            lane,
            now_epoch,
            min_views,
        } => {
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
        Cmd::Caprender {
            srt,
            duration,
            title,
            out,
        } => {
            let settings = config::load_settings(&root).ok();
            let text = std::fs::read_to_string(&srt)?;
            let segs = srt::parse_srt(&text);
            let chunks = captions::build_chunks(&segs, captions::MAX_WORDS, captions::MIN_DWELL);
            let title_opt = if title.is_empty() {
                None
            } else {
                Some(title.as_str())
            };
            let out_dir = out.unwrap_or_else(|| std::env::temp_dir().join("ycp_caprender"));
            let n = captions::render_overlay(
                &chunks,
                duration,
                &out_dir,
                title_opt,
                captions::SIZE,
                captions::FPS,
                None,
                settings.as_ref(),
            )?;
            // Deterministic frame schedule — pipe-delimited for byte-diffing against captions.py.
            let cfg = captions::caption_cfg(settings.as_ref());
            println!(
                "cfg|{}|{:.4}|{:.4}",
                cfg.case, cfg.size_pct, cfg.hook_hold_sec
            );
            println!("frames|{n}");
            for f in 0..n {
                let t = f as f64 / captions::FPS as f64;
                let title_shown = title_opt.is_some() && t < cfg.hook_hold_sec;
                let active = chunks
                    .iter()
                    .find(|c| c.start <= t && t < c.end)
                    .map(|c| c.text())
                    .unwrap_or_default();
                println!(
                    "{f}|{t:.4}|{}|{}",
                    if title_shown { "T" } else { "-" },
                    active
                );
            }
            // Prove the rasterizer ran: dims of frame 0 + count of frames with any ink.
            if let Ok(im) = image::open(out_dir.join("00000.png")) {
                println!("dims|{}x{}", im.width(), im.height());
            }
            let ink = (0..n)
                .filter(|f| {
                    image::open(out_dir.join(format!("{f:05}.png")))
                        .map(|im| im.to_rgba8().pixels().any(|p| p[3] > 0))
                        .unwrap_or(false)
                })
                .count();
            println!("ink_frames|{ink}");
        }
        Cmd::CropX {
            track,
            scaled_w,
            crop_w,
            jump,
        } => {
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
        Cmd::Autopilot {
            max_videos,
            skip_source,
            no_clip,
            no_hook,
        } => {
            let results = autopilot::run(
                &conn,
                &root,
                &autopilot::RunOpts {
                    max_videos,
                    skip_source,
                    do_clip: !no_clip,
                    hook_cta: !no_hook,
                    lanes: autopilot::DEFAULT_LANES,
                },
            )?;
            // Mirror cli.py `_cmd_autopilot`: exit 1 if any stage failed.
            if results.iter().any(|r| !r.ok) {
                std::process::exit(1);
            }
        }
        Cmd::Serve { port } => {
            // tokio runtime — axum needs it. The DB conn opened above is dropped here;
            // the editor doesn't use it (state lives in data/editor/).
            drop(conn);
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(server::run(&root, port))?;
        }
    }
    Ok(())
}
