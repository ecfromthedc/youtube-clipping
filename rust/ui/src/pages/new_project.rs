//! New project (upload flow) — class-for-class port of `newProjectPage`
//! (rust/web/app.js ~206-261). Drop or pick a video → POST /api/projects →
//! multipart upload → auto-advance to #/p/{id} where transcription runs.

use std::time::Duration;

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};

use crate::http::post_json;

#[derive(Serialize)]
struct CreateProjectBody {
    filename: String,
}

/// POST /api/projects response (server.rs `create_project` → `{ "id": … }`).
#[derive(Deserialize)]
struct CreatedProject {
    #[serde(default)]
    id: String,
}

/// Upload lifecycle for the status strip under the dropzone (mirrors the
/// clear-then-append sequence in app.js `handleFile`).
#[derive(Clone)]
enum UploadState {
    Idle,
    Uploading(String),
    Done,
    Error(String),
}

#[component]
pub fn NewProject() -> impl IntoView {
    let state = RwSignal::new(UploadState::Idle);
    let dragging = RwSignal::new(false);

    let handle_file = move |file: web_sys::File| {
        state.set(UploadState::Uploading(file.name()));
        spawn_local(async move {
            match create_and_upload(file).await {
                Ok(id) => {
                    state.set(UploadState::Done);
                    // Auto-advance to project page where transcription runs.
                    set_timeout(
                        move || {
                            if let Some(w) = web_sys::window() {
                                let _ = w.location().set_hash(&format!("/p/{id}"));
                            }
                        },
                        Duration::from_millis(600),
                    );
                }
                Err(e) => state.set(UploadState::Error(e)),
            }
        });
    };

    view! {
        <div class="hero">
            <div class="hero-badge">
                <span class="dot"></span>
                "STEP 1 · UPLOAD"
            </div>
            <h1>"Drop the footage."</h1>
            <p>
                "Pick the raw video. Once it's uploaded we'll transcribe it and surface the moments most likely to land as a Short."
            </p>
        </div>
        <div class="panel" style="max-width: 720px; margin: 0 auto;">
            <label
                class="dropzone"
                class:drag=move || dragging.get()
                on:dragover=move |ev| {
                    ev.prevent_default();
                    dragging.set(true);
                }
                on:dragleave=move |_| dragging.set(false)
                on:drop=move |ev| {
                    ev.prevent_default();
                    dragging.set(false);
                    if let Some(file) = dropped_file(&ev) {
                        handle_file(file);
                    }
                }
            >
                <div class="dropzone-icon">
                    <svg
                        width="28"
                        height="28"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path d="M12 16V4M12 4l-4 4M12 4l4 4"></path>
                        <path d="M4 16v2a2 2 0 002 2h12a2 2 0 002-2v-2"></path>
                    </svg>
                </div>
                <div class="dropzone-title">"Click to browse or drop a file"</div>
                <div class="dropzone-sub">"MP4, MOV, MKV — anything ffmpeg understands"</div>
                <input
                    type="file"
                    accept="video/*"
                    style="display:none"
                    on:change=move |ev| {
                        let input: web_sys::HtmlInputElement = event_target(&ev);
                        if let Some(file) = input.files().and_then(|l| l.get(0)) {
                            handle_file(file);
                        }
                    }
                />
            </label>
            <div class="mt-16">
                {move || match state.get() {
                    UploadState::Idle => ().into_any(),
                    UploadState::Uploading(name) => {
                        view! {
                            <div class="row">
                                <div class="spinner"></div>
                                {format!("Uploading {name}…")}
                            </div>
                        }
                            .into_any()
                    }
                    UploadState::Done => {
                        view! {
                            <div class="alert alert-info">
                                "✓ Uploaded. Transcribing now — this takes ~10% of the video length."
                            </div>
                        }
                            .into_any()
                    }
                    UploadState::Error(msg) => {
                        view! { <div class="alert alert-error">{format!("⚠ {msg}")}</div> }
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// app.js `handleFile`: create the project record, then multipart-upload the
/// footage into it. Returns the new project id for the redirect.
async fn create_and_upload(file: web_sys::File) -> Result<String, String> {
    let created: CreatedProject = post_json(
        "/api/projects",
        &CreateProjectBody {
            filename: file.name(),
        },
    )
    .await?;
    let id = created.id;
    upload_file(&format!("/api/projects/{id}/upload"), &file).await?;
    Ok(id)
}

/// Multipart POST with a `file` field — the one call shape http.rs doesn't
/// cover (mirrors app.js `api.upload`, including the `{error}` JSON surfacing).
async fn upload_file(url: &str, file: &web_sys::File) -> Result<(), String> {
    let form = web_sys::FormData::new().map_err(|_| "could not build upload form".to_string())?;
    form.append_with_blob("file", file)
        .map_err(|_| "could not attach file to upload form".to_string())?;
    let resp = gloo_net::http::Request::post(url)
        .body(form)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        let status_line = format!("{} {}", resp.status(), resp.status_text());
        let msg = resp
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_string))
            .unwrap_or(status_line);
        return Err(msg);
    }
    Ok(())
}

/// `e.dataTransfer.files[0]` from the drop event. The shared Cargo.toml (off
/// limits to page agents) doesn't enable the web-sys "DataTransfer" feature,
/// so reach through the untyped JS surface via Reflect — same runtime
/// behavior, no feature flag needed.
fn dropped_file(ev: &web_sys::DragEvent) -> Option<web_sys::File> {
    let ev_js: &JsValue = ev.as_ref();
    let dt = js_sys::Reflect::get(ev_js, &JsValue::from_str("dataTransfer")).ok()?;
    let files = js_sys::Reflect::get(&dt, &JsValue::from_str("files")).ok()?;
    let files: web_sys::FileList = files.dyn_into().ok()?;
    files.get(0)
}
