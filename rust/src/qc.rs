//! Stage 3 — APPROVE, channel-agnostic. Mirrors src/ycp/qc.py.
//!
//! Two knobs in settings (`qc:`):
//!   • `auto: true`  → no human; the in-code guardrails decide (`distribute::auto_qc`).
//!   • `auto: false` → a human ✅/❌ via `channel` (auto picks slack > telegram > local).
//!
//! Parity scope: `auto` and `local` (the configured default + the offline-safe path) are ported
//! faithfully. `slack`/`telegram` delegate to the Python `slack_qc` listener / Telegram Bot API —
//! live, creds-gated, not byte-diffable; not ported here (the qc ceiling). The shipped config is
//! `qc.auto: true`, so the path autopilot actually runs (auto-guardrails) is exact.
use std::path::Path;

use anyhow::{bail, Result};
use rusqlite::Connection;

use crate::{config, db, distribute};

/// Result of dispatching pending clips (mirrors the dict qc.py `dispatch_pending` returns).
pub enum Dispatch {
    /// `qc.auto` — in-code guardrails decided.
    Auto { approved: i64, rejected: i64 },
    /// A human review channel was sent `dispatched` clips.
    Human { channel: String, dispatched: i64 },
}

fn qc_cfg(settings: &serde_yaml::Value) -> &serde_yaml::Value {
    &settings["qc"]
}

/// Which human channel to use; `auto` picks by whatever creds are configured. Mirrors
/// `resolve_channel` (slack > telegram > local).
pub fn resolve_channel(root: &Path, settings: &serde_yaml::Value) -> String {
    let name = qc_cfg(settings)["channel"]
        .as_str()
        .unwrap_or("auto")
        .to_lowercase();
    if name != "auto" {
        return name;
    }
    let has = |k: &str| config::env_var(root, k).is_some();
    if has("SLACK_BOT_TOKEN") && has("SLACK_QC_CHANNEL") {
        "slack".to_string()
    } else if has("TELEGRAM_BOT_TOKEN") && has("TELEGRAM_QC_CHAT") {
        "telegram".to_string()
    } else {
        "local".to_string()
    }
}

/// Send pending clips for review (or auto-approve via guardrails when `qc.auto`). Mirrors
/// `dispatch_pending`.
pub fn dispatch_pending(conn: &Connection, root: &Path) -> Result<Dispatch> {
    let settings = config::load_settings(root)?;
    if qc_cfg(&settings)["auto"].as_bool().unwrap_or(false) {
        let (approved, rejected) = distribute::auto_qc(conn)?;
        return Ok(Dispatch::Auto { approved, rejected });
    }
    let name = resolve_channel(root, &settings);
    let dispatched = dispatch_human(conn, root, &name)?;
    Ok(Dispatch::Human {
        channel: name,
        dispatched,
    })
}

fn dispatch_human(conn: &Connection, root: &Path, channel: &str) -> Result<i64> {
    match channel {
        "local" => dispatch_local(conn, root),
        "slack" | "telegram" => bail!(
            "qc channel {channel:?} delegates to the Python slack_qc/Telegram listener — not \
             ported to the Rust binary (set qc.auto: true, or qc.channel: local)"
        ),
        other => bail!("unknown qc channel {other:?} (use auto|slack|telegram|local)"),
    }
}

/// No external service — write a review manifest; approve with `ycp qc-approve <id>`.
/// Mirrors `_LocalChannel.dispatch`.
fn dispatch_local(conn: &Connection, root: &Path) -> Result<i64> {
    let clips = db::pending_qc_clips(conn)?;
    let mut lines = vec!["# QC review — pending clips".to_string(), String::new()];
    for c in &clips {
        let creator = c.source_creator.as_deref().unwrap_or("?");
        let len = c
            .length_sec
            .map(|n| n.to_string())
            .unwrap_or_else(|| "?".to_string());
        let file = c.post_url.as_deref().unwrap_or("");
        lines.push(format!(
            "- **{id}** · {creator} · {len}s\n  - file: `{file}`\n  - approve: `ycp qc-approve {id}`  ·  reject: `ycp qc-reject {id}`",
            id = c.clip_id
        ));
    }
    let path = root.join("data").join("qc-review.md");
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).ok();
    }
    std::fs::write(&path, lines.join("\n") + "\n")?;
    println!("→ {} clips for review in {}", clips.len(), path.display());
    for c in &clips {
        println!("   {}  {}", c.clip_id, c.post_url.as_deref().unwrap_or(""));
    }
    Ok(clips.len() as i64)
}
