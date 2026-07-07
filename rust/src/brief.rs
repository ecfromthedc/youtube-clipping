//! Weekly Double-Down Brief — deterministic markdown. Mirrors src/ycp/brief.py:
//! pandas decides scale/kill/test from the scoring rollups, rule-based prose references
//! the actual creators/formats/hooks. No LLM. The bottom tables reproduce pandas
//! `DataFrame.to_markdown()` (tabulate "pipe" format, decimal-aligned) byte-for-byte.
use crate::scoring::{Analysis, Rollup};
use crate::util::{money, views};

/// Python `str(float)` for the 1-decimal-rounded avg_score used in prose: 50.0 → "50.0".
fn round1(x: f64) -> String {
    format!("{x:.1}")
}

// ---------- number formatting: Python "{:g}" (C %g, 6 significant digits) ----------

fn strip_trailing_zeros(s: String) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Reproduce Python `"{:g}".format(x)` — what tabulate uses to format float cells.
fn fmt_g(x: f64) -> String {
    if x == 0.0 {
        return "0".to_string();
    }
    const PREC: i32 = 6;
    // Round to PREC significant digits in scientific form, then read the exponent —
    // avoids the log10 boundary bug that bites a naive %g.
    let sci = format!("{:.*e}", (PREC - 1) as usize, x); // e.g. "9.17000e1"
    let (mantissa, exp_str) = sci.split_once('e').expect("scientific form has 'e'");
    let exp: i32 = exp_str.parse().expect("exponent is an int");
    if (-4..PREC).contains(&exp) {
        let decimals = (PREC - 1 - exp).max(0) as usize;
        strip_trailing_zeros(format!("{x:.decimals$}"))
    } else {
        let m = strip_trailing_zeros(mantissa.to_string());
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{m}e{sign}{:02}", exp.abs())
    }
}

// ---------- pandas to_markdown (tabulate "pipe" format) ----------

const MIN_PADDING: usize = 2; // tabulate default — headers get +2 over content

/// tabulate `_afterpoint`: digits after the decimal point of a formatted number,
/// or -1 for an integer-looking string. Drives decimal alignment.
fn afterpoint(s: &str) -> i32 {
    let lower = s.to_ascii_lowercase();
    if !lower.contains('.') && !lower.contains('e') {
        return -1; // integer-like
    }
    let pos = s.find('.').or_else(|| lower.find('e'));
    match pos {
        Some(p) => (s.len() - p - 1) as i32,
        None => -1,
    }
}

/// Render rows as a GitHub-pipe markdown table the way pandas `to_markdown(index=False)` does:
/// numeric columns decimal-aligned + right-justified, string columns left-justified, headers
/// padded to `max(content, header_len + 2)`. `numeric[i]` marks column i as a number column.
fn table(headers: &[&str], numeric: &[bool], rows: &[Vec<String>]) -> String {
    let ncol = headers.len();
    // 1. Per numeric column, decimal-align the cells (append trailing spaces to int-like cells).
    let mut cells: Vec<Vec<String>> = rows.to_vec();
    for c in 0..ncol {
        if !numeric[c] {
            continue;
        }
        let maxdec = cells.iter().map(|r| afterpoint(&r[c])).max().unwrap_or(-1);
        for r in cells.iter_mut() {
            let pad = (maxdec - afterpoint(&r[c])).max(0) as usize;
            if pad > 0 {
                r[c].push_str(&" ".repeat(pad));
            }
        }
    }
    // 2. Column widths: max(content, header + MIN_PADDING).
    let widths: Vec<usize> = (0..ncol)
        .map(|c| {
            let content = cells
                .iter()
                .map(|r| r[c].chars().count())
                .max()
                .unwrap_or(0);
            content.max(headers[c].chars().count() + MIN_PADDING)
        })
        .collect();
    // 3. Header row (numeric → right, string → left).
    let pad = |s: &str, w: usize, right: bool| {
        let n = w.saturating_sub(s.chars().count());
        if right {
            format!("{}{}", " ".repeat(n), s)
        } else {
            format!("{}{}", s, " ".repeat(n))
        }
    };
    let join = |fields: Vec<String>| format!("| {} |", fields.join(" | "));
    let mut out = vec![join(
        (0..ncol)
            .map(|c| pad(headers[c], widths[c], numeric[c]))
            .collect(),
    )];
    // 4. Separator with alignment colons (segment width = colwidth + 2 padding).
    out.push(format!(
        "|{}|",
        (0..ncol)
            .map(|c| {
                let w = widths[c] + 2;
                if numeric[c] {
                    format!("{}:", "-".repeat(w - 1))
                } else {
                    format!(":{}", "-".repeat(w - 1))
                }
            })
            .collect::<Vec<_>>()
            .join("|")
    ));
    // 5. Data rows.
    for r in &cells {
        out.push(join(
            (0..ncol)
                .map(|c| pad(&r[c], widths[c], numeric[c]))
                .collect(),
        ));
    }
    out.join("\n")
}

/// A combo/format/hook/etc. table, or the placeholder when there's no sample.
fn rollup_table(headers: &[&str], numeric: &[bool], rows: Vec<Vec<String>>) -> String {
    if rows.is_empty() {
        return "_(no combos with enough sample yet)_".to_string();
    }
    table(headers, numeric, &rows)
}

/// by_combo keys are "creator · fmt · hook" — split back into the three table columns.
/// ponytail: assumes no creator/fmt/hook literally contains " · "; true for all real labels.
fn split_combo(key: &str) -> (String, String, String) {
    let mut it = key.splitn(3, " · ");
    (
        it.next().unwrap_or("").to_string(),
        it.next().unwrap_or("").to_string(),
        it.next().unwrap_or("").to_string(),
    )
}

fn combo_rows(rs: &[Rollup], limit: usize) -> Vec<Vec<String>> {
    rs.iter()
        .take(limit)
        .map(|r| {
            let (creator, fmt, hook) = split_combo(&r.key);
            vec![
                creator,
                fmt,
                hook,
                r.n.to_string(),
                fmt_g(r.avg_score),
                fmt_g(r.avg_views),
                fmt_g(r.total_revenue),
            ]
        })
        .collect()
}

fn labeled_rows(rs: &[Rollup], limit: usize, with_revenue: bool) -> Vec<Vec<String>> {
    rs.iter()
        .take(limit)
        .map(|r| {
            let mut row = vec![
                r.key.clone(),
                r.n.to_string(),
                fmt_g(r.avg_score),
                fmt_g(r.avg_views),
            ];
            if with_revenue {
                row.push(fmt_g(r.total_revenue));
            }
            row
        })
        .collect()
}

// ---------- prose sections ----------

fn scale_section(scale: &[Rollup], top_n: usize) -> String {
    if scale.is_empty() {
        return "Not enough sample yet to crown winners. Keep volume broad across \
                creators/formats and let the next capture build signal."
            .to_string();
    }
    scale
        .iter()
        .take(top_n)
        .map(|r| {
            format!(
                "- **Make 3× more — {} hook**  → score {}, {} avg views over {} clips ({}).",
                r.key,
                round1(r.avg_score),
                views(r.avg_views),
                r.n,
                money(r.total_revenue)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn kill_section(kill: &[Rollup], top_n: usize) -> String {
    if kill.is_empty() {
        return "Nothing is clearly dead yet — no combo is underperforming enough to cut."
            .to_string();
    }
    kill
        .iter()
        .take(top_n)
        .map(|r| {
            format!(
                "- **Stop — {} hook**  → score {}, only {} avg views over {} clips. Reallocate those edits.",
                r.key,
                round1(r.avg_score),
                views(r.avg_views),
                r.n
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn test_section(a: &Analysis) -> String {
    let top_hook = a.by_hook.first().map(|r| r.key.clone());
    let top_creator = a.by_creator.first().map(|r| r.key.clone());
    let top_fmt = a.by_format.first().map(|r| r.key.clone());
    let mut bets: Vec<String> = vec![];
    if let (Some(c), Some(h)) = (&top_creator, &top_hook) {
        bets.push(format!(
            "- Pair the best hook (**{h}**) with the best creator (**{c}**) if you haven't \
             already — verify the combo holds."
        ));
    }
    if let (Some(f), Some(_h)) = (&top_fmt, &top_hook) {
        bets.push(format!(
            "- Push **{f}** into a second creator to see if the format travels or is creator-specific."
        ));
    }
    if let Some(best_len) = a.by_length.first().map(|r| r.key.clone()) {
        bets.push(format!(
            "- Best length right now is **{best_len}** — test one bucket shorter to chase higher completion."
        ));
    }
    if bets.is_empty() {
        "Collect one more week of data before placing test bets.".to_string()
    } else {
        bets.join("\n")
    }
}

fn money_section(scored: &[crate::scoring::Scored]) -> String {
    if scored.is_empty() {
        return "No revenue captured yet.".to_string();
    }
    let ads: f64 = scored.iter().map(|s| s.ad_revenue).sum();
    // Group revenue by platform (alphabetical via BTreeMap), then stable sort by value desc.
    let mut by_plat: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
    for s in scored {
        *by_plat.entry(s.platform.clone()).or_insert(0.0) += s.ad_revenue;
    }
    let mut plats: Vec<(String, f64)> = by_plat.into_iter().collect();
    plats.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let plat_line = plats
        .iter()
        .map(|(k, v)| format!("{k} {}", money(*v)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "- Ad revenue: **{}**\n- By platform: {plat_line}\n- Ad revenue from owned channels — \
         protect their YPP status and scale the proven winners.",
        money(ads)
    )
}

fn retention_section(scored: &[crate::scoring::Scored]) -> String {
    let with: Vec<&crate::scoring::Scored> = scored
        .iter()
        .filter(|s| s.swipe_away_pct.is_some())
        .collect();
    if with.is_empty() {
        return "No retention data yet — accrues ~24-48h after the first posts.".to_string();
    }
    let mean: f64 = with.iter().map(|s| s.swipe_away_pct.unwrap()).sum::<f64>() / with.len() as f64;
    // Mean swipe-away per hook, ascending (best-holding first).
    let mut groups: std::collections::BTreeMap<String, (f64, usize)> =
        std::collections::BTreeMap::new();
    for s in &with {
        let e = groups.entry(s.hook_type.clone()).or_insert((0.0, 0));
        e.0 += s.swipe_away_pct.unwrap();
        e.1 += 1;
    }
    let mut g: Vec<(String, f64)> = groups
        .into_iter()
        .map(|(k, (sum, n))| (k, sum / n as f64))
        .collect();
    g.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let (best_h, best_v) = &g[0];
    let (worst_h, worst_v) = &g[g.len() - 1];
    format!(
        "- Avg hook drop-off: **{mean:.0}%** lost by the hook's end.\n- Best-holding hook: \
         **{best_h}** ({best_v:.0}%) · worst: **{worst_h}** ({worst_v:.0}%) → make more of the former."
    )
}

fn timing_section(by_hour: &[Rollup]) -> String {
    if by_hour.is_empty() {
        return "Not enough posts yet to read timing — publish times are being logged; \
                the pattern surfaces as volume builds over weeks."
            .to_string();
    }
    let parts = by_hour
        .iter()
        .take(3)
        .map(|r| {
            let hour: i64 = r.key.parse().unwrap_or(0);
            format!(
                "**{hour:02}:00** (score {}, {} clips)",
                round1(r.avg_score),
                r.n
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("Best posting hours so far (channel-local): {parts}")
}

/// Render the full Double-Down Brief as markdown (deterministic).
pub fn build(a: &Analysis, n_clips: usize, top_n: usize, week_start: &str) -> String {
    let combo = rollup_table(
        &[
            "source_creator",
            "fmt",
            "hook_type",
            "n",
            "avg_score",
            "avg_views",
            "total_revenue",
        ],
        &[false, false, false, true, true, true, true],
        combo_rows(&a.by_combo, top_n),
    );
    let by_format = rollup_table(
        &["fmt", "n", "avg_score", "avg_views", "total_revenue"],
        &[false, true, true, true, true],
        labeled_rows(&a.by_format, top_n, true),
    );
    let by_hook = rollup_table(
        &["hook_type", "n", "avg_score", "avg_views", "total_revenue"],
        &[false, true, true, true, true],
        labeled_rows(&a.by_hook, top_n, true),
    );
    let by_length = rollup_table(
        &["length_bucket", "n", "avg_score", "avg_views"],
        &[false, true, true, true],
        labeled_rows(&a.by_length, top_n, false),
    );
    let by_platform = rollup_table(
        &["platform", "n", "avg_score", "avg_views", "total_revenue"],
        &[false, true, true, true, true],
        labeled_rows(&a.by_platform, a.by_platform.len(), true),
    );

    format!(
        "# 📈 Double-Down Brief — week of {week_start}\n\n\
         _Generated from {n_clips} clips with metrics. Virality score 0–100; revenue in USD._\n\n\
         ## 🟢 Scale\n{}\n\n\
         ## 🔴 Kill\n{}\n\n\
         ## 🆕 Test\n{}\n\n\
         ## 💰 Where the money is\n{}\n\n\
         ## 🎯 Hook health (retention)\n{}\n\n\
         ## ⏰ Timing\n{}\n\n\
         ---\n\n\
         ### Top combos (creator × format × hook)\n{combo}\n\n\
         ### By format\n{by_format}\n\n\
         ### By hook\n{by_hook}\n\n\
         ### By length\n{by_length}\n\n\
         ### By platform\n{by_platform}\n\n\
         > Next step: this brief is next week's sourcing spec. Point Stage 1 at the 🟢 combos.\n",
        scale_section(&a.scale, top_n),
        kill_section(&a.kill, top_n),
        test_section(a),
        money_section(&a.scored),
        retention_section(&a.scored),
        timing_section(&a.by_hour),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_g_matches_python() {
        assert_eq!(fmt_g(50.0), "50");
        assert_eq!(fmt_g(91.7), "91.7");
        assert_eq!(fmt_g(18.0), "18");
        assert_eq!(fmt_g(431176.0), "431176");
        assert_eq!(fmt_g(8872.0), "8872");
        assert_eq!(fmt_g(103.49), "103.49");
        assert_eq!(fmt_g(0.0), "0");
        assert_eq!(fmt_g(4311760.0), "4.31176e+06"); // 7 digits → exponential, like %g
    }

    #[test]
    fn afterpoint_int_vs_float() {
        assert_eq!(afterpoint("18"), -1);
        assert_eq!(afterpoint("91.7"), 1);
        assert_eq!(afterpoint("103.49"), 2);
    }

    #[test]
    fn decimal_alignment_pads_ints() {
        // "18" must trail-pad to line its (absent) decimals up under "91.7".
        let rows = vec![vec!["91.7".to_string()], vec!["18".to_string()]];
        let md = table(&["avg_score"], &[true], &rows);
        // header colwidth = len("avg_score")+2 = 11; "18  " right-justified within.
        assert!(md.contains("|        18   |"), "got:\n{md}");
        assert!(md.contains("|        91.7 |"), "got:\n{md}");
    }
}
