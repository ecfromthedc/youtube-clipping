//! The gamified "Race to $15K" scoreboard. Mirrors src/ycp/scoreboard.py: reduces the
//! clips+metrics rows to a single game state (level, run-rate, milestone ladder, quests,
//! channel leaderboard) and renders SCOREBOARD.md. Fully deterministic, no LLM.
use std::collections::{BTreeMap, HashSet};

use crate::db::ClipRow;
use crate::util::{money, views};

const GOAL: f64 = 15_000.0;
const HIT_VIEWS: i64 = 100_000;

/// (monthly run-rate threshold, name, badge, one-line meaning) — the level ladder.
const LEVELS: &[(f64, &str, &str, &str)] = &[
    (0.0, "Boot Up", "🟢", "system built — channels not live yet"),
    (
        1.0,
        "First Blood",
        "🩸",
        "first dollar earned / first channel live",
    ),
    (500.0, "Signal", "📡", "$500/mo — a format is working"),
    (
        2_500.0,
        "Traction",
        "🚀",
        "$2.5K/mo — the loop is compounding",
    ),
    (6_000.0, "Engine", "⚙️", "$6K/mo — scale the proven winners"),
    (10_000.0, "Cruise", "🛞", "$10K/mo — most of the way home"),
    (GOAL, "GOAL", "🏁", "$15K/mo — WIN. Then push Overdrive."),
];

/// concept (quest) -> the source creators that feed it, to detect when it's live.
const QUESTS: &[(&str, &[&str])] = &[
    (
        "1 · Hot Seat (debate/agitation)",
        &[
            "Jubilee",
            "Whatever (clips)",
            "No Jumper",
            "Pop Culture Crisis",
            "Modern Day Debate",
            "Flagrant",
        ],
    ),
    (
        "2 · Money Fights (finance conflict)",
        &[
            "Ramit Sethi",
            "Graham Stephan",
            "Codie Sanchez",
            "My First Million",
            "Alex Hormozi",
            "Gary Vaynerchuk",
        ],
    ),
    (
        "3 · Crash Out (comedy/reaction)",
        &[
            "Bad Friends",
            "Kill Tony",
            "This Is Important",
            "ModernWisdom",
        ],
    ),
];

/// Highest level reached → (1-based level number, current level, next level or None).
fn level_of(
    run_rate: f64,
) -> (
    usize,
    &'static (f64, &'static str, &'static str, &'static str),
    Option<&'static (f64, &'static str, &'static str, &'static str)>,
) {
    let mut idx = 0;
    for (i, lvl) in LEVELS.iter().enumerate() {
        if run_rate >= lvl.0 {
            idx = i;
        }
    }
    (idx + 1, &LEVELS[idx], LEVELS.get(idx + 1))
}

/// Python `round()` — banker's rounding (half to even) — for the progress-bar fill.
fn round_half_even(x: f64) -> i64 {
    let r = x.round();
    if (x - x.floor() - 0.5).abs() < f64::EPSILON {
        let f = x.floor() as i64;
        if f % 2 == 0 {
            f
        } else {
            f + 1
        }
    } else {
        r as i64
    }
}

fn bar(pct: f64) -> String {
    let pct = pct.clamp(0.0, 1.0);
    let width = 24i64;
    let fill = round_half_even(pct * width as f64).clamp(0, width);
    "▰".repeat(fill as usize) + &"▱".repeat((width - fill) as usize)
}

struct Best {
    channel: String,
    views: i64,
    creator: String,
}

struct Game {
    run_rate: f64,
    views: i64,
    clips: usize,
    posted: usize,
    channels: usize,
    hits: usize,
    hit_rate: f64,
    best: Option<Best>,
    leaderboard: Vec<(String, i64, f64, i64)>, // channel, views, revenue, clips
    quests: Vec<(String, String)>,             // name, status line
}

fn compute(rows: &[ClipRow]) -> Game {
    if rows.is_empty() {
        return Game {
            run_rate: 0.0,
            views: 0,
            clips: 0,
            posted: 0,
            channels: 0,
            hits: 0,
            hit_rate: 0.0,
            best: None,
            leaderboard: vec![],
            quests: QUESTS
                .iter()
                .map(|(name, _)| {
                    (
                        (*name).to_string(),
                        "⬜ queued — spin up a channel".to_string(),
                    )
                })
                .collect(),
        };
    }

    let run_rate: f64 = rows.iter().map(|r| r.ad_revenue).sum();
    let total_views: i64 = rows.iter().map(|r| r.views).sum();
    let posted = rows
        .iter()
        .filter(|r| r.status.as_deref() == Some("posted"))
        .count();
    let hits = rows.iter().filter(|r| r.views >= HIT_VIEWS).count();
    let n = rows.len();

    let channels: HashSet<&str> = rows.iter().filter_map(|r| r.channel.as_deref()).collect();

    // Best clip = first row holding the max views (mirrors pandas idxmax, first occurrence).
    let mut best_i = 0usize;
    for (i, r) in rows.iter().enumerate() {
        if r.views > rows[best_i].views {
            best_i = i;
        }
    }
    let b = &rows[best_i];
    let best = Some(Best {
        channel: b.channel.clone().unwrap_or_else(|| "?".to_string()),
        views: b.views,
        creator: b.source_creator.clone().unwrap_or_else(|| "?".to_string()),
    });

    // Channel leaderboard: group (alphabetical), stable-sort by revenue desc, top 5.
    let mut by_chan: BTreeMap<String, (i64, f64, i64)> = BTreeMap::new();
    for r in rows {
        let ch = r.channel.clone().unwrap_or_default();
        let e = by_chan.entry(ch).or_insert((0, 0.0, 0));
        e.0 += r.views;
        e.1 += r.ad_revenue;
        e.2 += 1;
    }
    let mut leaderboard: Vec<(String, i64, f64, i64)> = by_chan
        .into_iter()
        .map(|(ch, (v, rev, c))| (ch, v, rev, c))
        .collect();
    leaderboard.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    leaderboard.truncate(5);

    // Quests: a concept is live if any of its feeder creators appears in the data.
    let creators: HashSet<&str> = rows
        .iter()
        .filter_map(|r| r.source_creator.as_deref())
        .collect();
    let quests = QUESTS
        .iter()
        .map(|(name, feeders)| {
            let live: HashSet<&str> = feeders
                .iter()
                .copied()
                .filter(|c| creators.contains(c))
                .collect();
            let status = if live.is_empty() {
                "⬜ queued — spin up a channel".to_string()
            } else {
                let sub: Vec<&ClipRow> = rows
                    .iter()
                    .filter(|r| {
                        r.source_creator
                            .as_deref()
                            .map(|c| live.contains(c))
                            .unwrap_or(false)
                    })
                    .collect();
                let qv: i64 = sub.iter().map(|r| r.views).sum();
                let qr: f64 = sub.iter().map(|r| r.ad_revenue).sum();
                format!(
                    "🟢 LIVE — {} clips · {} views · {}",
                    sub.len(),
                    views(qv as f64),
                    money(qr)
                )
            };
            ((*name).to_string(), status)
        })
        .collect();

    Game {
        run_rate,
        views: total_views,
        clips: n,
        posted,
        channels: channels.len(),
        hits,
        hit_rate: if n > 0 { hits as f64 / n as f64 } else { 0.0 },
        best,
        leaderboard,
        quests,
    }
}

/// Render SCOREBOARD.md from the game state. Deterministic.
pub fn build(rows: &[ClipRow], as_of: &str) -> String {
    let s = compute(rows);
    let pct = s.run_rate / GOAL;
    let (level_n, lvl, nxt) = level_of(s.run_rate);

    let next_line = match nxt {
        Some(n) => {
            let gap = (n.0 - s.run_rate).max(0.0);
            format!(
                "**Next:** {} {} at {}/mo — {} to go.",
                n.2,
                n.1,
                money(n.0),
                money(gap)
            )
        }
        None => "**🏁 GOAL CLEARED. You win. Push to Overdrive.**".to_string(),
    };

    let ladder = LEVELS
        .iter()
        .map(|(thresh, name, badge, meaning)| {
            let lit = if s.run_rate >= *thresh { "✅" } else { "⬜" };
            let here = if *name == lvl.1 { " ◀ **YOU**" } else { "" };
            format!(
                "| {lit} | {badge} {name} | {}/mo | {meaning}{here} |",
                money(*thresh)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let leaderboard = if s.leaderboard.is_empty() {
        "_(no posted clips yet — leaderboard fills as channels go live)_".to_string()
    } else {
        let mut lines = vec![
            "| # | channel | views | revenue | clips |".to_string(),
            "|---|---|---|---|---|".to_string(),
        ];
        for (i, (ch, v, rev, nclips)) in s.leaderboard.iter().enumerate() {
            lines.push(format!(
                "| {} | {ch} | {} | {} | {nclips} |",
                i + 1,
                views(*v as f64),
                money(*rev)
            ));
        }
        lines.join("\n")
    };

    let quests = s
        .quests
        .iter()
        .map(|(name, status)| format!("- **{name}** — {status}"))
        .collect::<Vec<_>>()
        .join("\n");

    let best = match &s.best {
        Some(b) => format!(
            "{} views — {} (from {})",
            views(b.views as f64),
            b.channel,
            b.creator
        ),
        None => "—".to_string(),
    };

    format!(
        "# 🏁 Race to $15K — Scoreboard\n\n\
         _Auto-generated by `ycp scoreboard` from the closed-loop DB · as of {as_of}._\n\n\
         ## {} Level {level_n} — {}\n\n\
         ### {}/mo  ·  {}  {:.1}% to $15K\n\n\
         {next_line}\n\n\
         | stat | value |\n|---|---|\n\
         | 💵 Monthly run-rate | **{}** (ad revenue, owned channels) |\n\
         | 👀 Views captured | {} |\n\
         | 🎬 Clips (posted / total) | {} / {} |\n\
         | 📺 Channels live | {} |\n\
         | 🎯 Hit-rate (clips ≥100K) | {:.0}%  ({} hits) |\n\
         | 🏆 Best clip | {best} |\n\n\
         ## 🪜 Milestone ladder\n| | level | gate | meaning |\n|---|---|---|---|\n\
         {ladder}\n\n\
         ## 🗺️ Quests (the 5 concepts — clear each go/no-go gate to scale it)\n\
         {quests}\n\n\
         ## 🏅 Channel leaderboard\n{leaderboard}\n\n\
         ---\n\
         > The loop keeps score: every `ycp capture` + `ycp brief` cycle updates these numbers.\n\
         > Climb the ladder by making more of what the Double-Down Brief says wins. 🏁\n",
        lvl.2,
        lvl.1,
        money(s.run_rate),
        bar(pct),
        pct * 100.0,
        money(s.run_rate),
        views(s.views as f64),
        s.posted,
        s.clips,
        s.channels,
        s.hit_rate * 100.0,
        s.hits,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_boundaries() {
        assert_eq!(level_of(0.0).0, 1); // Boot Up
        assert_eq!(level_of(1.0).0, 2); // First Blood at exactly $1
        assert_eq!(level_of(244.0).0, 2);
        assert_eq!(level_of(500.0).0, 3); // Signal
        assert_eq!(level_of(GOAL).0, 7); // GOAL — no next level
        assert!(level_of(GOAL).2.is_none());
    }

    #[test]
    fn empty_bar_is_all_unfilled() {
        assert_eq!(bar(0.0).chars().filter(|c| *c == '▱').count(), 24);
        assert_eq!(bar(0.016).chars().filter(|c| *c == '▰').count(), 0);
    }
}
