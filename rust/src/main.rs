//! `ycp` — YouTube clipping closed-loop ops (Rust port, in progress).
//! The Python in ../src/ycp stays the live system until this reaches parity.
mod config;
mod db;
mod optimize;
mod scoring;
mod util;

use std::collections::BTreeMap;

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
    Scoreboard,
    /// Learned source weights + creative preferences (cross-checks optimize.py).
    Optimize,
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
            let posted = clips.iter().filter(|c| c.status.as_deref() == Some("posted")).count();
            let views: i64 = clips.iter().map(|c| c.views).sum();
            println!("ycp (rust) · {}", root.display());
            println!("clips: {} total · {} posted · {} views", clips.len(), posted, views);
            let mut by: BTreeMap<String, usize> = BTreeMap::new();
            for c in &clips {
                *by.entry(c.status.clone().unwrap_or_default()).or_default() += 1;
            }
            for (status, n) in by {
                println!("  {status:<12} {n}");
            }
        }
        Cmd::Scoreboard => {
            let settings = config::load_settings(&root)?;
            let cfg = scoring::ScoreCfg::from_settings(&settings);
            let clips = db::clips_with_latest_metrics(&conn)?;
            let a = scoring::analyze(&clips, &cfg);
            println!("scored {} clips · top creators by virality:", a.scored.len());
            for r in a.by_creator.iter().take(5) {
                println!(
                    "  {:<26} score {:>5.1} · {:>9.0} views · n={}",
                    r.key, r.avg_score, r.avg_views, r.n
                );
            }
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
    }
    Ok(())
}
