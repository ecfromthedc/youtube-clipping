//! A/B hook testing — crown the winning hook angle on hero clips. Mirrors src/ycp/experiment.py.
//!
//! A hero moment (top predicted score) is cut into several variants — the SAME clip with a
//! DIFFERENT hook style sharing an experiment_id. Once the variants have real views, resolve()
//! picks the winner by views, journals it, and feeds the winning hook style into optimize's
//! PROVEN preferences so generation biases toward what actually won.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;

use crate::db::ClipRow;
use crate::optimize::{self, Paths};

/// pandas `str(None)`/`str(nan)`/empty — hook labels that carry no creative signal.
const NON_CREATIVE: &[&str] = &["None", "", "nan"];

#[derive(Debug, Clone, PartialEq)]
pub struct Winner {
    pub experiment: String,
    pub winning_hook: String,
    pub winning_views: i64,
    pub variants: i64,
    /// Raw top/runner-up ratio. Always emitted rounded via `{:.1}` — matches Python
    /// `str(round(x, 1))` exactly (single half-to-even round of the same ratio).
    pub margin: f64,
}

/// For each experiment with >=2 posted variants and enough views, the winning variant.
/// Pure read of clips+latest-metrics rows. Mirrors experiment.winners().
pub fn winners(clips: &[ClipRow], min_views: i64) -> Vec<Winner> {
    // Group non-null experiment_id → rows in original order; BTreeMap iterates by sorted key
    // to match pandas `groupby(sort=True)`.
    let mut groups: BTreeMap<String, Vec<&ClipRow>> = BTreeMap::new();
    for c in clips {
        if let Some(exp) = c.experiment_id.as_deref() {
            groups.entry(exp.to_string()).or_default().push(c);
        }
    }
    let mut out = Vec::new();
    for (exp_id, grp) in &groups {
        let posted: Vec<&ClipRow> = grp
            .iter()
            .copied()
            .filter(|c| c.status.as_deref() == Some("posted"))
            .collect();
        if posted.len() < 2 {
            continue;
        }
        let max_views = posted.iter().map(|c| c.views).max().unwrap_or(0);
        if max_views < min_views {
            continue;
        }
        let mut ranked: Vec<i64> = posted.iter().map(|c| c.views).collect();
        ranked.sort_unstable_by(|a, b| b.cmp(a)); // descending
        let second = ranked[1];
        // idxmax: the FIRST posted row achieving the max (pandas returns first occurrence).
        let win = posted.iter().find(|c| c.views == max_views).unwrap();
        out.push(Winner {
            experiment: exp_id.clone(),
            winning_hook: win.hook_type.clone().unwrap_or_else(|| "None".to_string()),
            winning_views: max_views,
            variants: posted.len() as i64,
            margin: max_views as f64 / second.max(1) as f64,
        });
    }
    out
}

fn read_json_str_array(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<String>>(&t).ok())
        .unwrap_or_default()
}

/// Reproduce Python `json.dumps([...])` for a list of strings byte-for-byte: `, ` separators,
/// each element JSON-escaped. (Hook labels are ASCII, so ensure_ascii is a no-op here.)
fn py_json_str_array(items: &[String]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|s| serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()))
        .collect();
    format!("[{}]", parts.join(", "))
}

/// Crown NEW A/B winners (idempotent): journal them + bias generation toward the winning hook
/// styles (optimize's PROVEN list). Returns the freshly-resolved winners. Mirrors resolve().
pub fn resolve(root: &Path, clips: &[ClipRow], min_views: i64) -> Result<Vec<Winner>> {
    let paths = Paths::new(root);
    let resolved_path = root.join("data").join("resolved-experiments.json");
    let done: BTreeSet<String> = read_json_str_array(&resolved_path).into_iter().collect();
    let fresh: Vec<Winner> = winners(clips, min_views)
        .into_iter()
        .filter(|w| !done.contains(&w.experiment))
        .collect();
    if fresh.is_empty() {
        return Ok(Vec::new());
    }
    // PROVEN = winning hooks (minus non-creative labels) ++ existing, first-occurrence dedup.
    let mut proven: Vec<String> = Vec::new();
    let won = fresh
        .iter()
        .map(|w| w.winning_hook.clone())
        .filter(|h| !NON_CREATIVE.contains(&h.as_str()))
        .collect::<Vec<_>>();
    for h in won
        .iter()
        .chain(read_json_str_array(&paths.ab_winners).iter())
    {
        if !proven.contains(h) {
            proven.push(h.clone());
        }
    }
    if let Some(d) = paths.ab_winners.parent() {
        std::fs::create_dir_all(d).ok();
    }
    std::fs::write(&paths.ab_winners, py_json_str_array(&proven))?;

    let mut lines = vec![String::new(), "### A/B winners".to_string()];
    for w in &fresh {
        lines.push(format!(
            "- **{}** hook won {} — {} views, {:.1}× runner-up across {} angles",
            w.winning_hook,
            w.experiment,
            crate::util::comma(w.winning_views),
            w.margin,
            w.variants,
        ));
    }
    lines.push("- → added to PROVEN hook styles; generation now biases toward them.".to_string());
    optimize::append_log(&paths, &lines.join("\n"))?;

    let mut all = done;
    all.extend(fresh.iter().map(|w| w.experiment.clone()));
    std::fs::write(
        &resolved_path,
        py_json_str_array(&all.into_iter().collect::<Vec<_>>()),
    )?;
    Ok(fresh)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(exp: &str, status: &str, hook: &str, views: i64) -> ClipRow {
        ClipRow {
            experiment_id: Some(exp.to_string()),
            status: Some(status.to_string()),
            hook_type: Some(hook.to_string()),
            views,
            ..Default::default()
        }
    }

    #[test]
    fn winners_picks_top_posted_variant() {
        let clips = vec![
            clip("e1", "posted", "cold_open", 5000),
            clip("e1", "posted", "clipped_quote", 2000),
            clip("e1", "pending_qc", "stat_shock", 9999), // not posted → ignored
            clip("e2", "posted", "cold_open", 500),       // below min_views → skipped
            clip("e2", "posted", "clipped_quote", 400),
            clip("e3", "posted", "cold_open", 3000), // single posted variant → skipped
        ];
        let w = winners(&clips, 1000);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].experiment, "e1");
        assert_eq!(w[0].winning_hook, "cold_open");
        assert_eq!(w[0].winning_views, 5000);
        assert_eq!(w[0].variants, 2);
        assert_eq!(format!("{:.1}", w[0].margin), "2.5");
    }

    #[test]
    fn resolve_is_idempotent_and_dedupes() {
        let dir = std::env::temp_dir().join(format!("ycp-exp-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("data")).unwrap();
        let clips = vec![
            clip("e1", "posted", "cold_open", 5000),
            clip("e1", "posted", "clipped_quote", 2000),
        ];
        let first = resolve(&dir, &clips, 1000).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(
            std::fs::read_to_string(dir.join("data/ab-winners.json")).unwrap(),
            "[\"cold_open\"]"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("data/resolved-experiments.json")).unwrap(),
            "[\"e1\"]"
        );
        // Already resolved → second pass yields nothing fresh.
        assert!(resolve(&dir, &clips, 1000).unwrap().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
