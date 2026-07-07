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
        Factors {
            boost: f("boost", 1.5),
            suppress: f("suppress", 0.4),
            floor: f("floor", 0.1),
        }
    }
}

/// Per-creator source multiplier: scaled winners boosted, killed losers suppressed (kill
/// applied first so a creator in both ends up boosted). Pure.
pub fn creator_weights(
    a: &Analysis,
    f: &Factors,
    scale_q: f64,
    kill_q: f64,
) -> BTreeMap<String, f64> {
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

pub fn save_creative(
    p: &Paths,
    prefer_hooks: &[String],
    prefer_length: &Option<String>,
) -> Result<()> {
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
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
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

/// Top-N rollup rows as `key (avg_score)` (mirrors `_top_rows`; "—" when empty).
fn top_rows(rs: &[crate::scoring::Rollup], n: usize) -> String {
    if rs.is_empty() {
        return "—".to_string();
    }
    rs.iter()
        .take(n)
        .map(|r| format!("{} ({:.0})", r.key, r.avg_score))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn format_entry(a: &Analysis, weights: &BTreeMap<String, f64>, today: &str) -> String {
    let total_views = a.scored.iter().map(|s| s.views).sum::<f64>() as i64;
    let (prefer_hooks, prefer_length) = creative_prefs(a);
    let boosted: Vec<&String> = weights
        .iter()
        .filter(|(_, v)| **v > 1.0)
        .map(|(k, _)| k)
        .collect();
    let cut: Vec<&String> = weights
        .iter()
        .filter(|(_, v)| **v < 1.0)
        .map(|(k, _)| k)
        .collect();
    let join = |v: &[&String]| v.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
    [
        format!("## {today}"),
        format!(
            "- **Sampled:** {} clips · {} total views so far.",
            a.scored.len(),
            crate::util::comma(total_views)
        ),
        format!("- **Top creators:** {}", top_rows(&a.by_creator, 3)),
        format!(
            "- **Top formats:** {} · **lengths:** {}",
            top_rows(&a.by_format, 3),
            top_rows(&a.by_length, 3)
        ),
        format!(
            "- **Winning hook styles:** {} · **best length:** {} → fed back into hook generation.",
            if prefer_hooks.is_empty() {
                "— (not learned yet)".to_string()
            } else {
                prefer_hooks.join(", ")
            },
            prefer_length.unwrap_or_else(|| "—".into())
        ),
        format!(
            "- **Doubling down on:** {}",
            if boosted.is_empty() {
                "— (not enough signal yet)".to_string()
            } else {
                join(&boosted)
            }
        ),
        format!(
            "- **Starving:** {}",
            if cut.is_empty() {
                "—".to_string()
            } else {
                join(&cut)
            }
        ),
        "- **Why:** winners (top-quantile virality) get sourced harder next cycle + their \
         hook styles bias generation; losers get throttled. → learned-weights/creative.json."
            .to_string(),
    ]
    .join("\n")
}

const LOG_HEADER: &str = "# Improvement Log\n\n\
    _Auto-appended by the OPTIMIZE stage each cycle. Newest entries at the bottom._\n\
    _North star: 100M impressions / month. The loop doubles down on what wins._\n";

pub fn append_log(p: &Paths, entry: &str) -> Result<()> {
    let prior = std::fs::read_to_string(&p.log).unwrap_or_else(|_| LOG_HEADER.to_string());
    std::fs::write(&p.log, format!("{}\n\n{}\n", prior.trim_end(), entry))?;
    Ok(())
}

/// What `run` learned this cycle (mirrors the dict optimize.py `run` returns; autopilot reads
/// `clips`/`boosted`/`suppressed`). boosted/suppressed are sorted (BTreeMap key order).
pub struct RunSummary {
    pub clips: usize,
    pub boosted: Vec<String>,
    pub suppressed: Vec<String>,
}

/// Analyze captured metrics → learn source weights + creative prefs → persist + journal.
/// Mirrors optimize.py `run`. Reads scale_q/kill_q + boost/suppress/floor from settings.
pub fn run(conn: &rusqlite::Connection, root: &Path, today: &str) -> Result<RunSummary> {
    let settings = crate::config::load_settings(root)?;
    let cfg = crate::scoring::ScoreCfg::from_settings(&settings);
    let factors = Factors::from_settings(&settings);
    let clips = crate::db::clips_with_latest_metrics(conn)?;
    let a = crate::scoring::analyze(&clips, &cfg);
    let weights = creator_weights(&a, &factors, cfg.scale_q, cfg.kill_q);
    let (prefer_hooks, prefer_length) = creative_prefs(&a);
    let p = Paths::new(root);
    save_weights(&p, &weights)?;
    save_creative(&p, &prefer_hooks, &prefer_length)?;
    append_log(&p, &format_entry(&a, &weights, today))?;
    Ok(RunSummary {
        clips: a.scored.len(),
        boosted: weights
            .iter()
            .filter(|(_, v)| **v > 1.0)
            .map(|(k, _)| k.clone())
            .collect(),
        suppressed: weights
            .iter()
            .filter(|(_, v)| **v < 1.0)
            .map(|(k, _)| k.clone())
            .collect(),
    })
}
