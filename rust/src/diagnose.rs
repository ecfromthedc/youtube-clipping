//! WHY analysis — the causal layer on top of the quantitative scoreboard. Mirrors
//! src/ycp/diagnose.py.
//!
//! scoring/brief RANK what's winning; this asks WHY and returns prose (causal patterns +
//! 3 testable creative changes) via DeepSeek. Deliberately OPTIONAL and non-blocking:
//! returns None without a key or without enough data, so the deterministic brief + cron
//! never depend on it. Reuses the same DeepSeek plumbing as the hook agent.
//!
//! Parity note: without DEEPSEEK_API_KEY (or <6 scored clips) this returns None — the exact
//! path autopilot's brief stage takes on a no-creds run, so the brief is byte-identical there.
//! The live POST mirrors hooks.rs structurally; its output is model text (not byte-diffable).
use std::path::Path;

use crate::config;
use crate::scoring::{Analysis, Rollup};

const DEFAULT_MODEL: &str = "deepseek-chat";
const DEEPSEEK_URL: &str = "https://api.deepseek.com/chat/completions";
const MIN_CLIPS: usize = 6;

const SYSTEM: &str =
    "You are the performance analyst for a faceless YouTube Shorts factory. You are given \
aggregated clip performance (virality score 0-100, views, by creator / hook style / \
length / format) AND hook drop-off by style (% of viewers gone by the end of the hook — \
the single sharpest signal of whether a hook is working). Do NOT restate the numbers. \
Explain WHY the winners win and the losers lose — the causal pattern in hook style, \
topic, emotional trigger, length, and opening moment, leaning on the drop-off data to \
pinpoint whether clips fail at the hook or mid-clip. Be specific and falsifiable. Then \
give exactly 3 concrete, testable creative changes for next cycle (e.g. 'open more hooks \
with a number', 'cut 30-45s clips to 20-30s', 'lead with loss not curiosity for finance'). \
Format as short markdown: a '**Why:**' paragraph then a '**Do next:**' list of 3. <180 words.";

fn rollup_lines(name: &str, rs: &[Rollup], key_label: &str) -> String {
    if rs.is_empty() {
        return format!("{name}: (no data)");
    }
    let parts: Vec<String> = rs
        .iter()
        .take(5)
        .map(|r| {
            format!(
                "{} score={:.0} views={} n={}",
                r.key, r.avg_score, r.avg_views as i64, r.n
            )
        })
        .collect();
    // key_label kept for parity with Python's per-table key column; unused in the joined line.
    let _ = key_label;
    format!("{name}: {}", parts.join(" | "))
}

/// Mean hook drop-off (% gone by the hook's end) by hook style — the causal 'why' signal.
fn retention_line(scored: &[crate::scoring::Scored]) -> String {
    use std::collections::BTreeMap;
    let mut acc: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for s in scored {
        if let Some(v) = s.swipe_away_pct {
            let e = acc.entry(s.hook_type.clone()).or_insert((0.0, 0));
            e.0 += v;
            e.1 += 1;
        }
    }
    if acc.is_empty() {
        return "Hook drop-off: (no retention data yet)".to_string();
    }
    let mut means: Vec<(String, f64)> = acc
        .into_iter()
        .map(|(k, (sum, n))| (k, sum / n as f64))
        .collect();
    means.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let body: Vec<String> = means.iter().map(|(k, v)| format!("{k} {v:.0}%")).collect();
    format!(
        "Hook drop-off by style (lower=better, % gone by hook's end): {}",
        body.join(" | ")
    )
}

fn facts(a: &Analysis) -> String {
    [
        rollup_lines("By creator", &a.by_creator, "source_creator"),
        rollup_lines("By hook style", &a.by_hook, "hook_type"),
        rollup_lines("By length", &a.by_length, "length_bucket"),
        rollup_lines("By format", &a.by_format, "fmt"),
        rollup_lines("By posting hour (channel-local)", &a.by_hour, "post_hour"),
        retention_line(&a.scored),
    ]
    .join("\n")
}

/// Markdown WHY analysis, or None if no key / not enough data / API failure. Mirrors `diagnose`.
pub fn diagnose(root: &Path, settings: &serde_yaml::Value, a: &Analysis) -> Option<String> {
    let key = config::env_var(root, "DEEPSEEK_API_KEY")?;
    if a.scored.len() < MIN_CLIPS {
        return None;
    }
    let model = settings["hooks"]["model"].as_str().unwrap_or(DEFAULT_MODEL);
    let body = serde_json::json!({
        "model": model,
        "temperature": 0.7,
        "max_tokens": 500,
        "messages": [
            {"role": "system", "content": SYSTEM},
            {"role": "user", "content": facts(a)},
        ],
    });
    let resp = reqwest::blocking::Client::new()
        .post(DEEPSEEK_URL)
        .header("Authorization", format!("Bearer {key}"))
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(40))
        .json(&body)
        .send()
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: serde_json::Value = resp.json().ok()?;
    let text = v["choices"][0]["message"]["content"]
        .as_str()?
        .trim()
        .to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}
