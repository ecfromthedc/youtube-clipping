//! Analytics dashboard — port of `analyticsPage` + `sparkline` in rust/web/app.js.
//!
//! Closes the render → publish → MEASURE → tune loop: channel rollup tiles,
//! 7-day sparkline, "what's working" recommendations, and the top-videos
//! table. Same class names, element structure, visible text, and API calls
//! as the vanilla-JS original — styles.css is the parity contract.
//!
//! Data: four parallel GETs (mirrors the Promise.all + `.catch(() => null)`):
//!   /api/analytics/rollup?days=28 · /api/analytics/top?days=28&limit=15
//!   /api/analytics/daily?days=7   · /api/analytics/recommendations

use crate::http::get_json;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

// ── Response shapes (only the fields the page reads; see analytics.rs) ──

#[derive(Clone, Default, Deserialize)]
struct Rollup {
    #[serde(default)]
    configured: bool,
    #[serde(default)]
    views: f64,
    #[serde(default)]
    est_revenue: f64,
    #[serde(default)]
    subs_gained: f64,
    #[serde(default)]
    avg_watch_pct: f64,
}

#[derive(Clone, Default, Deserialize)]
struct TopVideos {
    #[serde(default)]
    videos: Vec<TopVideo>,
}

#[derive(Clone, Default, Deserialize)]
struct TopVideo {
    #[serde(default)]
    health: String,
    #[serde(default)]
    views: f64,
    #[serde(default, rename = "averagePercentageWatched")]
    average_percentage_watched: f64,
    #[serde(default, rename = "subscribersGained")]
    subscribers_gained: f64,
    #[serde(default, rename = "estimatedRevenue")]
    estimated_revenue: f64,
    #[serde(default)]
    video: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Clone, Default, Deserialize)]
struct Daily {
    #[serde(default)]
    views: Vec<f64>,
    #[serde(default)]
    revenue: Vec<f64>,
}

#[derive(Clone, Default, Deserialize)]
struct Recs {
    #[serde(default)]
    ready: bool,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    recommendations: Vec<String>,
    #[serde(default)]
    format_breakdown: Vec<FormatRow>,
}

#[derive(Clone, Default, Deserialize)]
struct FormatRow {
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    count: i64,
    #[serde(default)]
    total_views: f64,
    #[serde(default)]
    revenue: f64,
    #[serde(default)]
    avg_views: f64,
}

// ── Format helpers (port of app.js fmt.int / fmt.money) ──

/// Comma-groups the integer part of a non-negative-magnitude i64.
fn group_thousands(v: i64) -> String {
    let s = v.abs().to_string();
    let len = s.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if v < 0 {
        format!("-{out}")
    } else {
        out
    }
}

/// app.js `fmt.int` — `Number(n || 0).toLocaleString()`: thousands-grouped,
/// up to 3 fraction digits (only shows fractions for non-integer inputs,
/// e.g. avg views/post).
fn fmt_int(n: f64) -> String {
    let n = if n.is_finite() { n } else { 0.0 };
    let s = format!("{:.3}", n.abs());
    let (int_s, frac_s) = s.split_once('.').unwrap_or((s.as_str(), ""));
    let int_v: i64 = int_s.parse().unwrap_or(0);
    let mut out = group_thousands(int_v);
    let frac = frac_s.trim_end_matches('0');
    if !frac.is_empty() {
        out.push('.');
        out.push_str(frac);
    }
    if n < 0.0 && out != "0" {
        out.insert(0, '-');
    }
    out
}

/// app.js `fmt.money` — "$0" for falsy, 2 decimals under $100, else rounded
/// and comma-grouped.
fn fmt_money(n: f64) -> String {
    if n == 0.0 || !n.is_finite() {
        return "$0".to_string();
    }
    if n < 100.0 {
        return format!("${n:.2}");
    }
    format!("${}", group_thousands(n.round() as i64))
}

// ── Sparkline = SVG polyline scaled to fit (port of app.js sparkline) ──
// The original takes (views, revenue, dates) but only reads `views`.

fn sparkline(views: &[f64]) -> AnyView {
    const W: f64 = 600.0;
    const H: f64 = 80.0;
    const PAD: f64 = 4.0;
    if views.len() < 2 {
        return view! { <div class="muted">"Not enough data points."</div> }.into_any();
    }
    let max = views.iter().copied().fold(1.0_f64, f64::max);
    let step_x = (W - 2.0 * PAD) / (views.len() - 1) as f64;
    let pts = views
        .iter()
        .enumerate()
        .map(|(i, v)| {
            format!(
                "{},{}",
                PAD + i as f64 * step_x,
                H - PAD - (v / max) * (H - 2.0 * PAD)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let base_y = H - PAD;
    let right_x = W - PAD;
    let svg = format!(
        r##"<svg viewBox="0 0 {W} {H}" class="an-spark">
    <defs><linearGradient id="sg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#e100c3" stop-opacity="0.4"/>
      <stop offset="100%" stop-color="#e100c3" stop-opacity="0"/>
    </linearGradient></defs>
    <polyline points="{pts}" fill="none" stroke="#e100c3" stroke-width="2"/>
    <polygon points="{PAD},{base_y} {pts} {right_x},{base_y}" fill="url(#sg)"/>
  </svg>"##
    );
    view! { <div inner_html=svg></div> }.into_any()
}

// ── Sub-views (one per appended block in analyticsPage) ──

/// OAuth-not-connected warning (also shown when the rollup fetch fails,
/// mirroring `!rollup || !rollup.configured`).
fn oauth_warning() -> AnyView {
    view! {
        <div class="alert alert-warn" style="display:block">
            "⚠ No YouTube channel connected yet. Click "
            <a href="/api/oauth/yt/start">"＋ Connect a channel"</a>
            " (top right) and approve with the channel's Google login — analytics "
            "(views, retention, revenue) flow in from there. Multiple channels? "
            "Connect each one and toggle between them right here."
        </div>
    }
    .into_any()
}

/// Rollup tiles row.
fn tiles_view(r: &Rollup) -> AnyView {
    let tiles = [
        ("Views (28d)", fmt_int(r.views), false),
        ("Est. Revenue", fmt_money(r.est_revenue), true),
        ("Subs Gained", fmt_int(r.subs_gained), false),
        ("Avg Watch %", format!("{:.1}%", r.avg_watch_pct), false),
    ];
    view! {
        <div class="an-tiles">
            {tiles
                .into_iter()
                .map(|(label, value, accent)| {
                    let class = if accent { "an-tile an-tile-accent" } else { "an-tile" };
                    view! {
                        <div class=class>
                            <div class="an-tile-value">{value}</div>
                            <div class="an-tile-label">{label}</div>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
    .into_any()
}

/// Daily sparkline panel — only rendered when there are data points.
fn daily_view(d: Option<Daily>) -> Option<AnyView> {
    let d = d?;
    if d.views.is_empty() {
        return None;
    }
    let views_sum: f64 = d.views.iter().sum();
    let rev_sum: f64 = d.revenue.iter().sum();
    let summary = format!(
        "{} views · {} est. rev",
        fmt_int(views_sum),
        fmt_money(rev_sum)
    );
    Some(
        view! {
            <div class="panel mt-16">
                <div class="row-between mb-8">
                    <strong>"Last 7 days"</strong>
                    <span class="muted mono" style="font-size:11px;">{summary}</span>
                </div>
                {sparkline(&d.views)}
            </div>
        }
        .into_any(),
    )
}

/// "What's working" recommendations panel (ready + not-ready states).
fn recs_view(recs: Option<Recs>) -> Option<AnyView> {
    let rc = recs?;
    if !rc.ready {
        let note = rc
            .note
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Not enough posted videos yet.".to_string());
        return Some(
            view! {
                <div class="panel mt-16">
                    <strong>"🎯 What's working"</strong>
                    <p class="muted mt-8">{note}</p>
                </div>
            }
            .into_any(),
        );
    }
    let items = rc
        .recommendations
        .into_iter()
        .map(|r| view! { <div class="alert alert-info mb-8">{format!("→ {r}")}</div> })
        .collect_view();
    let breakdown: Option<AnyView> = if rc.format_breakdown.is_empty() {
        None
    } else {
        let rows = rc
            .format_breakdown
            .into_iter()
            .map(|f| {
                let format_label = f
                    .format
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "—".to_string());
                view! {
                    <tr>
                        <td>{format_label}</td>
                        <td class="mono">{f.count.to_string()}</td>
                        <td class="mono">{fmt_int(f.total_views)}</td>
                        <td class="mono">{fmt_money(f.revenue)}</td>
                        <td class="mono">{fmt_int(f.avg_views)}</td>
                    </tr>
                }
            })
            .collect_view();
        Some(
            view! {
                <div class="mt-16">
                    <strong style="font-size:13px;">"Format breakdown"</strong>
                    <table class="an-table mt-8">
                        <thead>
                            <tr>
                                <th>"Format"</th>
                                <th>"Posts"</th>
                                <th>"Total views"</th>
                                <th>"Revenue"</th>
                                <th>"Avg views/post"</th>
                            </tr>
                        </thead>
                        <tbody>{rows}</tbody>
                    </table>
                </div>
            }
            .into_any(),
        )
    };
    Some(
        view! {
            <div class="panel mt-16">
                <h3 style="margin: 0 0 12px;">"🎯 What's working"</h3>
                {items}
                {breakdown}
            </div>
        }
        .into_any(),
    )
}

/// Top videos table (or the honest empty state).
fn top_view(top: Option<TopVideos>) -> AnyView {
    let videos = top.map(|t| t.videos).unwrap_or_default();
    if videos.is_empty() {
        return view! {
            <div class="panel mt-16">
                <strong>"Top videos"</strong>
                <p class="muted mt-8">"No videos with view data in the last 28 days."</p>
            </div>
        }
        .into_any();
    }
    let n = videos.len();
    let rows = videos
        .into_iter()
        .map(|v| {
            let health_class = format!("health-dot health-{}", v.health);
            let video_cell = if let Some(u) = v.url.clone().filter(|u| !u.is_empty()) {
                let label = v
                    .video
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "watch ↗".to_string());
                view! { <a href=u target="_blank">{label}</a> }.into_any()
            } else {
                let label = v
                    .video
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "—".to_string());
                view! { <span class="mono muted">{label}</span> }.into_any()
            };
            view! {
                <tr>
                    <td><span class=health_class></span></td>
                    <td class="mono">{fmt_int(v.views)}</td>
                    <td class="mono">{format!("{:.1}%", v.average_percentage_watched)}</td>
                    <td class="mono">{fmt_int(v.subscribers_gained)}</td>
                    <td class="mono">{fmt_money(v.estimated_revenue)}</td>
                    <td>{video_cell}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <div class="panel mt-16">
            <h3 style="margin: 0 0 12px;">{format!("Top {n} videos (28d)")}</h3>
            <table class="an-table">
                <thead>
                    <tr>
                        // health dot
                        <th></th>
                        <th>"Views"</th>
                        <th>"Avg watch %"</th>
                        <th>"Subs"</th>
                        <th>"Est. rev"</th>
                        <th>"Video"</th>
                    </tr>
                </thead>
                <tbody>{rows}</tbody>
            </table>
        </div>
    }
    .into_any()
}

// ── Page component ──

/// Connected channels for the toggle — token-free (/api/channels).
#[derive(Clone, Deserialize)]
struct ChannelsResp {
    #[serde(default)]
    channels: Vec<ChannelMeta>,
}
#[derive(Clone, Deserialize, PartialEq)]
struct ChannelMeta {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
}

#[component]
pub fn Analytics() -> impl IntoView {
    // One signal per request; `None` = failed fetch (JS `.catch(() => null)`).
    let rollup = RwSignal::new(None::<Rollup>);
    let top = RwSignal::new(None::<TopVideos>);
    let daily = RwSignal::new(None::<Daily>);
    let recs = RwSignal::new(None::<Recs>);
    // Countdown mirrors Promise.all: body renders only once all four settle.
    let pending = RwSignal::new(4_i32);
    // Multi-channel toggle: connected channels + the one being viewed.
    // None = server default (first connected channel, .env fallback).
    let channels = RwSignal::new(Vec::<ChannelMeta>::new());
    let selected = RwSignal::new(None::<String>);

    // Fire the four dashboard requests in parallel for one channel
    // (try_* guards a late arrival after nav-away).
    let load = move |chan: Option<String>| {
        pending.set(4);
        let q = chan.map(|c| format!("&channel={c}")).unwrap_or_default();
        let q1 = q.clone();
        spawn_local(async move {
            let v = get_json::<Rollup>(&format!("/api/analytics/rollup?days=28{q1}"))
                .await
                .ok();
            let _ = rollup.try_set(v);
            let _ = pending.try_update(|p| *p -= 1);
        });
        let q2 = q.clone();
        spawn_local(async move {
            let v = get_json::<TopVideos>(&format!("/api/analytics/top?days=28&limit=15{q2}"))
                .await
                .ok();
            let _ = top.try_set(v);
            let _ = pending.try_update(|p| *p -= 1);
        });
        spawn_local(async move {
            let v = get_json::<Daily>(&format!("/api/analytics/daily?days=7{q}"))
                .await
                .ok();
            let _ = daily.try_set(v);
            let _ = pending.try_update(|p| *p -= 1);
        });
        spawn_local(async move {
            // Recommendations read the clips DB — channel-agnostic.
            let v = get_json::<Recs>("/api/analytics/recommendations")
                .await
                .ok();
            let _ = recs.try_set(v);
            let _ = pending.try_update(|p| *p -= 1);
        });
    };
    load(None);
    spawn_local(async move {
        if let Ok(r) = get_json::<ChannelsResp>("/api/channels").await {
            let _ = channels.try_set(r.channels);
        }
    });

    // app.js: fetch("/api/analytics/rollup").then(() => location.reload())
    // (reload only when the request resolves — the endpoint always answers 200,
    // so Ok-gating matches the JS fetch-resolved semantics exactly).
    let refresh = move |_| {
        spawn_local(async move {
            if get_json::<serde_json::Value>("/api/analytics/rollup")
                .await
                .is_ok()
            {
                if let Some(w) = web_sys::window() {
                    let _ = w.location().reload();
                }
            }
        });
    };

    view! {
        <div class="page-header">
            <div>
                <h1 class="page-title">"Analytics"</h1>
                <p class="page-sub">
                    "Channel rollup, top posts, and the 'what's working' recommendations derived from your own data."
                </p>
            </div>
            <div class="row">
                <a class="btn btn-sm" href="/api/oauth/yt/start">"＋ Connect a channel"</a>
                <button class="btn btn-ghost btn-sm" on:click=refresh>"↻ Refresh (1h cache)"</button>
            </div>
        </div>
        {move || {
            let list = channels.get();
            if list.len() < 2 {
                return ().into_any();
            }
            // Channel toggle: tap a chip → that channel's numbers everywhere.
            let active_id = selected.get().unwrap_or_else(|| list[0].id.clone());
            view! {
                <div class="row mb-24" style="flex-wrap: wrap; gap: 8px;">
                    {list
                        .into_iter()
                        .map(|c| {
                            let id = c.id.clone();
                            let is_active = c.id == active_id;
                            view! {
                                <button
                                    class=if is_active { "btn btn-sm" } else { "btn btn-ghost btn-sm" }
                                    on:click=move |_| {
                                        selected.set(Some(id.clone()));
                                        load(Some(id.clone()));
                                    }
                                >
                                    {format!("📺 {}", c.title)}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            }
                .into_any()
        }}
        {move || {
            if pending.get() > 0 {
                return view! {
                    <div class="row mb-24">
                        <div class="spinner"></div>
                        "Loading analytics…"
                    </div>
                }
                .into_any();
            }
            match rollup.get() {
                Some(r) if r.configured => {
                    view! {
                        {tiles_view(&r)}
                        {daily_view(daily.get())}
                        {recs_view(recs.get())}
                        {top_view(top.get())}
                    }
                    .into_any()
                }
                _ => oauth_warning(),
            }
        }}
    }
}
