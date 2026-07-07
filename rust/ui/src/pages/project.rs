//! Project page (editor) — port of `projectPage`, `timelineView`, `sidebarActions`,
//! `candidateList`, `rendersList`, `_studioOutputList`, `compilesList`, and
//! `compileSection` (app.js ~264-755) plus `openPublishModal` and
//! `renderCardActions` (app.js ~1071-1249).
//!
//! Class-for-class port: same element structure, same visible text, same API
//! calls (styles.css is the parity contract). After a successful transcribe /
//! render / compile the page reloads on the same delay app.js uses, so the
//! project data stays a plain (non-reactive) snapshot exactly like the old UI.
//!
//! ⛔ SHARED POSTIZ ACCOUNT: the publish modal lists integrations from
//! /api/postiz/integrations and publishes via /api/postiz/publish — ported
//! exactly, no new Postiz behavior of any kind.

use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::http::{delete, get_json, post_json};

// ── API shapes (server.rs Project / Render / publish routes) ──────────────────

#[derive(Clone, Deserialize)]
struct Project {
    #[serde(default)]
    id: String,
    #[serde(default)]
    filename: String,
    #[serde(default)]
    duration: f64,
    /// Only the length is used (`hasTranscript` gate), so keep entries opaque.
    #[serde(default)]
    transcript: Vec<serde_json::Value>,
    #[serde(default)]
    candidates: Vec<Candidate>,
    #[serde(default)]
    renders: Vec<Render>,
    #[serde(default)]
    compiles: Vec<Render>,
    #[serde(default)]
    stories: Vec<Render>,
    #[serde(default)]
    commentary: Vec<Render>,
}

#[derive(Clone, Deserialize)]
struct Candidate {
    #[serde(default)]
    start: f64,
    #[serde(default)]
    end: f64,
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    score: f64,
    #[serde(default)]
    text: String,
}

#[derive(Clone, Deserialize)]
struct Render {
    #[serde(default)]
    path: String,
    #[serde(default)]
    title: String,
}

#[derive(Serialize)]
struct RenderReq {
    start: f64,
    end: f64,
    title: Option<String>,
}

#[derive(Clone, Deserialize)]
struct RenderResp {
    #[serde(default)]
    path: String,
}

#[derive(Serialize)]
struct CompileItemReq {
    start: f64,
    end: f64,
    label: String,
}

#[derive(Serialize)]
struct CompileReq {
    items: Vec<CompileItemReq>,
    title: Option<String>,
    order: String,
}

#[derive(Clone, Deserialize)]
struct CompileResp {
    #[serde(default)]
    path: String,
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    segments: i64,
}

#[derive(Clone, Deserialize)]
struct IntegrationsResp {
    #[serde(default)]
    available: bool,
    #[serde(default)]
    token_configured: bool,
    #[serde(default)]
    integrations: Vec<Integration>,
}

#[derive(Clone, Deserialize)]
struct Integration {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    profile: String,
    #[serde(default)]
    identifier: String,
    #[serde(default)]
    disabled: bool,
}

#[derive(Serialize)]
struct PublishReq {
    path: String,
    integration_id: String,
    title: String,
    caption: Option<String>,
    schedule: String,
}

#[derive(Clone, Deserialize)]
struct PublishResp {
    #[serde(default)]
    post_id: String,
}

// ── Local UI state ─────────────────────────────────────────────────────────────

/// What the publish modal is pointed at (mirrors openPublishModal's args:
/// the full /api/.../files/... URL + the suggested title).
#[derive(Clone)]
struct PublishTarget {
    path: String,
    title: String,
}

#[derive(Clone)]
struct Pick {
    start: f64,
    end: f64,
    label: String,
}

#[derive(Clone)]
enum TxState {
    Idle,
    Working,
    Done,
    Error(String),
}

#[derive(Clone)]
enum RenderStatus {
    Idle,
    Invalid,
    Working,
    Done(String),
    Error(String),
}

#[derive(Clone)]
enum CompileStatus {
    Idle,
    NeedTwo,
    Working(usize),
    Done {
        segments: i64,
        duration: f64,
        path: String,
    },
    Error(String),
}

#[derive(Clone)]
enum IntState {
    Loading,
    Failed,
    Loaded(IntegrationsResp),
}

#[derive(Clone)]
enum PubStatus {
    Idle,
    Warn,
    Working,
    Done(String),
    Error(String),
}

// ── fmt helpers (port of the `fmt` object in app.js) ──────────────────────────

/// fmt.time — "m:ss", "0:00" for non-finite input.
fn fmt_time(s: f64) -> String {
    if !s.is_finite() {
        return "0:00".to_string();
    }
    let m = (s / 60.0).floor() as i64;
    let sec = (s % 60.0).floor() as i64;
    format!("{m}:{sec:02}")
}

/// fmt.duration — "—" when falsy, "Ns" under a minute, "Mm Ss" above.
fn fmt_duration(secs: f64) -> String {
    if !secs.is_finite() || secs == 0.0 {
        return "—".to_string();
    }
    if secs < 60.0 {
        return format!("{}s", secs.round() as i64);
    }
    let m = (secs / 60.0).floor() as i64;
    let s = (secs % 60.0).round() as i64;
    format!("{m}m {s}s")
}

/// JS `.slice(0, n)` on a string, but safe on char boundaries.
fn take_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// `setTimeout(() => location.reload(), ms)` — the post-mutation refresh app.js uses.
fn reload_after(ms: i32) {
    let Some(w) = web_sys::window() else { return };
    let cb = Closure::once_into_js(|| {
        if let Some(w) = web_sys::window() {
            let _ = w.location().reload();
        }
    });
    let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), ms);
}

// ── Page ───────────────────────────────────────────────────────────────────────

#[component]
pub fn ProjectPage(id: String) -> impl IntoView {
    let state: RwSignal<Option<Result<Project, String>>> = RwSignal::new(None);
    {
        let id = id.clone();
        spawn_local(async move {
            state.set(Some(
                get_json::<Project>(&format!("/api/projects/{id}")).await,
            ));
        });
    }

    view! {
        {move || match state.get() {
            None => ().into_any(),
            Some(Err(e)) => {
                view! { <div class="alert alert-error">{format!("⚠ {e}")}</div> }.into_any()
            }
            Some(Ok(p)) => project_editor(p).into_any(),
        }}
    }
}

/// The full editor once the project is loaded (app.js projectPage `render()`).
fn project_editor(project: Project) -> impl IntoView {
    let pid = StoredValue::new(project.id.clone());
    let video_ref = NodeRef::<html::Video>::new();
    let playhead_pct = RwSignal::new(0.0f64);
    let publish_modal: RwSignal<Option<PublishTarget>> = RwSignal::new(None);
    // `project.duration || 1` — used by both the timeline blocks and playhead sync.
    let dur = if project.duration > 0.0 {
        project.duration
    } else {
        1.0
    };

    let title_text = if project.filename.is_empty() {
        "untitled".to_string()
    } else {
        project.filename.clone()
    };
    let has_candidates = !project.candidates.is_empty();

    view! {
        <div class="page-header">
            <div>
                <h1 class="page-title">{title_text}</h1>
                <p class="page-sub">
                    <span class="mono">{project.id.clone()}</span>
                    " · "
                    <span>{fmt_duration(project.duration)}</span>
                    " · "
                    <span>{format!("{} candidates", project.candidates.len())}</span>
                </p>
            </div>
            <div class="row">
                <a class="btn btn-ghost btn-sm" href="#/">"← All projects"</a>
                <button
                    class="btn btn-danger btn-sm"
                    on:click=move |_| {
                        let confirmed = web_sys::window()
                            .map(|w| {
                                w.confirm_with_message("Delete this project and all renders?")
                                    .unwrap_or(false)
                            })
                            .unwrap_or(false);
                        if !confirmed {
                            return;
                        }
                        let id = pid.get_value();
                        spawn_local(async move {
                            let _ = delete(&format!("/api/projects/{id}")).await;
                            if let Some(w) = web_sys::window() {
                                let _ = w.location().set_hash("/");
                            }
                        });
                    }
                >
                    "Delete"
                </button>
            </div>
        </div>
        <div class="editor">
            <div class="editor-main">
                <div class="player-wrap">
                    <video
                        class="player"
                        src=format!("/api/projects/{}/files/source.mp4", project.id)
                        controls=true
                        playsinline=true
                        node_ref=video_ref
                        on:timeupdate=move |_| {
                            if let Some(v) = video_ref.get_untracked() {
                                let pct = (v.current_time() / dur * 100.0).clamp(0.0, 100.0);
                                playhead_pct.set(pct);
                            }
                        }
                    ></video>
                </div>
                {if has_candidates {
                    timeline_view(&project, video_ref, playhead_pct).into_any()
                } else {
                    let msg = if project.duration > 0.0 {
                        "Click Transcribe to surface ranked clip moments."
                    } else {
                        "No source video yet — drop one in from the dashboard."
                    };
                    view! {
                        <div class="panel">
                            <div class="row-between mb-16">
                                <strong>"Auto-clip timeline"</strong>
                                <span class="muted">"Transcribe first"</span>
                            </div>
                            <div class="alert alert-info">{msg}</div>
                        </div>
                    }
                        .into_any()
                }}
            </div>
            <div class="editor-sidebar">
                {sidebar_actions(&project)}
                {(project.candidates.len() >= 2).then(|| compile_section(&project))}
                {has_candidates.then(|| candidate_list(&project, video_ref))}
                {(!project.renders.is_empty()).then(|| renders_list(&project, publish_modal))}
                {(!project.compiles.is_empty()).then(|| compiles_list(&project, publish_modal))}
                {(!project.stories.is_empty())
                    .then(|| {
                        studio_output_list(
                            &project.stories,
                            "Storytelling",
                            "📖",
                            "linear-gradient(135deg,#7c3aed,#5b21b6)",
                            &project.id,
                            publish_modal,
                        )
                    })}
                {(!project.commentary.is_empty())
                    .then(|| {
                        studio_output_list(
                            &project.commentary,
                            "Commentary",
                            "🎬",
                            "linear-gradient(135deg,#0891b2,#155e75)",
                            &project.id,
                            publish_modal,
                        )
                    })}
            </div>
        </div>
        {move || publish_modal.get().map(|t| publish_modal_view(t, publish_modal))}
    }
}

// ── Timeline component (app.js timelineView) ──────────────────────────────────

fn timeline_view(
    project: &Project,
    video_ref: NodeRef<html::Video>,
    playhead_pct: RwSignal<f64>,
) -> impl IntoView {
    let dur = if project.duration > 0.0 {
        project.duration
    } else {
        1.0
    };
    let ticks = 10;
    let cands = project.candidates.clone();

    view! {
        <div class="panel">
            <div class="row-between mb-8">
                <strong>"Auto-clip timeline"</strong>
                <span class="muted">"click a block to scrub"</span>
            </div>
            <div class="timeline">
                <div class="timeline-ruler">
                    {(0..ticks)
                        .map(|i| {
                            view! { <span>{fmt_time(dur * i as f64 / ticks as f64)}</span> }
                        })
                        .collect_view()}
                </div>
                <div class="timeline-track">
                    {cands
                        .into_iter()
                        .map(|c| {
                            let left = c.start / dur * 100.0;
                            let width = (c.end - c.start) / dur * 100.0;
                            let start = c.start;
                            let short = take_chars(&c.text, 40);
                            view! {
                                <div
                                    class="timeline-cand"
                                    style=format!("left: {left}%; width: {width}%")
                                    title=c.text.clone()
                                    on:click=move |_| {
                                        if let Some(v) = video_ref.get_untracked() {
                                            v.set_current_time(start);
                                            let _ = v.play();
                                        }
                                    }
                                >
                                    <span class="score">{format!("{:.2}", c.score)}</span>
                                    {short}
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
                <div
                    class="timeline-playhead"
                    style=move || format!("left: {}%;", playhead_pct.get())
                ></div>
            </div>
        </div>
    }
}

// ── Sidebar: actions (transcribe, manual clip) — app.js sidebarActions ────────

fn sidebar_actions(project: &Project) -> AnyView {
    let has_video = project.duration > 0.0;
    let has_transcript = !project.transcript.is_empty();

    if !has_video {
        return view! {
            <div class="sidebar-section">
                <h3>"Actions"</h3>
                <div class="alert alert-warn">"Upload a source video first."</div>
            </div>
        }
        .into_any();
    }

    if !has_transcript {
        return transcribe_section(project.id.clone()).into_any();
    }

    manual_clip_form(project).into_any()
}

/// The one-click "Transcribe & find clips" button state machine.
fn transcribe_section(project_id: String) -> impl IntoView {
    let pid = StoredValue::new(project_id);
    let tx = RwSignal::new(TxState::Idle);

    view! {
        <div class="sidebar-section">
            <h3>"Actions"</h3>
            <button
                class="btn btn-primary"
                style="width:100%"
                disabled=move || matches!(tx.get(), TxState::Working | TxState::Done)
                on:click=move |_| {
                    if matches!(tx.get_untracked(), TxState::Working | TxState::Done) {
                        return;
                    }
                    tx.set(TxState::Working);
                    let pid = pid.get_value();
                    spawn_local(async move {
                        // app.js posts `{}` — the route's Json extractor needs a body.
                        let body = serde_json::json!({});
                        match post_json::<
                            serde_json::Value,
                            serde_json::Value,
                        >(&format!("/api/projects/{pid}/transcribe"), &body)
                            .await
                        {
                            Ok(_) => {
                                tx.set(TxState::Done);
                                reload_after(500);
                            }
                            Err(e) => tx.set(TxState::Error(e)),
                        }
                    });
                }
            >
                {move || match tx.get() {
                    TxState::Done => view! { "✓ Done — refresh view" }.into_any(),
                    TxState::Working => {
                        view! {
                            <span class="spinner"></span>
                            " Working…"
                        }
                            .into_any()
                    }
                    TxState::Error(e) => {
                        view! {
                            <span class="spinner hidden"></span>
                            {format!(" ⚠ {e}")}
                        }
                            .into_any()
                    }
                    TxState::Idle => {
                        view! {
                            <span class="spinner hidden"></span>
                            "Transcribe & find clips"
                        }
                            .into_any()
                    }
                }}
            </button>
            <p class="muted mt-8" style="font-size:12px">
                "Runs whisper.cpp over the audio. Takes ~10% of the video's length."
            </p>
        </div>
    }
}

/// Transcribed → manual clip form (start / end / hook title / render).
fn manual_clip_form(project: &Project) -> impl IntoView {
    let pid = StoredValue::new(project.id.clone());
    let start_ref = NodeRef::<html::Input>::new();
    let end_ref = NodeRef::<html::Input>::new();
    let title_ref = NodeRef::<html::Input>::new();

    // Pre-fill with the top candidate.
    let (start_pre, end_pre, title_pre) = match project.candidates.first() {
        Some(c) => (
            Some(format!("{:.1}", c.start)),
            Some(format!("{:.1}", c.end)),
            Some(take_chars(&c.text, 60)),
        ),
        None => (None, None, None),
    };
    let end_placeholder = format!("{:.1}", project.duration);

    let rstatus = RwSignal::new(RenderStatus::Idle);
    let rendering = RwSignal::new(false);

    view! {
        <div class="sidebar-section">
            <h3>"Actions"</h3>
            <div class="field-row">
                <div class="field">
                    <label>"Start (s)"</label>
                    <input
                        class="input"
                        type="number"
                        step="0.1"
                        placeholder="0.0"
                        value=start_pre
                        node_ref=start_ref
                    />
                </div>
                <div class="field">
                    <label>"End (s)"</label>
                    <input
                        class="input"
                        type="number"
                        step="0.1"
                        placeholder=end_placeholder
                        value=end_pre
                        node_ref=end_ref
                    />
                </div>
            </div>
            <div class="field">
                <label>"Hook title"</label>
                <input
                    class="input"
                    placeholder="Hook title (optional)"
                    value=title_pre
                    node_ref=title_ref
                />
            </div>
            <button
                class="btn btn-primary"
                style="width:100%"
                disabled=move || rendering.get()
                on:click=move |_| {
                    let start_v = start_ref.get_untracked().map(|i| i.value()).unwrap_or_default();
                    let end_v = end_ref.get_untracked().map(|i| i.value()).unwrap_or_default();
                    let (Ok(start), Ok(end)) = (
                        start_v.trim().parse::<f64>(),
                        end_v.trim().parse::<f64>(),
                    ) else {
                        rstatus.set(RenderStatus::Invalid);
                        return;
                    };
                    if !start.is_finite() || !end.is_finite() || end <= start {
                        rstatus.set(RenderStatus::Invalid);
                        return;
                    }
                    if rendering.get_untracked() {
                        return;
                    }
                    rendering.set(true);
                    rstatus.set(RenderStatus::Working);
                    let title_val = title_ref
                        .get_untracked()
                        .map(|i| i.value())
                        .unwrap_or_default();
                    let body = RenderReq {
                        start,
                        end,
                        title: if title_val.is_empty() { None } else { Some(title_val) },
                    };
                    let pid = pid.get_value();
                    spawn_local(async move {
                        match post_json::<
                            RenderReq,
                            RenderResp,
                        >(&format!("/api/projects/{pid}/render"), &body)
                            .await
                        {
                            Ok(out) => {
                                rstatus.set(RenderStatus::Done(out.path));
                                // Refresh the project so the renders list updates (app.js
                                // re-fetches before the reload).
                                let _ = get_json::<
                                    serde_json::Value,
                                >(&format!("/api/projects/{pid}"))
                                    .await;
                                reload_after(800);
                            }
                            Err(e) => {
                                rendering.set(false);
                                rstatus.set(RenderStatus::Error(e));
                            }
                        }
                    });
                }
            >
                "Render 9:16 clip"
            </button>
            <div class="mt-8">
                {move || match rstatus.get() {
                    RenderStatus::Idle => ().into_any(),
                    RenderStatus::Invalid => {
                        view! { <div class="alert alert-error">"Enter a valid start/end."</div> }
                            .into_any()
                    }
                    RenderStatus::Working => {
                        view! {
                            <div class="row">
                                <div class="spinner"></div>
                                "Rendering… trim + reframe + captions."
                            </div>
                        }
                            .into_any()
                    }
                    RenderStatus::Done(path) => {
                        view! {
                            <div class="alert alert-info">
                                "✓ Rendered. "
                                <a href=path download="">
                                    "Download MP4"
                                </a>
                            </div>
                        }
                            .into_any()
                    }
                    RenderStatus::Error(e) => {
                        view! { <div class="alert alert-error">{format!("⚠ {e}")}</div> }
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}

// ── Sidebar: candidate list (app.js candidateList) ────────────────────────────

fn candidate_list(project: &Project, video_ref: NodeRef<html::Video>) -> impl IntoView {
    let count = project.candidates.len();
    // Sort a copy best-first (already best-first from plan_clips, but be safe).
    let mut sorted = project.candidates.clone();
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    view! {
        <div class="sidebar-section">
            <h3>{format!("Ranked moments ({count})")}</h3>
            <div class="cand-list">
                {sorted
                    .into_iter()
                    .map(|c| {
                        let hot = c.score >= 3.0;
                        let start = c.start;
                        let full_text = c.text.clone();
                        view! {
                            <div
                                class="cand-item"
                                title=full_text
                                on:click=move |_| {
                                    if let Some(v) = video_ref.get_untracked() {
                                        v.set_current_time(start);
                                        let _ = v.play();
                                    }
                                }
                            >
                                <div class="cand-head">
                                    <span class=if hot {
                                        "cand-score hot"
                                    } else {
                                        "cand-score"
                                    }>{format!("★ {:.2}", c.score)}</span>
                                    <span class="cand-time">
                                        {format!(
                                            "{}–{} · {:.1}s",
                                            fmt_time(c.start),
                                            fmt_time(c.end),
                                            c.duration,
                                        )}
                                    </span>
                                </div>
                                <div class="cand-text">{c.text}</div>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

// ── Render-card actions (app.js renderCardActions) ────────────────────────────

fn render_card_actions(
    project_id: &str,
    render_path: &str,
    title: &str,
    modal: RwSignal<Option<PublishTarget>>,
) -> impl IntoView {
    let file_url = format!("/api/projects/{project_id}/files/{render_path}");
    let target = PublishTarget {
        path: file_url.clone(),
        title: title.to_string(),
    };
    view! {
        <div class="row">
            <a class="btn btn-ghost btn-sm" href=file_url download="" title="Download">
                "↓"
            </a>
            <button
                class="btn btn-primary btn-sm"
                title="Publish to Postiz"
                on:click=move |_| modal.set(Some(target.clone()))
            >
                "📤"
            </button>
        </div>
    }
}

// ── Sidebar: renders (app.js rendersList) ─────────────────────────────────────

fn renders_list(project: &Project, modal: RwSignal<Option<PublishTarget>>) -> impl IntoView {
    let pid = project.id.clone();
    let renders = project.renders.clone();
    let count = renders.len();
    view! {
        <div class="sidebar-section">
            <h3>{format!("Renders ({count})")}</h3>
            <div class="renders">
                {renders
                    .into_iter()
                    .map(|r| {
                        let file_name = r.path.rsplit('/').next().unwrap_or_default().to_string();
                        let card_title = if r.title.is_empty() {
                            "untitled".to_string()
                        } else {
                            r.title.clone()
                        };
                        view! {
                            <div class="render-card">
                                <div class="render-thumb">"📦"</div>
                                <div class="render-info">
                                    <div class="render-title">{card_title}</div>
                                    <div class="render-meta">{file_name}</div>
                                </div>
                                {render_card_actions(&pid, &r.path, &r.title, modal)}
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

// ── Sidebar: storytelling + commentary (app.js _studioOutputList) ─────────────

fn studio_output_list(
    items: &[Render],
    title: &str,
    icon: &'static str,
    badge_color: &'static str,
    project_id: &str,
    modal: RwSignal<Option<PublishTarget>>,
) -> impl IntoView {
    let heading = format!("{title} ({})", items.len());
    let pid = project_id.to_string();
    let items = items.to_vec();
    view! {
        <div class="sidebar-section">
            <h3>{heading}</h3>
            <div class="renders">
                {items
                    .into_iter()
                    .map(|r| {
                        let file_name = r.path.rsplit('/').next().unwrap_or_default().to_string();
                        let card_title = if r.title.is_empty() {
                            "untitled".to_string()
                        } else {
                            r.title.clone()
                        };
                        view! {
                            <div class="render-card">
                                <div
                                    class="render-thumb"
                                    style=format!("background: {badge_color}; color: white;")
                                >
                                    {icon}
                                </div>
                                <div class="render-info">
                                    <div class="render-title">{card_title}</div>
                                    <div class="render-meta">{file_name}</div>
                                </div>
                                {render_card_actions(&pid, &r.path, &r.title, modal)}
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

// ── Sidebar: compiles (app.js compilesList) ───────────────────────────────────

fn compiles_list(project: &Project, modal: RwSignal<Option<PublishTarget>>) -> impl IntoView {
    let pid = project.id.clone();
    let compiles = project.compiles.clone();
    let count = compiles.len();
    view! {
        <div class="sidebar-section">
            <h3>{format!("Compilations ({count})")}</h3>
            <div class="renders">
                {compiles
                    .into_iter()
                    .map(|r| {
                        let file_name = r.path.rsplit('/').next().unwrap_or_default().to_string();
                        let card_title = if r.title.is_empty() {
                            "ranking compilation".to_string()
                        } else {
                            r.title.clone()
                        };
                        view! {
                            <div class="render-card">
                                <div
                                    class="render-thumb"
                                    style="background: var(--brand-gradient); color: white;"
                                >
                                    "🏆"
                                </div>
                                <div class="render-info">
                                    <div class="render-title">{card_title}</div>
                                    <div class="render-meta">{file_name}</div>
                                </div>
                                {render_card_actions(&pid, &r.path, &r.title, modal)}
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

// ── Sidebar: ranking compile builder (app.js compileSection) ──────────────────

fn compile_section(project: &Project) -> impl IntoView {
    let pid = StoredValue::new(project.id.clone());

    // Start from top-N candidates, best LAST (countup reveal — the reference
    // reel saves the best moment for last).
    let mut by_score = project.candidates.clone();
    by_score.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let initial: Vec<Pick> = by_score
        .into_iter()
        .take(5)
        .map(|c| Pick {
            start: c.start,
            end: c.end,
            label: take_chars(&c.text, 60),
        })
        .collect();
    let picks = RwSignal::new(initial);

    let title_default: String = project
        .candidates
        .first()
        .map(|c| take_chars(&c.text, 60))
        .unwrap_or_default();
    let title_ref = NodeRef::<html::Input>::new();

    let order = RwSignal::new("countup");
    let status = RwSignal::new(CompileStatus::Idle);
    let compiling = RwSignal::new(false);

    view! {
        <div class="sidebar-section compile-section">
            <h3>"🏆 Ranking compilation"</h3>
            <p class="muted" style="font-size: 12px; margin: 0 0 10px;">
                "Cuts each ranked moment, stamps a rank number on the left edge, concatenates into one 9:16 video."
            </p>
            <input
                class="input"
                placeholder="Hook title (e.g. 'Top 5 Funniest Moments')"
                value=title_default
                node_ref=title_ref
            />
            <div class="field" style="margin-top: 10px;">
                <label>"Reveal order"</label>
                <div class="compile-order-toggle">
                    <button
                        class=move || {
                            if order.get() == "countup" {
                                "compile-order-btn active"
                            } else {
                                "compile-order-btn"
                            }
                        }
                        on:click=move |_| order.set("countup")
                    >
                        "▲ Best last"
                    </button>
                    <button
                        class=move || {
                            if order.get() == "countdown" {
                                "compile-order-btn active"
                            } else {
                                "compile-order-btn"
                            }
                        }
                        on:click=move |_| order.set("countdown")
                    >
                        "▼ Best first"
                    </button>
                </div>
            </div>
            <div class="compile-list">
                {move || {
                    let ps = picks.get();
                    let len = ps.len();
                    let rows = ps
                        .into_iter()
                        .enumerate()
                        .map(|(i, pick)| {
                            let rank_style = if i + 1 == len {
                                "background: var(--brand-gradient);"
                            } else {
                                ""
                            };
                            let text = if pick.label.is_empty() {
                                format!("{:.1}–{:.1}s", pick.start, pick.end)
                            } else {
                                pick.label.clone()
                            };
                            view! {
                                <div class="compile-item">
                                    <div class="compile-rank" style=rank_style>
                                        {(i + 1).to_string()}
                                    </div>
                                    <div class="compile-pick-text">{text}</div>
                                    <div class="compile-controls">
                                        <button
                                            class="btn btn-ghost btn-sm"
                                            title="Move up"
                                            on:click=move |_| {
                                                if i == 0 {
                                                    return;
                                                }
                                                picks.update(|v| v.swap(i - 1, i));
                                            }
                                        >
                                            "↑"
                                        </button>
                                        <button
                                            class="btn btn-ghost btn-sm"
                                            title="Move down"
                                            on:click=move |_| {
                                                picks
                                                    .update(|v| {
                                                        if i + 1 < v.len() {
                                                            v.swap(i, i + 1);
                                                        }
                                                    });
                                            }
                                        >
                                            "↓"
                                        </button>
                                        <button
                                            class="btn btn-danger btn-sm"
                                            title="Remove"
                                            on:click=move |_| {
                                                picks
                                                    .update(|v| {
                                                        if i < v.len() {
                                                            v.remove(i);
                                                        }
                                                    });
                                            }
                                        >
                                            "✕"
                                        </button>
                                    </div>
                                </div>
                            }
                        })
                        .collect_view();
                    view! {
                        {rows}
                        {(len < 2)
                            .then(|| {
                                view! {
                                    <div class="muted mt-8" style="font-size: 12px;">
                                        "Need at least 2 picks to compile."
                                    </div>
                                }
                            })}
                    }
                }}
            </div>
            <div style="margin-top: 12px;">
                <button
                    class="btn btn-primary"
                    style="width: 100%;"
                    disabled=move || compiling.get()
                    on:click=move |_| {
                        let ps = picks.get_untracked();
                        if ps.len() < 2 {
                            status.set(CompileStatus::NeedTwo);
                            return;
                        }
                        if compiling.get_untracked() {
                            return;
                        }
                        compiling.set(true);
                        status.set(CompileStatus::Working(ps.len()));
                        let title_val = title_ref
                            .get_untracked()
                            .map(|i| i.value())
                            .unwrap_or_default();
                        let body = CompileReq {
                            items: ps
                                .iter()
                                .map(|p| CompileItemReq {
                                    start: p.start,
                                    end: p.end,
                                    label: p.label.clone(),
                                })
                                .collect(),
                            title: if title_val.is_empty() { None } else { Some(title_val) },
                            order: order.get_untracked().to_string(),
                        };
                        let id = pid.get_value();
                        spawn_local(async move {
                            match post_json::<
                                CompileReq,
                                CompileResp,
                            >(&format!("/api/projects/{id}/compile"), &body)
                                .await
                            {
                                Ok(out) => {
                                    status
                                        .set(CompileStatus::Done {
                                            segments: out.segments,
                                            duration: out.duration,
                                            path: out.path,
                                        });
                                    reload_after(1000);
                                }
                                Err(e) => {
                                    compiling.set(false);
                                    status.set(CompileStatus::Error(e));
                                }
                            }
                        });
                    }
                >
                    "Compile ranking video"
                </button>
            </div>
            <div class="mt-8">
                {move || match status.get() {
                    CompileStatus::Idle => ().into_any(),
                    CompileStatus::NeedTwo => {
                        view! {
                            <div class="alert alert-warn">"Add at least 2 picks to compile."</div>
                        }
                            .into_any()
                    }
                    CompileStatus::Working(n) => {
                        view! {
                            <div class="row">
                                <div class="spinner"></div>
                                {format!("Compiling {n} clips → one ranking video…")}
                            </div>
                        }
                            .into_any()
                    }
                    CompileStatus::Done { segments, duration, path } => {
                        view! {
                            <div class="alert alert-info">
                                {format!("✓ Compiled {segments} clips ({duration:.1}s). ")}
                                <a href=path download="">
                                    "Download MP4"
                                </a>
                            </div>
                        }
                            .into_any()
                    }
                    CompileStatus::Error(e) => {
                        view! { <div class="alert alert-error">{format!("⚠ {e}")}</div> }
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}

// ── Publish modal — Postiz publishing from any rendered card ──────────────────
// ⛔ SHARED POSTIZ ACCOUNT — same calls, same fields as app.js openPublishModal.

fn publish_modal_view(
    target: PublishTarget,
    modal: RwSignal<Option<PublishTarget>>,
) -> impl IntoView {
    let path = StoredValue::new(target.path);
    let suggested = StoredValue::new(target.title);

    let state = RwSignal::new(IntState::Loading);
    spawn_local(async move {
        match get_json::<IntegrationsResp>("/api/postiz/integrations").await {
            Ok(d) => state.set(IntState::Loaded(d)),
            Err(_) => state.set(IntState::Failed),
        }
    });

    let chan_ref = NodeRef::<html::Select>::new();
    let title_ref = NodeRef::<html::Input>::new();
    let caption_ref = NodeRef::<html::Textarea>::new();
    let sched = RwSignal::new("now");
    let pub_status = RwSignal::new(PubStatus::Idle);
    let publishing = RwSignal::new(false);
    let done = RwSignal::new(false);

    view! {
        <div
            class="modal-backdrop"
            on:click=move |ev| {
                let clicked_backdrop = ev
                    .target()
                    .zip(ev.current_target())
                    .is_some_and(|(t, ct)| t == ct);
                if clicked_backdrop {
                    modal.set(None);
                }
            }
        >
            <div class="modal" style="max-width: 540px;">
                <h3>"📤 Publish to Postiz"</h3>
                <p class="muted" style="font-size: 13px; margin: 0 0 16px;">
                    "Uploads the MP4 + creates a post on the chosen YouTube channel via Postiz."
                </p>
                {move || match state.get() {
                    IntState::Loading => {
                        view! {
                            <div class="row">
                                <div class="spinner"></div>
                                "Loading channels…"
                            </div>
                        }
                            .into_any()
                    }
                    IntState::Failed => {
                        modal_error("POSTIZ_API_TOKEN not configured. Add it to .env.", modal)
                            .into_any()
                    }
                    IntState::Loaded(d) if !d.available => {
                        let msg = if !d.token_configured {
                            "POSTIZ_API_TOKEN not configured. Add it to .env."
                        } else {
                            "Couldn't reach Postiz. Check the token + network."
                        };
                        modal_error(msg, modal).into_any()
                    }
                    IntState::Loaded(d) => {
                        let options = d
                            .integrations
                            .iter()
                            .filter(|i| i.identifier == "youtube" && !i.disabled)
                            .map(|i| {
                                let profile = i.profile.strip_prefix('@').unwrap_or(&i.profile);
                                view! {
                                    <option value=i.id.clone()>
                                        {format!("{} (@{profile})", i.name)}
                                    </option>
                                }
                            })
                            .collect_view();
                        view! {
                            <div class="field">
                                <label>"Channel"</label>
                                <select class="select" node_ref=chan_ref>
                                    {options}
                                </select>
                            </div>
                            <div class="field">
                                <label>"Title"</label>
                                <input
                                    class="input"
                                    value=suggested.get_value()
                                    placeholder="YouTube title (≤100 chars)"
                                    node_ref=title_ref
                                />
                            </div>
                            <div class="field">
                                <label>"Description (optional)"</label>
                                <textarea
                                    class="textarea"
                                    style="min-height: 60px;"
                                    placeholder="Description (default: title + #shorts)"
                                    node_ref=caption_ref
                                ></textarea>
                            </div>
                            <div class="field">
                                <label>"When"</label>
                                <div class="compile-order-toggle" style="margin-top: 4px;">
                                    <button
                                        class=move || {
                                            if sched.get() == "now" {
                                                "compile-order-btn active"
                                            } else {
                                                "compile-order-btn"
                                            }
                                        }
                                        on:click=move |_| sched.set("now")
                                    >
                                        "⚡ Post now"
                                    </button>
                                    <button
                                        class=move || {
                                            if sched.get() == "schedule" {
                                                "compile-order-btn active"
                                            } else {
                                                "compile-order-btn"
                                            }
                                        }
                                        on:click=move |_| sched.set("schedule")
                                    >
                                        "📅 Schedule"
                                    </button>
                                </div>
                            </div>
                            <button
                                class="btn btn-primary"
                                style="width: 100%; margin-top: 12px;"
                                disabled=move || publishing.get()
                                on:click=move |_| {
                                    let title_val = title_ref
                                        .get_untracked()
                                        .map(|i| i.value())
                                        .unwrap_or_default();
                                    if title_val.trim().is_empty() {
                                        pub_status.set(PubStatus::Warn);
                                        return;
                                    }
                                    if publishing.get_untracked() {
                                        return;
                                    }
                                    publishing.set(true);
                                    pub_status.set(PubStatus::Working);
                                    let caption_val = caption_ref
                                        .get_untracked()
                                        .map(|t| t.value())
                                        .unwrap_or_default();
                                    let body = PublishReq {
                                        path: path.get_value(),
                                        integration_id: chan_ref
                                            .get_untracked()
                                            .map(|s| s.value())
                                            .unwrap_or_default(),
                                        title: title_val,
                                        caption: if caption_val.is_empty() {
                                            None
                                        } else {
                                            Some(caption_val)
                                        },
                                        schedule: sched.get_untracked().to_string(),
                                    };
                                    spawn_local(async move {
                                        match post_json::<
                                            PublishReq,
                                            PublishResp,
                                        >("/api/postiz/publish", &body)
                                            .await
                                        {
                                            Ok(out) => {
                                                pub_status.set(PubStatus::Done(out.post_id));
                                                done.set(true);
                                            }
                                            Err(e) => {
                                                publishing.set(false);
                                                pub_status.set(PubStatus::Error(e));
                                            }
                                        }
                                    });
                                }
                            >
                                {move || if done.get() { "✓ Done" } else { "📤 Publish" }}
                            </button>
                            <div class="mt-8">
                                {move || match pub_status.get() {
                                    PubStatus::Idle => ().into_any(),
                                    PubStatus::Warn => {
                                        view! {
                                            <div class="alert alert-warn">"Add a title first."</div>
                                        }
                                            .into_any()
                                    }
                                    PubStatus::Working => {
                                        view! {
                                            <div class="row">
                                                <div class="spinner"></div>
                                                "Uploading to Postiz + creating post… (30-90s)"
                                            </div>
                                        }
                                            .into_any()
                                    }
                                    PubStatus::Done(post_id) => {
                                        view! {
                                            <div class="alert alert-info">
                                                "✓ Published. Postiz post id: "
                                                <span class="mono">{post_id}</span>
                                            </div>
                                        }
                                            .into_any()
                                    }
                                    PubStatus::Error(e) => {
                                        view! {
                                            <div class="alert alert-error">{format!("⚠ {e}")}</div>
                                        }
                                            .into_any()
                                    }
                                }}
                            </div>
                            <div class="modal-actions">
                                <button class="btn btn-ghost" on:click=move |_| modal.set(None)>
                                    "Close"
                                </button>
                            </div>
                        }
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Unavailable-Postiz branch of the modal (alert + Close).
fn modal_error(msg: &'static str, modal: RwSignal<Option<PublishTarget>>) -> impl IntoView {
    view! {
        <div class="alert alert-error">{msg}</div>
        <div class="modal-actions">
            <button class="btn" on:click=move |_| modal.set(None)>
                "Close"
            </button>
        </div>
    }
}
