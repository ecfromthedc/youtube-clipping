//! Dashboard — port of `dashboardPage` in rust/web/app.js (lines ~127–170).
//!
//! Route: `#/`
//! Data: `GET /api/projects` → `{ projects: [{ id, filename, duration,
//!       candidates, renders }] }` (see `list_projects` in rust/src/server.rs).
//! Class-for-class port: page-header → proj-grid → proj-card list, with the
//! same loading spinner, empty state, and router-style error alert.

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::http::get_json;

/// Response envelope for `GET /api/projects`.
#[derive(Deserialize)]
struct ProjectsResponse {
    #[serde(default)]
    projects: Vec<ProjectSummary>,
}

/// One row of the projects list (only the fields the dashboard renders).
#[derive(Deserialize, Clone)]
struct ProjectSummary {
    #[serde(default)]
    id: String,
    #[serde(default)]
    filename: String,
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    candidates: u64,
    #[serde(default)]
    renders: u64,
}

/// Fetch state for the grid.
#[derive(Clone)]
enum Load {
    Loading,
    Failed(String),
    Ready(Vec<ProjectSummary>),
}

/// Port of app.js `fmt.duration`: falsy → "—", <60 → "Ns", else "Mm Ss".
fn fmt_duration(secs: f64) -> String {
    if secs == 0.0 || secs.is_nan() {
        return "—".into();
    }
    if secs < 60.0 {
        return format!("{}s", secs.round() as i64);
    }
    let m = (secs / 60.0).floor() as i64;
    let s = (secs % 60.0).round() as i64;
    format!("{m}m {s}s")
}

#[component]
pub fn Dashboard() -> impl IntoView {
    let state = RwSignal::new(Load::Loading);
    // One-shot fetch on mount (mirrors the single `await api.get` in app.js).
    spawn_local(async move {
        match get_json::<ProjectsResponse>("/api/projects").await {
            Ok(r) => state.set(Load::Ready(r.projects)),
            Err(e) => state.set(Load::Failed(e)),
        }
    });

    view! {
        <div class="page-header">
            <div>
                <h1 class="page-title">"Projects"</h1>
                <p class="page-sub">
                    "Drop in raw footage. The pipeline transcribes it, ranks your best moments, and renders a captioned 9:16 clip ready to post."
                </p>
            </div>
            <a class="btn btn-primary" href="#/new">"+ New project"</a>
        </div>
        <div class="proj-grid">
            {move || match state.get() {
                // Loading — and on error too, matching the vanilla page: the
                // thrown fetch leaves the spinner in the grid and the router
                // appends the alert after it.
                Load::Loading | Load::Failed(_) => view! {
                    <div class="empty">
                        <div class="spinner"></div>
                        "Loading…"
                    </div>
                }
                .into_any(),
                Load::Ready(projects) if projects.is_empty() => view! {
                    <div class="panel-soft" style="grid-column: 1/-1; padding: 48px;">
                        <div class="empty">
                            <div class="empty-icon">"⛵"</div>
                            <div>"No projects yet."</div>
                            <div class="mt-8">
                                <a class="btn btn-primary btn-sm" href="#/new">"Start your first"</a>
                            </div>
                        </div>
                    </div>
                }
                .into_any(),
                Load::Ready(projects) => projects
                    .into_iter()
                    .map(|p| {
                        let ProjectSummary { id, filename, duration, candidates, renders } = p;
                        let name = if filename.is_empty() {
                            "untitled".to_string()
                        } else {
                            filename
                        };
                        view! {
                            <a class="proj-card" href=format!("#/p/{id}")>
                                <div class="proj-thumb">"🎬"</div>
                                <div class="proj-name">{name}</div>
                                <div class="proj-meta">
                                    <span>{format!("⏱ {}", fmt_duration(duration))}</span>
                                    <span>{format!("✂ {candidates} cands")}</span>
                                    <span>{format!("📦 {renders} rendered")}</span>
                                </div>
                            </a>
                        }
                    })
                    .collect_view()
                    .into_any(),
            }}
        </div>
        // app.js router catch: `⚠ ${err.message}` appended after the grid.
        {move || match state.get() {
            Load::Failed(msg) => Some(view! {
                <div class="alert alert-error">{format!("⚠ {msg}")}</div>
            }),
            _ => None,
        }}
    }
}
