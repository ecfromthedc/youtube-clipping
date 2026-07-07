//! Tides Tiller — Leptos CSR shell (port of rust/web/app.js).
//!
//! Hash routing on purpose: the old vanilla SPA is hash-routed (#/studio …), so
//! keeping the same scheme makes every href a class-for-class port AND lets the
//! app mount anywhere (/next during the side-by-side phase, / after cutover)
//! with zero router base-path plumbing.
//!
//! SHARED-FILE RULE (page agents): this file, http.rs, pages/mod.rs, Cargo.toml,
//! Trunk.toml, index.html, and styles.css are off-limits to page agents. Pages
//! own only their pages/<name>.rs. Gaps get reported, not hand-patched here.

mod http;
mod pages;

use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app);
}

/// Current route from location.hash, "" → "/" (mirrors app.js route()).
fn current_route() -> String {
    let hash = web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .unwrap_or_default();
    let r = hash.trim_start_matches('#');
    if r.is_empty() {
        "/".to_string()
    } else {
        r.to_string()
    }
}

fn app() -> impl IntoView {
    let route = RwSignal::new(current_route());
    window_event_listener(leptos::ev::hashchange, move |_| route.set(current_route()));

    view! {
        <Topbar route=route />
        <main class="page">{move || page_for(route.get())}</main>
    }
}

/// Route → page component (mirrors the dispatch chain in app.js route()).
fn page_for(route: String) -> AnyView {
    match route.as_str() {
        "/" => view! { <pages::dashboard::Dashboard /> }.into_any(),
        "/pipeline" => view! { <pages::pipeline::Pipeline /> }.into_any(),
        "/studio" => view! { <pages::studio::Studio /> }.into_any(),
        "/analytics" => view! { <pages::analytics::Analytics /> }.into_any(),
        r if r.starts_with("/studio/") => {
            let slug = r["/studio/".len()..].to_string();
            view! { <pages::studio_format::StudioFormat slug=slug /> }.into_any()
        }
        r if r.starts_with("/p/") => {
            let id = r["/p/".len()..].to_string();
            view! { <pages::project::ProjectPage id=id /> }.into_any()
        }
        r if r.starts_with("/new") => view! { <pages::new_project::NewProject /> }.into_any(),
        _ => view! { <div class="empty">"Not found."</div> }.into_any(),
    }
}

#[component]
fn Topbar(route: RwSignal<String>) -> impl IntoView {
    let nav = move |label: &'static str, href: &'static str| {
        let class = move || {
            if route.get() == href {
                "nav-link active"
            } else {
                "nav-link"
            }
        };
        view! { <a class=class href=format!("#{href}")>{label}</a> }
    };
    view! {
        <header class="topbar">
            <a class="brand" href="#/">
                <span class="brand-mark clay">
                    <img src="/static/logo.svg" alt="Tides Tiller" class="brand-mark-img" />
                </span>
                <span class="brand-name">
                    <span>"Tides"</span>
                    <span class="amp">"·"</span>
                    <span>"Tiller"</span>
                </span>
            </a>
            <nav class="topbar-nav">
                {nav("Projects", "/")}
                {nav("Studio", "/studio")}
                {nav("Analytics", "/analytics")}
                {nav("Pipeline", "/pipeline")}
                <span class="pill live">
                    <span class="dot"></span>
                    "online"
                </span>
            </nav>
        </header>
    }
}
