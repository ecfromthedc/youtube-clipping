//! Studio format page — port of `studioFormatPage` in rust/web/app.js (~791-909).
//!
//! Storytelling / Commentary: script → OmniVoice VO → render a Short. Class
//! names, visible text, and API behavior mirror the vanilla page 1:1
//! (styles.css is the parity contract). Ranking (and unknown slugs) show the
//! same "Unknown format." empty state as the JS page.

use crate::http::{get_json, post_json};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// GET /api/voices — `{available, voices: [{id, name}]}`.
#[derive(Clone, Default, Deserialize)]
struct VoicesResp {
    #[serde(default)]
    available: bool,
    #[serde(default)]
    voices: Vec<Voice>,
}

#[derive(Clone, Deserialize)]
struct Voice {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
}

/// POST /api/studio/render → `{path, project}` (the page only uses `path`).
#[derive(Deserialize)]
struct RenderOut {
    #[serde(default)]
    path: String,
}

/// One entry in the status area under the render button. The vanilla page
/// *appends* validation warnings without clearing, so this is a list.
#[derive(Clone)]
enum StatusMsg {
    Warn(String),
    Busy(String),
    /// Download path of the rendered MP4.
    Done(String),
    Error(String),
}

/// Mirrors app.js `parseFloat(x) || null` / `|| 0.25`: NaN and 0 are falsy.
fn parse_float_truthy(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok().filter(|v| *v != 0.0)
}

/// `setTimeout(() => location.reload(), ms)` — used by the download link so the
/// dashboard picks up the new render after the download starts.
fn reload_after(ms: i32) {
    let cb = Closure::once_into_js(|| {
        if let Some(w) = web_sys::window() {
            let _ = w.location().reload();
        }
    });
    if let Some(w) = web_sys::window() {
        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.unchecked_ref::<js_sys::Function>(),
            ms,
        );
    }
}

#[component]
pub fn StudioFormat(slug: String) -> impl IntoView {
    let is_story = slug == "storytelling";
    let is_commentary = slug == "commentary";
    if !is_story && !is_commentary {
        return view! { <div class="empty">"Unknown format."</div> }.into_any();
    }

    // The Rust enum tags on "story" (the variant name lowercased); map from URL slug.
    let format_tag = if is_story { "story" } else { "commentary" };
    let (icon, name, blurb) = if is_story {
        (
            "📖",
            "Storytelling / Roblox Rants",
            "Write a script → AI voiceover → looping gameplay background → opus captions → 9:16. Generates original content from words.",
        )
    } else {
        (
            "🎬",
            "Commentary / Reaction",
            "Paste a viral clip → write commentary → AI VO over ducked original audio + captions. His highest-RPM niche (35-40¢/1k).",
        )
    };
    let script_placeholder = if is_story {
        "Write the story / hot take the VO will read. e.g. 'This is the most ridiculous thing that happened at school today...'"
    } else {
        "Write the commentary the VO speaks over the clip. e.g. 'Okay watch what this guy does next — this is actually insane...'"
    };
    let (source_label, source_placeholder) = if is_story {
        (
            "Background footage (gameplay/Minecraft) — URL or local path",
            "https://www.youtube.com/watch?v=... or /path/to/subway_surfer.mp4",
        )
    } else {
        (
            "Source clip — URL (YT/TT/IG) or local path",
            "https://www.tiktok.com/@.../video/... or /path/to/clip.mp4",
        )
    };
    let busy_msg = if is_story {
        "Synthesizing VO → transcribing → compositing gameplay… (60-120s)"
    } else {
        "Fetching source → VO → captions → mix… (60-120s)"
    };

    // None = voices still loading (the JS page awaits the fetch before it
    // builds the form panel — header shows immediately, panel after).
    let voices = RwSignal::new(None::<VoicesResp>);
    spawn_local(async move {
        // Mirrors `.catch(() => ({ available: false, voices: [] }))`.
        let vo = get_json::<VoicesResp>("/api/voices")
            .await
            .unwrap_or_default();
        voices.set(Some(vo));
    });

    let source = RwSignal::new(String::new());
    let script = RwSignal::new(String::new());
    let title = RwSignal::new(String::new());
    let voice = RwSignal::new("default".to_string());
    let speed = RwSignal::new("1.0".to_string());
    let language = RwSignal::new(String::new());
    let duck = RwSignal::new("0.25".to_string());
    // true while a render is in flight — and stays true after success (the JS
    // button is only re-enabled on error; success path reloads via the link).
    let rendering = RwSignal::new(false);
    let status = RwSignal::new(Vec::<StatusMsg>::new());

    let do_render = move || {
        let script_v = script.get_untracked();
        if script_v.trim().is_empty() {
            status.update(|s| s.push(StatusMsg::Warn("Write the script first.".to_string())));
            return;
        }
        let source_v = source.get_untracked();
        if source_v.trim().is_empty() {
            let what = if is_story {
                "background"
            } else {
                "source clip"
            };
            status.update(|s| s.push(StatusMsg::Warn(format!("Add the {what}."))));
            return;
        }
        rendering.set(true);
        status.set(vec![StatusMsg::Busy(busy_msg.to_string())]);

        let title_v = title.get_untracked();
        let language_v = language.get_untracked();
        let mut body = serde_json::json!({
            "format": format_tag,
            "script": script_v,
            "voice": voice.get_untracked(),
            "title": if title_v.is_empty() { None } else { Some(title_v) },
            "speed": parse_float_truthy(&speed.get_untracked()),
            "language": if language_v.is_empty() { None } else { Some(language_v) },
        });
        body[if is_story { "background" } else { "source" }] = serde_json::Value::String(source_v);
        if is_commentary {
            let duck_v = parse_float_truthy(&duck.get_untracked()).unwrap_or(0.25);
            body["duck_volume"] = serde_json::json!(duck_v);
        }
        spawn_local(async move {
            match post_json::<serde_json::Value, RenderOut>("/api/studio/render", &body).await {
                Ok(out) => status.set(vec![StatusMsg::Done(out.path)]),
                Err(err) => {
                    rendering.set(false);
                    status.set(vec![StatusMsg::Error(err)]);
                }
            }
        });
    };

    let status_view = move || {
        status
            .get()
            .into_iter()
            .map(|item| match item {
                StatusMsg::Warn(m) => view! { <div class="alert alert-warn">{m}</div> }.into_any(),
                StatusMsg::Busy(m) => view! {
                    <div class="row">
                        <div class="spinner"></div>
                        {m}
                    </div>
                }
                .into_any(),
                StatusMsg::Done(path) => view! {
                    <div class="alert alert-info">
                        "✓ Rendered. "
                        <a
                            class="btn btn-primary btn-sm"
                            href=path
                            download=""
                            on:click=move |_| reload_after(800)
                        >
                            "↓ Download MP4"
                        </a>
                    </div>
                }
                .into_any(),
                StatusMsg::Error(m) => {
                    view! { <div class="alert alert-error">{format!("⚠ {m}")}</div> }.into_any()
                }
            })
            .collect_view()
    };

    view! {
        <div class="page-header">
            <div>
                <a class="muted" href="#/studio" style="font-size:13px;">
                    "← Studio"
                </a>
                <h1 class="page-title mt-8">{format!("{icon} {name}")}</h1>
                <p class="page-sub">{blurb}</p>
            </div>
        </div>
        {move || match voices.get() {
            None => ().into_any(),
            Some(vo) => {
                let available = vo.available;
                let voice_options = vo
                    .voices
                    .iter()
                    .map(|v| {
                        let label = format!("{} ({})", v.name, v.id);
                        view! { <option value=v.id.clone()>{label}</option> }
                    })
                    .collect_view();
                view! {
                    <div class="panel" style="max-width: 760px;">
                        <div class="field">
                            <label>{source_label}</label>
                            <input
                                class="input"
                                placeholder=source_placeholder
                                on:input=move |ev| source.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="field">
                            <label>"Script (will be spoken by the VO)"</label>
                            <textarea
                                class="textarea"
                                style="min-height: 160px;"
                                placeholder=script_placeholder
                                on:input=move |ev| script.set(event_target_value(&ev))
                            ></textarea>
                        </div>
                        <div class="field">
                            <label>"Hook title"</label>
                            <input
                                class="input"
                                placeholder="Hook title (top of frame, optional)"
                                on:input=move |ev| title.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="field-row">
                            <div class="field">
                                <label>"Voice"</label>
                                <select
                                    class="select"
                                    disabled=!available
                                    on:change=move |ev| voice.set(event_target_value(&ev))
                                >
                                    <option value="default">"Default voice"</option>
                                    {voice_options}
                                </select>
                            </div>
                            <div class="field">
                                <label>"Speed"</label>
                                <input
                                    class="input"
                                    type="number"
                                    step="0.05"
                                    min="0.5"
                                    max="2.0"
                                    value="1.0"
                                    placeholder="1.0"
                                    on:input=move |ev| speed.set(event_target_value(&ev))
                                />
                            </div>
                            <div class="field">
                                <label>"Language"</label>
                                <input
                                    class="input"
                                    placeholder="en (optional)"
                                    on:input=move |ev| language.set(event_target_value(&ev))
                                />
                            </div>
                        </div>
                        {is_commentary
                            .then(|| {
                                view! {
                                    <div class="field">
                                        <label>"Original clip duck volume (0-1)"</label>
                                        <input
                                            class="input"
                                            type="number"
                                            step="0.05"
                                            min="0"
                                            max="1"
                                            value="0.25"
                                            on:input=move |ev| duck.set(event_target_value(&ev))
                                        />
                                    </div>
                                }
                            })}
                        <button
                            class="btn btn-primary"
                            style="width: 100%;"
                            disabled=move || !available || rendering.get()
                            on:click=move |_| do_render()
                        >
                            {if available { "Render Short" } else { "OmniVoice offline — start it first" }}
                        </button>
                        <div class="mt-16">{status_view}</div>
                    </div>
                }
                    .into_any()
            }
        }}
    }
    .into_any()
}
