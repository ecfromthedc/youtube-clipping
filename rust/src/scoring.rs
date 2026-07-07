//! Scoring engine — the deterministic core. Mirrors src/ycp/scoring.py: per-clip 0-100
//! virality score, then rollups by dimension + scale/kill verdicts. Pure, reproducible.
use std::collections::BTreeMap;

use chrono::{DateTime, Timelike};
use chrono_tz::Tz;

use crate::db::ClipRow;

#[derive(Clone, Debug, Default)]
pub struct Weights {
    pub views_7d: f64,
    pub retention: f64,
    pub revenue_per_1k: f64,
}

pub struct ScoreCfg {
    pub weights: Weights,
    pub length_buckets: Vec<i64>,
    pub min_sample: i64,
    pub scale_q: f64,
    pub kill_q: f64,
    pub timezone: String,
}

impl ScoreCfg {
    pub fn from_settings(s: &serde_yaml::Value) -> Self {
        let sc = &s["scoring"];
        let w = &sc["weights"];
        let f = |v: &serde_yaml::Value, k: &str, d: f64| v[k].as_f64().unwrap_or(d);
        let buckets = sc["length_buckets"]
            .as_sequence()
            .map(|seq| seq.iter().filter_map(|x| x.as_i64()).collect())
            .unwrap_or_else(|| vec![0, 15, 25, 35, 45, 60]);
        ScoreCfg {
            weights: Weights {
                views_7d: f(w, "views_7d", 0.5),
                retention: f(w, "retention", 0.3),
                revenue_per_1k: f(w, "revenue_per_1k", 0.2),
            },
            length_buckets: buckets,
            min_sample: sc["min_sample"].as_i64().unwrap_or(4),
            scale_q: f(sc, "scale_quantile", 0.8),
            kill_q: f(sc, "kill_quantile", 0.2),
            timezone: s["distribution"]["postiz"]["timezone"]
                .as_str()
                .unwrap_or("UTC")
                .to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Scored {
    #[allow(dead_code)] // parity with the Python dict shape; read at cutover
    pub clip_id: String,
    pub source_creator: String,
    pub fmt: String,
    pub hook_type: String,
    pub platform: String,
    pub length_bucket: String,
    pub post_hour: Option<i64>,
    pub views: f64,
    pub virality_score: f64,
    pub ad_revenue: f64,
    pub swipe_away_pct: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct Rollup {
    pub key: String,
    pub n: i64,
    pub avg_score: f64,
    pub avg_views: f64,
    pub total_revenue: f64,
}

pub struct Analysis {
    pub scored: Vec<Scored>,
    pub by_combo: Vec<Rollup>,
    pub by_creator: Vec<Rollup>,
    pub by_format: Vec<Rollup>,
    pub by_hook: Vec<Rollup>,
    pub by_length: Vec<Rollup>,
    pub by_platform: Vec<Rollup>,
    pub by_hour: Vec<Rollup>,
    pub scale: Vec<Rollup>,
    pub kill: Vec<Rollup>,
}

fn length_bucket(seconds: Option<i64>, buckets: &[i64]) -> String {
    let s = match seconds {
        Some(v) => v,
        None => return "unknown".into(),
    };
    for w in buckets.windows(2) {
        if w[0] <= s && s < w[1] {
            return format!("{}-{}s", w[0], w[1]);
        }
    }
    format!("{}s+", buckets.last().copied().unwrap_or(0))
}

fn post_hour(posted_at: &Option<String>, tz: &str) -> Option<i64> {
    let raw = posted_at.as_ref()?;
    let dt = DateTime::parse_from_rfc3339(raw).ok()?;
    let zone: Tz = tz.parse().unwrap_or(chrono_tz::UTC);
    Some(dt.with_timezone(&zone).hour() as i64)
}

/// Scale a slice to 0..1. Constant or empty → 0.5 (neutral, no signal). Mirrors _minmax.
fn minmax(vals: &[f64]) -> Vec<f64> {
    if vals.is_empty() {
        return vec![];
    }
    let lo = vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if !lo.is_finite() || !hi.is_finite() || hi == lo {
        return vec![0.5; vals.len()];
    }
    vals.iter().map(|v| (v - lo) / (hi - lo)).collect()
}

pub fn compute_scores(rows: &[ClipRow], cfg: &ScoreCfg) -> Vec<Scored> {
    if rows.is_empty() {
        return vec![];
    }
    let views: Vec<f64> = rows
        .iter()
        .map(|r| (r.views.max(0) as f64).ln_1p())
        .collect();
    let ret: Vec<f64> = rows
        .iter()
        .map(|r| r.retention_pct.unwrap_or(0.0).max(0.0))
        .collect();
    let rev1k: Vec<f64> = rows
        .iter()
        .map(|r| {
            let v = r.views.max(0) as f64;
            if v > 0.0 {
                r.ad_revenue / (v / 1000.0)
            } else {
                0.0
            }
        })
        .collect();
    let (nv, nr, nrev) = (minmax(&views), minmax(&ret), minmax(&rev1k));
    let w = &cfg.weights;
    rows.iter()
        .enumerate()
        .map(|(i, r)| {
            let score = w.views_7d * nv[i] + w.retention * nr[i] + w.revenue_per_1k * nrev[i];
            Scored {
                clip_id: r.clip_id.clone(),
                source_creator: r.source_creator.clone().unwrap_or_default(),
                fmt: r.fmt.clone().unwrap_or_default(),
                hook_type: r.hook_type.clone().unwrap_or_default(),
                platform: r.platform.clone().unwrap_or_default(),
                length_bucket: length_bucket(r.length_sec, &cfg.length_buckets),
                post_hour: post_hour(&r.posted_at, &cfg.timezone),
                views: r.views as f64,
                // Python `(score*100).round(1)` — round-half-to-even via util::round_to.
                virality_score: crate::util::round_to(score * 100.0, 1),
                ad_revenue: r.ad_revenue,
                swipe_away_pct: r.swipe_away_pct,
            }
        })
        .collect()
}

/// Group by a key, keep combos with >= min_sample clips, sort by avg_score desc.
pub fn rollup<F: Fn(&Scored) -> String>(scored: &[Scored], key: F, min_sample: i64) -> Vec<Rollup> {
    let mut groups: BTreeMap<String, (i64, f64, f64, f64)> = BTreeMap::new();
    for s in scored {
        let e = groups.entry(key(s)).or_insert((0, 0.0, 0.0, 0.0));
        e.0 += 1;
        e.1 += s.virality_score;
        e.2 += s.views;
        e.3 += s.ad_revenue;
    }
    let mut out: Vec<Rollup> = groups
        .into_iter()
        .filter(|(_, (n, ..))| *n >= min_sample)
        .map(|(key, (n, sc, vw, rev))| Rollup {
            // Python pandas `.round(n)` is round-half-to-even; util::round_to reproduces it.
            // Rust's bare `.round()` is round-half-away → diverges on exact .5 ties (e.g. an
            // avg_views of 0.5 → 0 in Python but 1 with bare round()). Surfaced by autopilot.
            key,
            n,
            avg_score: crate::util::round_to(sc / n as f64, 1),
            avg_views: crate::util::round_to(vw / n as f64, 0),
            total_revenue: crate::util::round_to(rev, 2),
        })
        .collect();
    out.sort_by(|a, b| b.avg_score.partial_cmp(&a.avg_score).unwrap());
    out
}

/// Linear-interpolation quantile (matches pandas default).
fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let pos = q * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
    }
}

pub fn scale_and_kill(rolled: &[Rollup], scale_q: f64, kill_q: f64) -> (Vec<Rollup>, Vec<Rollup>) {
    if rolled.is_empty() {
        return (vec![], vec![]);
    }
    let mut scores: Vec<f64> = rolled.iter().map(|r| r.avg_score).collect();
    scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let hi = quantile(&scores, scale_q);
    let lo = quantile(&scores, kill_q);
    let scale: Vec<Rollup> = rolled
        .iter()
        .filter(|r| r.avg_score >= hi)
        .cloned()
        .collect();
    let mut kill: Vec<Rollup> = rolled
        .iter()
        .filter(|r| r.avg_score <= lo)
        .cloned()
        .collect();
    kill.sort_by(|a, b| a.avg_score.partial_cmp(&b.avg_score).unwrap());
    (scale, kill)
}

pub fn analyze(rows: &[ClipRow], cfg: &ScoreCfg) -> Analysis {
    let scored = compute_scores(rows, cfg);
    let by_combo = rollup(
        &scored,
        |s| format!("{} · {} · {}", s.source_creator, s.fmt, s.hook_type),
        cfg.min_sample,
    );
    let (scale, kill) = scale_and_kill(&by_combo, cfg.scale_q, cfg.kill_q);
    Analysis {
        by_creator: rollup(&scored, |s| s.source_creator.clone(), cfg.min_sample),
        by_format: rollup(&scored, |s| s.fmt.clone(), cfg.min_sample),
        by_hook: rollup(&scored, |s| s.hook_type.clone(), cfg.min_sample),
        by_length: rollup(&scored, |s| s.length_bucket.clone(), cfg.min_sample),
        by_platform: rollup(&scored, |s| s.platform.clone(), cfg.min_sample),
        by_hour: rollup(
            &scored
                .iter()
                .filter(|s| s.post_hour.is_some())
                .cloned()
                .collect::<Vec<_>>(),
            |s| s.post_hour.unwrap().to_string(),
            cfg.min_sample,
        ),
        by_combo,
        scale,
        kill,
        scored,
    }
}
