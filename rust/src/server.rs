//! `ycp serve` — wraps the clip pipeline in a browser editor for the team.
//!
//! One binary boots an axum server that exposes the existing `transcribe`,
//! `clip::plan_clips`, `clip::cut_vertical`, and `captions::burn_captions` modules
//! over a small REST API, with a viblo-inspired editor embedded in the binary
//! (rust-embed → still ships as a single static file).
//!
//! The pipeline stays the source of truth — this layer only orchestrates it for
//! interactive browser use. POST a video → we transcribe + rank candidate windows
//! → the editor shows them on a timeline → POST {start,end,title} → we cut, reframe
//! to 9:16, burn opus-style captions, and stream the rendered MP4 back.
//!
//! No DB, no billing, no auth — this is an internal team tool behind the firewall.
//! Projects live in `data/editor/<id>/` and are ephemeral.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use axum::{
    body::Body,
    extract::{Multipart, Path as AxumPath, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::{
    analytics, captions, clip, commentary, config, distribute, listicle, srt, story, transcribe,
    voice,
};
// Adapter::deliver is a trait method — bring it into scope for the publish path.
use distribute::Adapter;

/// Embedded static frontend (rust-embed) — bundled at compile time.
#[derive(RustEmbed)]
#[folder = "web/"]
struct WebAsset;

/// Per-process shared state: the project registry + project root.
#[derive(Clone)]
struct AppState {
    root: PathBuf,
    projects: Arc<RwLock<HashMap<String, Project>>>,
}

/// One editor project = an uploaded video + its transcript + ranked clip candidates.
#[derive(Clone, Serialize)]
#[allow(dead_code)]
struct Project {
    id: String,
    /// Filename of the source video, e.g. "raw.mp4".
    filename: String,
    /// Total source duration in seconds.
    duration: f64,
    /// Transcript segments, seconds-based (mirrors srt::Segment).
    transcript: Vec<SerdeSegment>,
    /// Ranked clip candidates, best first (mirrors clip::plan_clips).
    candidates: Vec<SerdeCandidate>,
    /// Renders produced from this project, keyed by candidate index.
    renders: Vec<Render>,
    /// Ranking-listicle compilations produced from this project.
    compiles: Vec<Render>,
    /// Storytelling-format renders (script → VO → gameplay bg).
    stories: Vec<Render>,
    /// Commentary-format renders (source clip + VO + ducked audio).
    commentary: Vec<Render>,
}

#[derive(Clone, Serialize, Deserialize)]
struct SerdeSegment {
    start: f64,
    end: f64,
    text: String,
}

#[derive(Clone, Serialize)]
struct SerdeCandidate {
    start: f64,
    end: f64,
    duration: f64,
    score: f64,
    text: String,
}

#[derive(Clone, Serialize)]
struct Render {
    /// Path relative to project dir, e.g. "renders/0.mp4".
    path: String,
    title: String,
}

/// Where on disk a project's files live.
fn project_dir(root: &Path, id: &str) -> PathBuf {
    config::data_dir(root).join("editor").join(id)
}

/// Where the source video lives inside a project.
fn source_video(root: &Path, id: &str) -> PathBuf {
    project_dir(root, id).join("source.mp4")
}

/// Serializes OmniVoice calls. The local Studio server lazy-loads the model and an
/// asyncio lock inside it crosses event loops if two requests hit on first load, so
/// we keep it strictly one-at-a-time at the editor layer too.
static STUDIO_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

// ── entrypoint ────────────────────────────────────────────────────────────────

/// Boot the editor server. `port` 0 lets the OS pick (used in tests).
pub async fn run(root: &Path, port: u16) -> Result<()> {
    let state = AppState {
        root: root.to_path_buf(),
        projects: Arc::new(RwLock::new(HashMap::new())),
    };

    // Pre-scan existing project dirs so a server restart doesn't wipe the dashboard.
    warm_cache(&state).await;

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/editor", get(index_handler)) // SPA: any app route → index.html
        .route("/api/health", get(health))
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/:id", get(get_project).delete(delete_project))
        .route("/api/projects/:id/upload", post(upload_video))
        .route("/api/projects/:id/transcribe", post(transcribe_project))
        .route("/api/projects/:id/render", post(render_clip))
        .route("/api/projects/:id/compile", post(compile_project))
        .route("/api/studio/render", post(studio_render))
        .route("/api/studio/source-clip", post(studio_source_clip))
        .route("/api/voices", get(list_voices_route))
        .route("/api/postiz/integrations", get(list_postiz_integrations))
        .route("/api/postiz/publish", post(publish_to_postiz))
        .route("/api/analytics/rollup", get(an_rollup))
        .route("/api/analytics/top", get(an_top))
        .route("/api/analytics/daily", get(an_daily))
        .route("/api/analytics/retention/:vid", get(an_retention))
        .route("/api/analytics/recommendations", get(an_recommendations))
        // LLM proxy for Page Agent — injects the server-side DeepSeek key so the
        // browser agent can drive the editor without the key landing in client JS.
        .route("/api/llm/proxy/chat/completions", post(llm_proxy_chat))
        .route("/api/projects/:id/files/*path", get(serve_project_file))
        // Side-by-side phase (TILLER-LOOP-PROMPT.md P0): the Leptos rewrite is
        // served at /next from rust/ui/dist while the old bundle keeps serving /.
        // ponytail: disk reads for now; folds into rust-embed at P5 cutover.
        .route("/next", get(next_ui_index))
        .route("/next/", get(next_ui_index))
        .route("/next/*path", get(next_ui_asset))
        .route("/static/*path", get(static_handler))
        .fallback(get(index_handler)) // unknown → SPA shell
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;
    let bound = listener.local_addr()?;
    println!("ycp editor → http://localhost:{}", bound.port());
    println!("  · project root: {}", root.display());
    println!(
        "  · projects dir: {}",
        config::data_dir(root).join("editor").display()
    );
    axum::serve(listener, app).await.context("axum::serve")?;
    Ok(())
}

// ── routes — projects ─────────────────────────────────────────────────────────

async fn health(State(_s): State<AppState>) -> Json<Value> {
    Json(json!({ "ok": true, "service": "ycp-editor" }))
}

/// List every project on disk (used by the dashboard).
async fn list_projects(State(s): State<AppState>) -> Json<Value> {
    let projects = s.projects.read().await;
    let summaries: Vec<Value> = projects
        .values()
        .map(|p| {
            json!({
                "id": p.id,
                "filename": p.filename,
                "duration": p.duration,
                "candidates": p.candidates.len(),
                "renders": p.renders.len(),
            })
        })
        .collect();
    Json(json!({ "projects": summaries }))
}

/// Create an empty project; the upload comes next.
#[derive(Deserialize)]
struct CreateBody {
    filename: Option<String>,
}
async fn create_project(
    State(s): State<AppState>,
    Json(body): Json<CreateBody>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let dir = project_dir(&s.root, &id);
    tokio::fs::create_dir_all(&dir).await?;
    let filename = body.filename.unwrap_or_else(|| "upload.mp4".into());
    let p = Project {
        id: id.clone(),
        filename,
        duration: 0.0,
        transcript: vec![],
        candidates: vec![],
        renders: vec![],
        compiles: vec![],
        stories: vec![],
        commentary: vec![],
    };
    s.projects.write().await.insert(id.clone(), p);
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

async fn get_project(
    State(s): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<Project>, AppError> {
    let projects = s.projects.read().await;
    projects
        .get(&id)
        .cloned()
        .ok_or_else(|| AppError(anyhow!("project not found")))
        .map(Json)
}

async fn delete_project(
    State(s): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Result<StatusCode, AppError> {
    s.projects.write().await.remove(&id);
    let dir = project_dir(&s.root, &id);
    if dir.exists() {
        tokio::fs::remove_dir_all(&dir).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Multipart upload — writes source.mp4, then probes duration.
async fn upload_video(
    State(s): State<AppState>,
    AxumPath(id): AxumPath<String>,
    mut multipart: Multipart,
) -> Result<Json<Value>, AppError> {
    // Verify project exists.
    {
        let projects = s.projects.read().await;
        if !projects.contains_key(&id) {
            return Err(AppError(anyhow!("project not found")));
        }
    }
    let dir = project_dir(&s.root, &id);
    tokio::fs::create_dir_all(&dir).await?;
    let dest = source_video(&s.root, &id);

    let mut saved_filename = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError(anyhow!("multipart: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "video" {
            let filename = field
                .file_name()
                .map(String::from)
                .unwrap_or_else(|| "source.mp4".into());
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError(anyhow!("read field: {e}")))?;
            tokio::fs::write(&dest, &data).await?;
            saved_filename = Some(filename);
        }
    }

    if !dest.exists() {
        return Err(AppError(anyhow!("no file field in upload")));
    }

    // Probe duration synchronously (ffmpeg is fast on this).
    let dest_owned = dest.clone();
    let duration =
        tokio::task::spawn_blocking(move || probe_duration(&dest_owned.to_string_lossy()))
            .await
            .map_err(|e| AppError(anyhow!("join: {e}")))??;

    let mut projects = s.projects.write().await;
    let p = projects
        .get_mut(&id)
        .ok_or_else(|| AppError(anyhow!("project vanished")))?;
    if let Some(f) = saved_filename {
        p.filename = f;
    }
    p.duration = duration;
    let _ = persist_project_meta(&s.root, p);
    Ok(Json(json!({ "id": id, "duration": duration })))
}

/// Transcribe + plan candidate clips. Heavy work — runs on the blocking pool.
#[derive(Deserialize)]
struct TranscribeBody {
    #[serde(default = "default_min_len")]
    min_len: f64,
    #[serde(default = "default_max_len")]
    max_len: f64,
    #[serde(default)]
    top: Option<usize>,
}
fn default_min_len() -> f64 {
    15.0
}
fn default_max_len() -> f64 {
    clip::MAX_CLIP_SEC
}

async fn transcribe_project(
    State(s): State<AppState>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<TranscribeBody>,
) -> Result<Json<Project>, AppError> {
    // Snapshot what we need before moving into spawn_blocking.
    let video = {
        let projects = s.projects.read().await;
        if !projects.contains_key(&id) {
            return Err(AppError(anyhow!("project not found")));
        }
        source_video(&s.root, &id)
    };
    if !video.exists() {
        return Err(AppError(anyhow!("no source video uploaded yet")));
    }

    let root = s.root.clone();
    let id_clone = id.clone();
    let min_len = body.min_len;
    let max_len = body.max_len;
    let top = body.top;
    let result = tokio::task::spawn_blocking(
        move || -> Result<(Vec<SerdeSegment>, Vec<SerdeCandidate>)> {
            let workdir = std::env::temp_dir().join(format!("ycp-editor-{id_clone}"));
            std::fs::create_dir_all(&workdir)?;
            let segments = transcribe::transcribe(&root, &video, &workdir)?;
            let cands = clip::plan_clips(&segments, min_len, max_len, top);
            let _ = std::fs::remove_dir_all(&workdir);
            let s_segs = segments
                .iter()
                .map(|sg| SerdeSegment {
                    start: sg.start,
                    end: sg.end,
                    text: sg.text.clone(),
                })
                .collect();
            let s_cands = cands
                .iter()
                .map(|c| SerdeCandidate {
                    start: c.start,
                    end: c.end,
                    duration: c.duration(),
                    score: c.score,
                    text: c.text.clone(),
                })
                .collect();
            Ok((s_segs, s_cands))
        },
    )
    .await
    .map_err(|e| AppError(anyhow!("join: {e}")))??;

    let mut projects = s.projects.write().await;
    let p = projects
        .get_mut(&id)
        .ok_or_else(|| AppError(anyhow!("project vanished")))?;
    p.transcript = result.0.clone();
    p.candidates = result.1;
    // Persist the transcript so render_clip + a server restart can rebuild chunks.
    let _ = persist_project_meta(&s.root, p);
    Ok(Json(p.clone()))
}

/// Cut + reframe + burn captions for one chosen window. Returns the rendered file.
#[derive(Deserialize)]
struct RenderBody {
    /// seconds
    start: f64,
    /// seconds
    end: f64,
    /// Hook title burned at the top of the frame (optional).
    #[serde(default)]
    title: Option<String>,
}
async fn render_clip(
    State(s): State<AppState>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<RenderBody>,
) -> Result<Json<Value>, AppError> {
    let video = {
        let projects = s.projects.read().await;
        if !projects.contains_key(&id) {
            return Err(AppError(anyhow!("project not found")));
        }
        source_video(&s.root, &id)
    };
    if !video.exists() {
        return Err(AppError(anyhow!("no source video uploaded yet")));
    }

    let root = s.root.clone();
    let id_for_task = id.clone();
    let title_for_task = body.title.clone();
    let title_used = body.title.unwrap_or_default();
    let start = body.start;
    let end = body.end;
    let result = tokio::task::spawn_blocking(move || -> Result<(String, f64)> {
        let workdir = std::env::temp_dir().join(format!("ycp-render-{id_for_task}"));
        std::fs::create_dir_all(&workdir)?;
        let cand = clip::Candidate::new(start, end, "", 0.0);
        let staged = workdir.join("staged.mp4");
        clip::cut_vertical(&root, &video, &cand, &staged, &workdir)?;

        // Build caption chunks from the source transcript overlapping [start,end].
        let projects_json = std::fs::read_to_string(
            root.join("data")
                .join("editor")
                .join(&id_for_task)
                .join("transcript.json"),
        )
        .ok();
        let segs: Vec<srt::Segment> = match projects_json {
            Some(t) => serde_json::from_str::<Vec<SerdeSegment>>(&t)
                .unwrap_or_default()
                .into_iter()
                .map(|s| srt::Segment::new(s.start, s.end, s.text))
                .collect(),
            None => vec![],
        };
        let sliced = srt::slice_and_shift(&segs, start, end);
        let chunks = captions::build_chunks(&sliced, captions::MAX_WORDS, captions::MIN_DWELL);

        let renders_dir = project_dir(&root, &id_for_task).join("renders");
        std::fs::create_dir_all(&renders_dir)?;
        let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S").to_string();
        let out_name = format!("{stamp}.mp4");
        let out_path = renders_dir.join(&out_name);
        let title_ref = title_for_task.as_deref();
        let settings = config::load_settings(&root).ok();

        // burn_captions is the caption overlay path; falls back to plain clip on failure.
        match captions::burn_captions(
            &staged,
            &chunks,
            &out_path,
            &workdir,
            title_ref,
            captions::FPS,
            captions::SIZE,
            None,
            settings.as_ref(),
        ) {
            Ok(_) => {}
            Err(e) => {
                // Caption render failed — ship the staged vertical clip as-is.
                eprintln!("  ! caption burn failed ({e}); shipping plain clip");
                std::fs::copy(&staged, &out_path)?;
            }
        }
        let _ = std::fs::remove_dir_all(&workdir);
        // Sidecar title.txt so warm_cache can rebuild the render's title after restart.
        if let Some(t) = title_for_task.as_ref() {
            let _ = std::fs::write(renders_dir.join(format!("{out_name}.title")), t);
        }
        let duration = end - start;
        Ok((format!("renders/{out_name}"), duration))
    })
    .await
    .map_err(|e| AppError(anyhow!("join: {e}")))??;

    let mut projects = s.projects.write().await;
    let p = projects
        .get_mut(&id)
        .ok_or_else(|| AppError(anyhow!("project vanished")))?;
    p.renders.push(Render {
        path: result.0,
        title: title_used,
    });
    let render_idx = p.renders.len() - 1;
    Ok(Json(json!({
        "path": format!("/api/projects/{id}/files/{}", p.renders[render_idx].path),
        "duration": result.1,
        "render_index": render_idx,
    })))
}

/// Persist the transcript alongside the source video so render can rebuild chunks later.
/// (Called from transcribe_project via a follow-up write — kept as a helper to keep that
/// handler readable.)
#[allow(dead_code)]
fn write_transcript_snapshot(root: &Path, id: &str, segs: &[SerdeSegment]) -> Result<()> {
    let p = project_dir(root, id).join("transcript.json");
    let v: Vec<SerdeSegment> = segs.to_vec();
    std::fs::write(p, serde_json::to_vec(&v)?)?;
    Ok(())
}

/// Compile N ranked windows into one countdown-style compilation video.
#[derive(Deserialize)]
struct CompileItem {
    start: f64,
    end: f64,
    #[serde(default)]
    label: Option<String>,
}
#[derive(Deserialize)]
struct CompileBody {
    items: Vec<CompileItem>,
    #[serde(default)]
    title: Option<String>,
    /// "countdown" (1,2,3...N — best first) or "countup" (N...3,2,1 — best last, the default
    /// reference-reel reveal). Default countdown for backward-compat feel.
    #[serde(default)]
    order: Option<String>,
}
async fn compile_project(
    State(s): State<AppState>,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<CompileBody>,
) -> Result<Json<Value>, AppError> {
    if body.items.is_empty() {
        return Err(AppError(anyhow!("need at least one item to compile")));
    }
    // Snapshot what we need before spawn_blocking.
    let video = {
        let projects = s.projects.read().await;
        if !projects.contains_key(&id) {
            return Err(AppError(anyhow!("project not found")));
        }
        source_video(&s.root, &id)
    };
    if !video.exists() {
        return Err(AppError(anyhow!("no source video uploaded yet")));
    }
    // Pull the transcript snapshot from disk (render transcribe_project persists it).
    let transcript_path = project_dir(&s.root, &id).join("transcript.json");
    let segs: Vec<srt::Segment> = std::fs::read_to_string(&transcript_path)
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<SerdeSegment>>(&t).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|x| srt::Segment::new(x.start, x.end, x.text))
        .collect();

    // Assign ranks 1..N by input order, then build RankItems.
    let items: Vec<listicle::RankItem> = body
        .items
        .iter()
        .enumerate()
        .map(|(i, it)| listicle::RankItem {
            start: it.start,
            end: it.end,
            rank: i + 1,
            label: it.label.clone().unwrap_or_default(),
        })
        .collect();
    let order = match body.order.as_deref().unwrap_or("countup") {
        "countdown" => listicle::Order::CountDown,
        _ => listicle::Order::CountUp,
    };
    let title = body.title.unwrap_or_default();
    let title_for_task = title.clone();
    let title_used = title.clone();
    let opts = listicle::CompileOpts { title, order };

    let root = s.root.clone();
    let id_for_task = id.clone();
    let items_for_task = items.clone();
    let opts_for_task = opts;
    let result = tokio::task::spawn_blocking(move || -> Result<(String, f64, usize)> {
        let compiles_dir = project_dir(&root, &id_for_task).join("compiles");
        std::fs::create_dir_all(&compiles_dir)?;
        let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S").to_string();
        let out_name = format!("{stamp}.mp4");
        let out_path = compiles_dir.join(&out_name);
        listicle::compile(
            &root,
            &video,
            &segs,
            &items_for_task,
            &opts_for_task,
            &out_path,
        )?;
        // Sidecar title for warm_cache.
        if !title_for_task.is_empty() {
            let _ = std::fs::write(
                compiles_dir.join(format!("{out_name}.title")),
                &title_for_task,
            );
        }
        let total: f64 = items_for_task.iter().map(|i| i.end - i.start).sum();
        Ok((format!("compiles/{out_name}"), total, items_for_task.len()))
    })
    .await
    .map_err(|e| AppError(anyhow!("join: {e}")))??;

    let mut projects = s.projects.write().await;
    let p = projects
        .get_mut(&id)
        .ok_or_else(|| AppError(anyhow!("project vanished")))?;
    p.compiles.push(Render {
        path: result.0,
        title: title_used,
    });
    Ok(Json(json!({
        "path": format!("/api/projects/{id}/files/{}", p.compiles.last().unwrap().path),
        "duration": result.1,
        "segments": result.2,
    })))
}

// ── Studio: format-aware renderer (storytelling / commentary) ─────────────────

#[derive(Deserialize)]
#[serde(tag = "format", rename_all = "lowercase")]
enum StudioBody {
    /// Script → VO → looping background → captions → 9:16.
    Story {
        script: String,
        voice: Option<String>,
        /// URL or path to the background footage (gameplay/Minecraft). Resolved via
        /// yt-dlp if it's a URL, used as-is if it's an existing local path.
        background: String,
        title: Option<String>,
        speed: Option<f32>,
        language: Option<String>,
        /// Optional project id to associate the output with (shows in dashboard).
        project: Option<String>,
    },
    /// Source clip + commentary script + VO + ducked original audio + captions.
    Commentary {
        /// URL or path to the source clip.
        source: String,
        script: String,
        voice: Option<String>,
        title: Option<String>,
        speed: Option<f32>,
        language: Option<String>,
        duck_volume: Option<f32>,
        project: Option<String>,
    },
}

/// Format-dispatching render endpoint. POST {format: "story"|"commentary", ...}.
/// Serialized through STUDIO_MUTEX so we never issue concurrent OmniVoice calls
/// (the local server lazy-loads the model and an asyncio lock crosses event loops
/// if you fire two requests at once on first load).
async fn studio_render(
    State(s): State<AppState>,
    Json(body): Json<StudioBody>,
) -> Result<Json<Value>, AppError> {
    let root = s.root.clone();
    let _guard = STUDIO_MUTEX.lock().await;
    let result = tokio::task::spawn_blocking(move || -> Result<(String, String)> {
        match body {
            StudioBody::Story {
                script,
                voice,
                background,
                title,
                speed,
                language,
                project,
            } => {
                let bg = resolve_source(&root, &background, "background")?;
                let opts = story::StoryOpts {
                    script,
                    voice: voice.unwrap_or_else(|| "default".to_string()),
                    background: bg,
                    title,
                    speed,
                    language,
                };
                let (id, out_path) = studio_output_path(&root, project.as_deref(), "stories")?;
                story::render(&root, &opts, &out_path)?;
                if let Some(t) = opts.title.as_ref() {
                    let _ = std::fs::write(format!("{}.title", out_path.display()), t);
                }
                Ok((id, out_path.to_string_lossy().into_owned()))
            }
            StudioBody::Commentary {
                source,
                script,
                voice,
                title,
                speed,
                language,
                duck_volume,
                project,
            } => {
                let src = resolve_source(&root, &source, "source")?;
                let opts = commentary::CommentaryOpts {
                    source: src,
                    script,
                    voice: voice.unwrap_or_else(|| "default".to_string()),
                    title,
                    speed,
                    language,
                    duck_volume: duck_volume.unwrap_or(0.25),
                };
                let title_clone = opts.title.clone();
                let (id, out_path) = studio_output_path(&root, project.as_deref(), "commentary")?;
                commentary::render(&root, &opts, &out_path)?;
                if let Some(t) = title_clone {
                    let _ = std::fs::write(format!("{}.title", out_path.display()), t);
                }
                Ok((id, out_path.to_string_lossy().into_owned()))
            }
        }
    })
    .await
    .map_err(|e| AppError(anyhow!("join: {e}")))??;

    // Format the path for browser download.
    let (id, abs_path) = result;
    // The serve_project_file route takes the path relative to data/editor/<id>/.
    // For studio outputs we wrote into data/editor/<id>/<subdir>/<stamp>.mp4 — derive that.
    let download_path = format_relative_for_download(&s.root, std::path::Path::new(&abs_path), &id);
    Ok(Json(json!({
        "path": download_path,
        "project": id,
    })))
}

/// yt-dlp fetch a URL → local mp4 (or pass-through if already a local path).
/// Used by studio_render to source background footage or commentary clips.
async fn studio_source_clip(
    State(s): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<Value>, AppError> {
    let url = body
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError(anyhow!("missing 'url' field")))?;
    let root = s.root.clone();
    let url_owned = url.to_string();
    let _guard = STUDIO_MUTEX.lock().await;
    let local = tokio::task::spawn_blocking(move || -> Result<String> {
        let local = resolve_source(&root, &url_owned, "source")?;
        Ok(local.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| AppError(anyhow!("join: {e}")))??;
    Ok(Json(json!({ "path": local })))
}

/// GET /api/voices — list OmniVoice voice profiles (empty if server's not running).
async fn list_voices_route(State(s): State<AppState>) -> Json<Value> {
    let root = s.root.clone();
    let voices = tokio::task::spawn_blocking(move || voice::list_voices(&root))
        .await
        .unwrap_or_default();
    let available = voice::available(&s.root);
    Json(json!({
        "available": available,
        "voices": voices.into_iter().map(|(id, name)| {
            json!({ "id": id, "name": name })
        }).collect::<Vec<_>>(),
    }))
}

// ── Postiz: list integrations + publish ──────────────────────────────────────
//
// Reuses distribute::PostizAdapter (the production posting path). The autopilot
// orchestrator runs the whole queue through run(); these routes expose the same
// adapter for one-off "publish this render now" calls from the editor.

/// GET /api/postiz/integrations — proxy the live Postiz integrations list so the
/// UI can show a channel picker. Returns `{available, token_configured, integrations:[]}`.
async fn list_postiz_integrations(State(s): State<AppState>) -> Json<Value> {
    let token = config::env_var(&s.root, "POSTIZ_API_TOKEN");
    let api_url = std::env::var("POSTIZ_API_URL").unwrap_or_else(|_| {
        config::load_settings(&s.root)
            .ok()
            .and_then(|y| {
                y["distribution"]["postiz"]["api_url"]
                    .as_str()
                    .map(String::from)
            })
            .unwrap_or_else(|| "https://api.postiz.com/public/v1".to_string())
    });

    let token_clone = token.clone();
    let api_clone = api_url.clone();
    let integrations = tokio::task::spawn_blocking(move || -> Result<Vec<Value>> {
        let token = match token_clone.as_ref() {
            Some(t) if !t.is_empty() => t.clone(),
            _ => bail!("POSTIZ_API_TOKEN not set"),
        };
        let resp = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?
            .get(format!("{api_clone}/integrations"))
            .header("Authorization", token)
            .send()?;
        if !resp.status().is_success() {
            bail!("Postiz HTTP {}", resp.status());
        }
        Ok(resp.json::<Vec<Value>>()?)
    })
    .await
    .map(|r| r.unwrap_or_default())
    .unwrap_or_default();

    Json(json!({
        "available": !integrations.is_empty(),
        "token_configured": token.is_some(),
        "api_url": api_url,
        "integrations": integrations,
    }))
}

/// POST /api/postiz/publish — publish one rendered MP4 to a chosen Postiz integration.
///
/// Body: `{ path, integration_id, title, caption?, platform?, privacy?, schedule?, date? }`
/// `path` is the editor-relative path returned by /render|/compile|/studio/render,
/// e.g. `/api/projects/<id>/files/renders/<stamp>.mp4`. We resolve it to the on-disk
/// absolute path, then hand off to PostizAdapter.deliver() which does the upload + post.
#[derive(Deserialize)]
struct PublishBody {
    /// Editor-relative URL path of the rendered file to publish.
    path: String,
    /// Postiz integration id (from GET /api/postiz/integrations).
    integration_id: String,
    title: String,
    #[serde(default)]
    caption: Option<String>,
    #[serde(default)]
    platform: Option<String>,
    #[serde(default)]
    privacy: Option<String>,
    /// "now" (default) or "schedule".
    #[serde(default)]
    schedule: Option<String>,
    /// ISO-8601 schedule time (only used when schedule="schedule").
    #[serde(default)]
    date: Option<String>,
}
async fn publish_to_postiz(
    State(s): State<AppState>,
    Json(body): Json<PublishBody>,
) -> Result<Json<Value>, AppError> {
    // Resolve the editor path back to an absolute on-disk path.
    // Format: /api/projects/<id>/files/<rel>
    let abs = resolve_editor_path(&s.root, &body.path)?;
    if !abs.exists() {
        return Err(AppError(anyhow!(
            "rendered file not found on disk: {}",
            abs.display()
        )));
    }

    let settings = config::load_settings(&s.root)
        .ok()
        .unwrap_or_else(|| serde_yaml::Value::Null);
    let api_url = std::env::var("POSTIZ_API_URL").unwrap_or_else(|_| {
        settings["distribution"]["postiz"]["api_url"]
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| "https://api.postiz.com/public/v1".to_string())
    });
    let token = config::env_var(&s.root, "POSTIZ_API_TOKEN")
        .ok_or_else(|| AppError(anyhow!("POSTIZ_API_TOKEN not set — add it to .env")))?;

    // Build a one-shot adapter with the chosen integration id directly (bypasses the
    // channels map in settings.yaml — the editor picks the channel per publish, so
    // we don't require a pre-mapped slug).
    let integration_id = body.integration_id.clone();
    let schedule_type = body.schedule.clone().unwrap_or_else(|| "now".to_string());
    let mut channels = std::collections::HashMap::new();
    channels.insert("__editor__".to_string(), integration_id.clone());

    let adapter = distribute::PostizAdapter::new(token, api_url, channels, schedule_type);
    let meta = distribute::DeliverMeta {
        clip_id: None,
        caption: body.caption.clone().unwrap_or_else(|| body.title.clone()),
        title: Some(body.title.clone()),
        channel: Some("__editor__".to_string()),
        platform: body
            .platform
            .clone()
            .or_else(|| Some("youtube".to_string())),
        privacy: body.privacy.clone().or_else(|| Some("public".to_string())),
        date: body.date.clone(),
    };

    let abs_for_task = abs.clone();
    let adapter_for_task = adapter;
    let meta_for_task = meta;
    let _guard = STUDIO_MUTEX.lock().await; // reuse the serialize-mutex — uploads are heavy
    let result = tokio::task::spawn_blocking(move || -> Result<(String,)> {
        let post_id = adapter_for_task.deliver(&abs_for_task, &meta_for_task)?;
        Ok((post_id,))
    })
    .await
    .map_err(|e| AppError(anyhow!("join: {e}")))??;

    // Best-effort sidecar so analytics can join render → post → metrics later.
    write_publish_sidecar(&abs, &result.0, &integration_id, &body.title);

    Ok(Json(json!({
        "ok": true,
        "post_id": result.0,
        "integration_id": integration_id,
        "title": body.title,
    })))
}

/// After publishing, write a `.publish.json` sidecar next to the rendered file so the
/// analytics page can later join "what we rendered" → "what we posted" → "how it performed."
/// Best-effort: never blocks the publish success on a sidecar write failing.
#[allow(dead_code)]
fn write_publish_sidecar(render_abs: &Path, post_id: &str, integration_id: &str, title: &str) {
    let sidecar = render_abs.with_extension("publish.json");
    let payload = json!({
        "post_id": post_id,
        "integration_id": integration_id,
        "title": title,
        "published_at": chrono::Utc::now().to_rfc3339(),
    });
    let _ = std::fs::write(
        &sidecar,
        serde_json::to_string_pretty(&payload).unwrap_or_default(),
    );
}

// ── Analytics: channel rollup + per-video + recommendations ───────────────────
//
// All hit YouTube Analytics via analytics.rs, which reuses the OAuth path from capture.rs
// and caches results for 1h (tight quotas). Routes degrade gracefully when OAuth isn't
// connected (return {configured: false}, never 500).

async fn an_rollup(
    State(s): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<DaysParam>,
) -> Json<Value> {
    let days = q.days.unwrap_or(28);
    let root = s.root.clone();
    let v = tokio::task::spawn_blocking(move || {
        analytics::channel_rollup(&root, days).unwrap_or_else(
            |e| json!({ "configured": analytics::configured(&root), "error": e.to_string() }),
        )
    })
    .await
    .unwrap_or(json!({ "error": "join failed" }));
    Json(v)
}

async fn an_top(
    State(s): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<TopParam>,
) -> Json<Value> {
    let days = q.days.unwrap_or(28);
    let limit = q.limit.unwrap_or(15);
    let root = s.root.clone();
    let v = tokio::task::spawn_blocking(move || {
        analytics::top_videos(&root, days, limit).unwrap_or_else(
            |e| json!({ "configured": analytics::configured(&root), "error": e.to_string() }),
        )
    })
    .await
    .unwrap_or(json!({ "error": "join failed" }));
    Json(v)
}

async fn an_daily(
    State(s): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<DaysParam>,
) -> Json<Value> {
    let days = q.days.unwrap_or(7);
    let root = s.root.clone();
    let v = tokio::task::spawn_blocking(move || {
        analytics::daily_series(&root, days).unwrap_or_else(
            |e| json!({ "configured": analytics::configured(&root), "error": e.to_string() }),
        )
    })
    .await
    .unwrap_or(json!({ "error": "join failed" }));
    Json(v)
}

async fn an_retention(State(s): State<AppState>, AxumPath(vid): AxumPath<String>) -> Json<Value> {
    let root = s.root.clone();
    let v = tokio::task::spawn_blocking(move || {
        analytics::retention_curve(&root, &vid).unwrap_or_else(
            |e| json!({ "configured": analytics::configured(&root), "error": e.to_string() }),
        )
    })
    .await
    .unwrap_or(json!({ "error": "join failed" }));
    Json(v)
}

async fn an_recommendations(State(s): State<AppState>) -> Json<Value> {
    let root = s.root.clone();
    let v = tokio::task::spawn_blocking(move || {
        analytics::recommendations(&root).unwrap_or_else(
            |e| json!({ "configured": analytics::configured(&root), "error": e.to_string() }),
        )
    })
    .await
    .unwrap_or(json!({ "error": "join failed" }));
    Json(v)
}

#[derive(Deserialize)]
struct DaysParam {
    days: Option<u32>,
}
#[derive(Deserialize)]
struct TopParam {
    days: Option<u32>,
    limit: Option<usize>,
}

// ── LLM proxy — keeps the DeepSeek key server-side for Page Agent ─────────────
//
// Page Agent needs an OpenAI-compatible chat endpoint + API key. Rather than
// bake DEEPSEEK_API_KEY into client JS (where anyone with DevTools could read
// it), we proxy the request through the editor server which injects the key.

const DEEPSEEK_BASE: &str = "https://api.deepseek.com/v1";
const DEEPSEEK_MODEL: &str = "deepseek-chat";
/// Copilot system prompt — GENERATED from the action map (rust/src/actions.rs)
/// so it can't drift from the real UI; cargo test enforces the markers.
static PAGE_AGENT_SYSTEM: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(crate::actions::system_prompt);

async fn llm_proxy_chat(
    State(s): State<AppState>,
    body: axum::body::Body,
) -> Result<Response, AppError> {
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|e| AppError(anyhow!("read body: {e}")))?;
    let mut req: Value =
        serde_json::from_slice(&bytes).map_err(|e| AppError(anyhow!("parse json: {e}")))?;

    // Force our system prompt to the front so the agent always knows the editor shape.
    if let Some(msgs) = req.get_mut("messages").and_then(Value::as_array_mut) {
        let sys = json!({ "role": "system", "content": &*PAGE_AGENT_SYSTEM });
        msgs.insert(0, sys);
    }
    // Pin the model to DeepSeek (the key the team holds) unless the caller set one.
    if req.get("model").is_none() {
        req["model"] = json!(DEEPSEEK_MODEL);
    }
    // Strip any caller-provided apiKey hint — we use the server-side key.
    if let Some(obj) = req.as_object_mut() {
        obj.remove("apiKey");
    }

    // LOUD failure (P4): no key → clear 503 in the OpenAI error shape the Page
    // Agent panel surfaces to the user — not a generic 500 buried in a console.
    let Some(key) = config::env_var(&s.root, "DEEPSEEK_API_KEY") else {
        return Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::CONTENT_TYPE, "application/json")],
            Json(json!({ "error": { "message": "DEEPSEEK_API_KEY not configured on the server — copilot disabled. Add it to .env and restart ycp serve." } }))
                .to_string(),
        )
            .into_response());
    };
    let stream = req.get("stream").and_then(Value::as_bool).unwrap_or(false);

    let req_for_task = req.clone();
    let resp = tokio::task::spawn_blocking(move || -> Result<Value> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(if stream {
                300
            } else {
                120
            }))
            .build()?;
        let r = client
            .post(format!("{DEEPSEEK_BASE}/chat/completions"))
            .header("Authorization", format!("Bearer {key}"))
            .header("Content-Type", "application/json")
            .json(&req_for_task)
            .send()?;
        if !r.status().is_success() {
            let status = r.status();
            let body = r.text().unwrap_or_default();
            let tail = &body[body.len().saturating_sub(500)..];
            bail!("DeepSeek HTTP {status}: {}", tail.trim());
        }
        Ok(r.json::<Value>()?)
    })
    .await
    .map_err(|e| AppError(anyhow!("join: {e}")))??;

    Ok(Json(resp).into_response())
}

/// Translate a `/api/projects/<id>/files/<rel>` URL into the absolute on-disk path.
fn resolve_editor_path(root: &Path, url_path: &str) -> Result<PathBuf> {
    // Reject any traversal.
    if url_path.contains("..") {
        bail!("bad path");
    }
    let prefix = "/api/projects/";
    let rest = url_path
        .strip_prefix(prefix)
        .ok_or_else(|| anyhow!("not an editor path"))?;
    let (id, after) = rest
        .split_once("/files/")
        .ok_or_else(|| anyhow!("malformed editor path"))?;
    let abs = config::data_dir(root).join("editor").join(id).join(after);
    Ok(abs)
}

/// Resolve a source spec (URL → yt-dlp download, else local path). `kind` is just for
/// error messages. Runs synchronously — callers should be on the blocking pool.
fn resolve_source(root: &Path, spec: &str, kind: &str) -> Result<std::path::PathBuf> {
    if spec.starts_with("http://") || spec.starts_with("https://") {
        // yt-dlp into a per-source temp dir.
        let hash = {
            use sha1::{Digest, Sha1};
            let mut h = Sha1::new();
            h.update(spec.as_bytes());
            let digest = h.finalize();
            digest
                .iter()
                .take(4)
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        let dir = config::data_dir(root)
            .join("studio")
            .join("sources")
            .join(&hash);
        std::fs::create_dir_all(&dir)?;
        let out = dir.join("source.mp4");
        if out.exists() {
            return Ok(out); // cached
        }
        let status = std::process::Command::new("yt-dlp")
            .args(["-f", "mp4/best", "--no-warnings", "-o"])
            .arg(&out)
            .arg(spec)
            .status()?;
        if !status.success() || !out.exists() {
            bail!("yt-dlp failed to fetch {kind}: {spec}");
        }
        Ok(out)
    } else {
        let p = std::path::PathBuf::from(spec);
        if !p.exists() {
            bail!("{kind} not found: {spec}");
        }
        Ok(p)
    }
}

/// Build the output path for a studio render: data/editor/<id>/<subdir>/<stamp>.mp4.
/// `project` may be None → a fresh synthetic project id is allocated.
fn studio_output_path(
    root: &Path,
    project: Option<&str>,
    subdir: &str,
) -> Result<(String, std::path::PathBuf)> {
    let id = match project {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => uuid::Uuid::new_v4().to_string()[..8].to_string(),
    };
    let dir = config::data_dir(root).join("editor").join(&id).join(subdir);
    std::fs::create_dir_all(&dir)?;
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S").to_string();
    Ok((id, dir.join(format!("{stamp}.mp4"))))
}

/// Convert an absolute output path into the form the serve_project_file route expects:
/// /api/projects/<id>/files/<subdir>/<stamp>.mp4
fn format_relative_for_download(root: &Path, abs_path: &Path, id: &str) -> String {
    let proj_dir = config::data_dir(root).join("editor").join(id);
    if let Ok(rel) = abs_path.strip_prefix(&proj_dir) {
        let rel_str = rel.to_string_lossy().trim_start_matches('/').to_string();
        return format!("/api/projects/{id}/files/{rel_str}");
    }
    // Fallback: just serve the absolute path raw (will 404, but the error is debuggable).
    format!(
        "/api/projects/{id}/files/{}",
        abs_path.file_name().unwrap_or_default().to_string_lossy()
    )
}

// ── routes — static + project files ───────────────────────────────────────────

/// Serve `/` and the SPA shell.
async fn index_handler() -> Response {
    static_response("index.html")
}

/// Serve embedded static files under /static/...
async fn static_handler(AxumPath(path): AxumPath<String>) -> Response {
    static_response(&path)
}

/// Stream a rendered video back to the browser.
async fn serve_project_file(
    State(s): State<AppState>,
    AxumPath((id, sub)): AxumPath<(String, String)>,
) -> Result<Response, AppError> {
    // Reject any path traversal.
    if sub.contains("..") {
        return Err(AppError(anyhow!("bad path")));
    }
    let full = project_dir(&s.root, &id).join(&sub);
    if !full.exists() {
        return Err(AppError(anyhow!("not found")));
    }
    let mime = mime_guess::from_path(&full).first_or_octet_stream();
    let bytes = tokio::fs::read(&full).await?;
    let mut resp = Response::new(Body::from(bytes));
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, mime.as_ref().parse().unwrap());
    Ok(resp)
}

/// Serve the Leptos rewrite (rust/ui/dist) at /next during the side-by-side phase.
async fn next_ui_index(State(s): State<AppState>) -> Response {
    next_dist_file(&s.root, "index.html").await
}

async fn next_ui_asset(State(s): State<AppState>, AxumPath(path): AxumPath<String>) -> Response {
    next_dist_file(&s.root, &path).await
}

async fn next_dist_file(root: &Path, rel: &str) -> Response {
    let dist = root.join("rust/ui/dist");
    // Trust boundary: rel comes off the URL — canonicalize and require the
    // result to stay inside dist (same posture as serve_project_file).
    let full = match dist.join(rel).canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // Unknown path → SPA shell (hash router owns the route)… unless the
            // dist was never built, then say so instead of a blank page.
            let index = dist.join("index.html");
            if !index.exists() {
                return Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Body::from(
                        "new UI not built — run: cd rust/ui && trunk build",
                    ))
                    .unwrap();
            }
            return next_dist_bytes(&index).await;
        }
    };
    match dist.canonicalize() {
        Ok(base) if full.starts_with(&base) => next_dist_bytes(&full).await,
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found"))
            .unwrap(),
    }
}

async fn next_dist_bytes(path: &Path) -> Response {
    match tokio::fs::read(path).await {
        Ok(bytes) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut resp = Response::new(Body::from(bytes));
            resp.headers_mut()
                .insert(header::CONTENT_TYPE, mime.as_ref().parse().unwrap());
            resp.headers_mut()
                .insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
            resp
        }
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found"))
            .unwrap(),
    }
}

fn static_response(path: &str) -> Response {
    match WebAsset::get(path) {
        Some(asset) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut resp = Response::new(Body::from(asset.data.into_owned()));
            resp.headers_mut()
                .insert(header::CONTENT_TYPE, mime.as_ref().parse().unwrap());
            resp.headers_mut()
                .insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
            resp
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found"))
            .unwrap(),
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Scan a directory of MP4 renders/compilations and rebuild the `Render` list with
/// sidecar titles. Used by `warm_cache` for both `renders/` and `compiles/`.
fn scan_renders_dir(dir: &Path, prefix: &str) -> Vec<Render> {
    let mut out = Vec::new();
    let rd = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return out,
    };
    let mut mp4s: Vec<(String, PathBuf)> = Vec::new();
    for entry in rd.flatten() {
        if entry.path().extension().and_then(|e| e.to_str()) == Some("mp4") {
            let name = entry.file_name().to_string_lossy().to_string();
            mp4s.push((name, entry.path()));
        }
    }
    mp4s.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, path) in mp4s {
        let title =
            std::fs::read_to_string(format!("{}.title", path.display())).unwrap_or_default();
        out.push(Render {
            path: format!("{prefix}/{name}"),
            title,
        });
    }
    out
}

/// ffprobe → duration seconds. Errors propagate; callers fall back to 0.
fn probe_duration(path: &str) -> Result<f64> {
    let out = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            path,
        ])
        .output()
        .context("ffprobe")?;
    if !out.status.success() {
        bail!("ffprobe failed");
    }
    let s = String::from_utf8_lossy(&out.stdout);
    s.trim().parse::<f64>().context("parse duration")
}

/// On boot, walk data/editor/ and rebuild the in-memory project registry from what's
/// on disk — so a server restart doesn't blank the dashboard.
async fn warm_cache(s: &AppState) {
    let editor_root = config::data_dir(&s.root).join("editor");
    let mut entries = match tokio::fs::read_dir(&editor_root).await {
        Ok(e) => e,
        Err(_) => return,
    };
    let mut to_load = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let id = entry.file_name().to_string_lossy().to_string();
        let dir = entry.path();
        // Reconstruct state from what's on disk.
        let source = dir.join("source.mp4");
        if !source.exists() {
            continue;
        }
        to_load.push((id, dir));
    }
    drop(entries);

    let root = s.root.clone();
    let loaded = tokio::task::spawn_blocking(move || -> Vec<(String, Project)> {
        let mut out = Vec::new();
        for (id, dir) in to_load {
            let transcript: Vec<SerdeSegment> =
                std::fs::read_to_string(dir.join("transcript.json"))
                    .ok()
                    .and_then(|t| serde_json::from_str(&t).ok())
                    .unwrap_or_default();
            let duration = std::fs::read_to_string(dir.join("duration.txt"))
                .ok()
                .and_then(|t| t.trim().parse().ok())
                .unwrap_or(0.0);
            let renders = scan_renders_dir(&dir.join("renders"), "renders");
            let compiles = scan_renders_dir(&dir.join("compiles"), "compiles");
            let stories = scan_renders_dir(&dir.join("stories"), "stories");
            let commentary = scan_renders_dir(&dir.join("commentary"), "commentary");
            // Re-plan candidates from the transcript if it exists.
            let segs: Vec<srt::Segment> = transcript
                .iter()
                .map(|s| srt::Segment::new(s.start, s.end, s.text.clone()))
                .collect();
            let cands: Vec<SerdeCandidate> = if !segs.is_empty() {
                clip::plan_clips(&segs, 15.0, clip::MAX_CLIP_SEC, None)
                    .iter()
                    .map(|c| SerdeCandidate {
                        start: c.start,
                        end: c.end,
                        duration: c.duration(),
                        score: c.score,
                        text: c.text.clone(),
                    })
                    .collect()
            } else {
                vec![]
            };
            let filename = std::fs::read_to_string(dir.join("filename.txt"))
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "upload.mp4".into());
            out.push((
                id.clone(),
                Project {
                    id,
                    filename,
                    duration,
                    transcript,
                    candidates: cands,
                    renders,
                    compiles,
                    stories,
                    commentary,
                },
            ));
        }
        let _ = root; // touch root to keep the closure's intent clear (future: re-probe durations)
        out
    })
    .await
    .unwrap_or_default();

    let mut projects = s.projects.write().await;
    for (id, p) in loaded {
        projects.insert(id, p);
    }
}

/// Persist a project's transcript + duration + filename so render + warm_cache can
/// rebuild state after the server restarts.
fn persist_project_meta(root: &Path, p: &Project) -> Result<()> {
    let dir = project_dir(root, &p.id);
    std::fs::create_dir_all(&dir)?;
    if !p.transcript.is_empty() {
        std::fs::write(
            dir.join("transcript.json"),
            serde_json::to_vec(&p.transcript)?,
        )?;
    }
    std::fs::write(dir.join("duration.txt"), format!("{}", p.duration))?;
    std::fs::write(dir.join("filename.txt"), &p.filename)?;
    Ok(())
}

// ── error + extensions ────────────────────────────────────────────────────────

/// anyhow → 500 with the message body (the editor surfaces this in the UI).
#[derive(Debug)]
struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let msg = format!("{:#}", self.0);
        eprintln!("  ! editor error: {msg}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "application/json")],
            Json(json!({ "error": msg })).to_string(),
        )
            .into_response()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError(e.into())
    }
}
impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError(e)
    }
}
