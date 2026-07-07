//! Pipeline page (about) — port of `pipelinePage` in rust/web/app.js.
//!
//! Fully static: a page header plus a grid of six panel cards describing each
//! step of the clipping chain. No API calls, no state — exact same class
//! names, element structure, and visible text as the vanilla-JS original.

use leptos::prelude::*;

/// (step number, title, body) — mirrors the `steps` array in app.js verbatim.
const STEPS: [(&str, &str, &str); 6] = [
    (
        "01",
        "Upload",
        "Source video lands in data/editor/<id>/source.mp4. ffprobe reports duration.",
    ),
    (
        "02",
        "Transcribe",
        "whisper.cpp (or openai-whisper fallback) → word-level transcript. Pure-Rust SRT parse.",
    ),
    (
        "03",
        "Plan clips",
        "clip::plan_clips groups segments into 15–38s windows, scores each by hook strength.",
    ),
    (
        "04",
        "Edit",
        "Pick a window on the timeline. Drag start/end. Set the hook title burned in at top.",
    ),
    (
        "05",
        "Render",
        "ffmpeg trims → reframe 9:16 → ab_glyph burns opus-style word-by-word captions.",
    ),
    (
        "06",
        "Ship",
        "MP4 lands in data/editor/<id>/renders/. Download or push to a channel.",
    ),
];

#[component]
pub fn Pipeline() -> impl IntoView {
    view! {
        <div class="page-header">
            <div>
                <h1 class="page-title">"Pipeline"</h1>
                <p class="page-sub">
                    "Every step in the chain is a Rust module that already exists in the clipping engine. The editor just orchestrates them for interactive use."
                </p>
            </div>
        </div>
        <div class="proj-grid">
            {STEPS
                .iter()
                .map(|(n, title, body)| {
                    view! {
                        <div class="panel">
                            <div class="row">
                                <span class="pill">{*n}</span>
                                <strong>{*title}</strong>
                            </div>
                            <p class="muted mt-8">{*body}</p>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}
