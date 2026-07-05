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

use crate::{captions, clip, config, srt, transcribe};

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
        .route("/api/projects/:id/files/*path", get(serve_project_file))
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
    println!("  · projects dir: {}", config::data_dir(root).join("editor").display());
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
    let duration = tokio::task::spawn_blocking(move || probe_duration(&dest_owned.to_string_lossy()))
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
fn default_min_len() -> f64 { 15.0 }
fn default_max_len() -> f64 { clip::MAX_CLIP_SEC }

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
    let result = tokio::task::spawn_blocking(move || -> Result<(Vec<SerdeSegment>, Vec<SerdeCandidate>)> {
        let workdir = std::env::temp_dir().join(format!("ycp-editor-{id_clone}"));
        std::fs::create_dir_all(&workdir)?;
        let segments = transcribe::transcribe(&root, &video, &workdir)?;
        let cands = clip::plan_clips(&segments, min_len, max_len, top);
        let _ = std::fs::remove_dir_all(&workdir);
        let s_segs = segments
            .iter()
            .map(|sg| SerdeSegment { start: sg.start, end: sg.end, text: sg.text.clone() })
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
    })
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
        let projects_json = std::fs::read_to_string(root.join("data").join("editor").join(&id_for_task).join("transcript.json")).ok();
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
    p.renders.push(Render { path: result.0, title: title_used });
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
    resp.headers_mut().insert(header::CONTENT_TYPE, mime.as_ref().parse().unwrap());
    Ok(resp)
}

fn static_response(path: &str) -> Response {
    match WebAsset::get(path) {
        Some(asset) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut resp = Response::new(Body::from(asset.data.into_owned()));
            resp.headers_mut()
                .insert(header::CONTENT_TYPE, mime.as_ref().parse().unwrap());
            resp.headers_mut().insert(
                header::CACHE_CONTROL,
                "no-cache".parse().unwrap(),
            );
            resp
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found"))
            .unwrap(),
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

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
            let transcript: Vec<SerdeSegment> = std::fs::read_to_string(dir.join("transcript.json"))
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();
            let duration = std::fs::read_to_string(dir.join("duration.txt"))
                .ok()
                .and_then(|t| t.trim().parse().ok())
                .unwrap_or(0.0);
            let renders_root = dir.join("renders");
            let mut renders = Vec::new();
            if let Ok(rd) = std::fs::read_dir(&renders_root) {
                let mut mp4s: Vec<(String, PathBuf)> = Vec::new();
                for entry in rd.flatten() {
                    if entry.path().extension().and_then(|e| e.to_str()) == Some("mp4") {
                        let name = entry.file_name().to_string_lossy().to_string();
                        mp4s.push((name, entry.path()));
                    }
                }
                mp4s.sort_by(|a, b| a.0.cmp(&b.0));
                for (name, path) in mp4s {
                    let title = std::fs::read_to_string(format!("{}.title", path.display()))
                        .unwrap_or_default();
                    renders.push(Render {
                        path: format!("renders/{name}"),
                        title,
                    });
                }
            }
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
                Project { id, filename, duration, transcript, candidates: cands, renders },
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
        std::fs::write(dir.join("transcript.json"), serde_json::to_vec(&p.transcript)?)?;
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
    fn from(e: std::io::Error) -> Self { AppError(e.into()) }
}
impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self { AppError(e) }
}
