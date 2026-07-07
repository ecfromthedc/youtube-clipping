//! Tides Tiller Copilot — the in-page Page Agent, hardened (P4).
//!
//! The Page Agent bundle is VENDORED (rust/ui/vendor/, pinned 1.11.0, loaded
//! by index.html and exposed as window.PageAgent) — no CDN at runtime. This
//! component owns the trigger bead + a real task panel (no window.prompt) and
//! fails LOUD: init/execute errors render in the panel, never console-only.

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::RefCell;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

thread_local! {
    static AGENT: RefCell<Option<JsValue>> = const { RefCell::new(None) };
}

fn js_err(e: JsValue) -> String {
    e.as_string().unwrap_or_else(|| {
        js_sys::Reflect::get(&e, &"message".into())
            .ok()
            .and_then(|m| m.as_string())
            .unwrap_or_else(|| format!("{e:?}"))
    })
}

/// Lazily construct (and cache) the PageAgent instance against our LLM proxy.
fn agent() -> Result<JsValue, String> {
    if let Some(a) = AGENT.with(|c| c.borrow().clone()) {
        return Ok(a);
    }
    let window = web_sys::window().ok_or("no window")?;
    let ctor: js_sys::Function = js_sys::Reflect::get(&window, &"PageAgent".into())
        .map_err(js_err)?
        .dyn_into()
        .map_err(|_| {
            "Page Agent bundle not loaded (window.PageAgent missing) — check \
             vendor/page-agent-1.11.0.bundle.mjs shipped with the build"
                .to_string()
        })?;
    let origin = window.location().origin().map_err(js_err)?;
    let cfg = js_sys::Object::new();
    let set = |k: &str, v: JsValue| {
        let _ = js_sys::Reflect::set(&cfg, &JsValue::from_str(k), &v);
    };
    set("model", "deepseek-chat".into());
    set("baseURL", format!("{origin}/api/llm/proxy").into());
    set("apiKey", "server-side".into()); // proxy injects the real key
    set("language", "en-US".into());
    set("promptForNextTask", JsValue::TRUE);
    let inst =
        js_sys::Reflect::construct(&ctor, &js_sys::Array::of1(&cfg)).map_err(js_err)?;
    AGENT.with(|c| *c.borrow_mut() = Some(inst.clone()));
    Ok(inst)
}

async fn run_task(task: String) -> Result<(), String> {
    let inst = agent()?;
    let exec: js_sys::Function = js_sys::Reflect::get(&inst, &"execute".into())
        .map_err(js_err)?
        .dyn_into()
        .map_err(|_| "PageAgent.execute is not a function".to_string())?;
    let ret = exec.call1(&inst, &task.into()).map_err(js_err)?;
    let promise: js_sys::Promise = ret
        .dyn_into()
        .map_err(|_| "PageAgent.execute did not return a Promise".to_string())?;
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map(|_| ())
        .map_err(js_err)
}

#[component]
pub fn Copilot() -> impl IntoView {
    let open = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let err = RwSignal::new(String::new());
    let task = RwSignal::new(String::new());

    let submit = move || {
        let t = task.get().trim().to_string();
        if t.is_empty() || busy.get() {
            return;
        }
        busy.set(true);
        err.set(String::new());
        spawn_local(async move {
            match run_task(t).await {
                Ok(()) => {
                    busy.set(false);
                    task.set(String::new());
                    open.set(false); // the agent's own panel has taken over
                }
                Err(e) => {
                    busy.set(false);
                    err.set(e); // LOUD: shown in the panel, not console-only
                }
            }
        });
    };

    // Ctrl+/ (Cmd+/) toggles the panel — same shortcut as the old app.
    window_event_listener(leptos::ev::keydown, move |e| {
        if (e.ctrl_key() || e.meta_key()) && e.key() == "/" {
            e.prevent_default();
            open.update(|o| *o = !*o);
        }
    });

    view! {
        <button
            class="pa-trigger"
            type="button"
            aria-label="Open Tides Tiller Copilot"
            title="Tides Tiller Copilot (Ctrl+/)"
            on:click=move |_| open.update(|o| *o = !*o)
        >
            <svg
                viewBox="0 0 24 24"
                width="22"
                height="22"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
            >
                <path d="M12 2L9.5 8.5L3 11l6.5 2.5L12 20l2.5-6.5L21 11l-6.5-2.5L12 2z" />
            </svg>
        </button>
        <Show when=move || open.get()>
            <div class="pa-panel">
                <div class="pa-panel-title">"🤖 Tides Tiller Copilot"</div>
                <input
                    class="pa-input"
                    type="text"
                    placeholder="e.g. go to Studio and render a storytelling short"
                    prop:value=move || task.get()
                    on:input=move |e| task.set(event_target_value(&e))
                    on:keydown=move |e| {
                        if e.key() == "Enter" {
                            submit();
                        }
                    }
                />
                <div class="row" style="margin-top:10px; justify-content: flex-end; gap: 8px;">
                    <button
                        class="btn btn-ghost"
                        on:click=move |_| {
                            open.set(false);
                            err.set(String::new());
                        }
                    >
                        "Close"
                    </button>
                    <button
                        class="btn btn-primary"
                        disabled=move || busy.get()
                        on:click=move |_| submit()
                    >
                        {move || if busy.get() { "Working…" } else { "Run" }}
                    </button>
                </div>
                <Show when=move || !err.get().is_empty()>
                    <div class="alert alert-error" style="margin-top:10px;">
                        "⚠ "
                        {move || err.get()}
                    </div>
                </Show>
            </div>
        </Show>
    }
}
