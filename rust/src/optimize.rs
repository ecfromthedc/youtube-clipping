//! OPTIMIZE actuator + learned preferences. Mirrors src/ycp/optimize.py: turn the
//! scoreboard's scale/kill verdicts into per-creator source multipliers + winning hook
//! styles/length, persist them, and journal to IMPROVEMENT-LOG.md.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::scoring::{scale_and_kill, Analysis};

const NON_CREATIVE: &[&str] = &["tbd", "heuristic", "manual", "uncategorized", "", "nan"];

pub struct Paths {
    pub weights: PathBuf,
    pub creative: PathBuf,
    pub ab_winners: PathBuf,
    pub log: PathBuf,
}

impl Paths {
    pub fn new(root: &Path) -> Self {
        let d = root.join("data");
        Paths {
            weights: d.join("learned-weights.json"),
            creative: d.join("learned-creative.json"),
            ab_winners: d.join("ab-winners.json"),
            log: root.join("IMPROVEMENT-LOG.md"),
        }
    }
}

pub struct Factors {
    pub boost: f64,
    pub suppress: f64,
    pub floor: f64,
}

impl Factors {
    pub fn from_settings(s: &serde_yaml::Value) -> Self {
        let o = &s["optimize"];
        let f = |k: &str, d: f64| o[k].as_f64().unwrap_or(d);
        Factors { boost: f("boost", 1.5), suppress: f("suppress", 0.4), floor: f("floor", 0.1) }
    }
}

/// Per-creator source multiplier: scaled winners boosted, killed losers suppressed (kill
/// applied first so a creator in both ends up boosted). Pure.
pub fn creator_weights(a: &Analysis, f: &Factors, scale_q: f64, kill_q: f64) -> BTreeMap<String, f64> {
    let mut w = BTreeMap::new();
    if a.by_creator.is_empty() {
        return w;
    }
    let (scale, kill) = scale_and_kill(&a.by_creator, scale_q, kill_q);
    for r in &kill {
        w.insert(r.key.clone(), f.floor.max(f.suppress));
    }
    for r in &scale {
        w.insert(r.key.clone(), f.boost);
    }
    w
}

/// Winning creative levers: top-2 hook styles (excluding non-creative labels) + best length.
pub fn creative_prefs(a: &Analysis) -> (Vec<String>, Option<String>) {
    let prefer_hooks: Vec<String> = a
        .by_hook
        .iter()
        .take(2)
        .map(|r| r.key.clone())
        .filter(|h| !NON_CREATIVE.contains(&h.to_lowercase().as_str()))
        .collect();
    let prefer_length = a.by_length.first().map(|r| r.key.clone());
    (prefer_hooks, prefer_length)
}

fn read_json_array(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok())
        .unwrap_or_default()
}

pub fn save_weights(p: &Paths, weights: &BTreeMap<String, f64>) -> Result<()> {
    if let Some(d) = p.weights.parent() {
        std::fs::create_dir_all(d).ok();
    }
    std::fs::write(&p.weights, serde_json::to_string_pretty(weights)?)?;
    Ok(())
}

pub fn load_weights(p: &Paths) -> BTreeMap<String, f64> {
    std::fs::read_to_string(&p.weights)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_creative(p: &Paths, prefer_hooks: &[String], prefer_length: &Option<String>) -> Result<()> {
    let v = serde_json::json!({ "prefer_hooks": prefer_hooks, "prefer_length": prefer_length });
    std::fs::write(&p.creative, serde_json::to_string_pretty(&v)?)?;
    Ok(())
}

/// A/B-PROVEN winners first, then inferred winners from the scoreboard (mirrors preferred_hooks).
pub fn preferred_hooks(p: &Paths) -> Vec<String> {
    let creative: serde_json::Value = std::fs::read_to_string(&p.creative)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(serde_json::Value::Null);
    let inferred: Vec<String> = creative["prefer_hooks"]
        .as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let proven = read_json_array(&p.ab_winners);
    let mut out = proven;
    for h in inferred {
        if !out.contains(&h) {
            out.push(h);
        }
    }
    out
}

pub fn format_entry(a: &Analysis, weights: &BTreeMap<String, f64>, today: &str) -> String {
    let total_views: f64 = a.scored.iter().map(|s| s.views).sum();
    let (prefer_hooks, prefer_length) = creative_prefs(a);
    let boosted: Vec<&String> = weights.iter().filter(|(_, v)| **v > 1.0).map(|(k, _)| k).collect();
    let cut: Vec<&String> = weights.iter().filter(|(_, v)| **v < 1.0).map(|(k, _)| k).collect();
    let top = |rs: &[crate::scoring::Rollup], n: usize| {
        if rs.is_empty() {
            "—".to_string()
        } else {
            rs.iter().take(n).map(|r| format!("{} ({:.0})", r.key, r.avg_score)).collect::<Vec<_>>().join(", ")
        }
    };
    let join = |v: &[&String]| if v.is_empty() { "—".to_string() } else { v.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ") };
    format!(
        "## {today}\n- **Sampled:** {} clips · {} total views so far.\n- **Top creators:** {}\n\
         - **Winning hook styles:** {} · **best length:** {}\n- **Doubling down on:** {}\n\
         - **Starving:** {}\n- **Why:** winners get sourced harder + their hook styles bias \
         generation; losers throttled. → learned-weights/creative.json.",
        a.scored.len(),
        total_views as i64,
        top(&a.by_creator, 3),
        if prefer_hooks.is_empty() { "— (not learned yet)".into() } else { prefer_hooks.join(", ") },
        prefer_length.unwrap_or_else(|| "—".into()),
        if boosted.is_empty() { "— (not enough signal yet)".into() } else { join(&boosted) },
        join(&cut),
    )
}

pub fn append_log(p: &Paths, entry: &str) -> Result<()> {
    let header = "# Improvement Log — Phoenix Protocol clip factory\n";
    let prior = std::fs::read_to_string(&p.log).unwrap_or_else(|_| header.to_string());
    std::fs::write(&p.log, format!("{}\n\n{}\n", prior.trim_end(), entry))?;
    Ok(())
}
