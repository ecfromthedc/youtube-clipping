//! Studio — format picker (port of `studioPage` in rust/web/app.js).
//!
//! Route: `#/studio`
//! Data: `GET /api/voices` — `{available, voices:[{id,name}]}`; the picker only
//! needs `available` (banner when OmniVoice isn't reachable). Fetch failure is
//! treated as unavailable, mirroring app.js `.catch(() => ({available:false}))`.

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

use crate::http::get_json;

/// The slice of `GET /api/voices` this page consumes.
#[derive(Deserialize, Default)]
struct VoicesResp {
    #[serde(default)]
    available: bool,
}

/// One card in the format grid — mirrors the `FORMATS` array in app.js
/// (the `slug` field is only used by the per-format page, so it's omitted here).
struct Format {
    name: &'static str,
    icon: &'static str,
    blurb: &'static str,
    difficulty: &'static str,
    href: &'static str,
    cta: &'static str,
    note: &'static str,
}

const FORMATS: [Format; 3] = [
    Format {
        name: "Ranking Compilation",
        icon: "🏆",
        blurb: "Top-N ranked clips, big rank numbers on the left edge, countdown reveal (best plays last). The highest-volume format.",
        difficulty: "Easy",
        href: "#/p/",
        cta: "Open a project →",
        note: "Lives inside each project (after upload + transcribe)",
    },
    Format {
        name: "Storytelling / Roblox Rants",
        icon: "📖",
        blurb: "Write a script → AI voiceover → looping gameplay background → opus captions → 9:16. Generates original content from words.",
        difficulty: "Easy",
        href: "#/studio/storytelling",
        cta: "Write a script",
        note: "Needs OmniVoice + a background clip",
    },
    Format {
        name: "Commentary / Reaction",
        icon: "🎬",
        blurb: "Paste a viral clip → write commentary → AI VO over ducked original audio + captions. His highest-RPM niche (35-40¢/1k).",
        difficulty: "Medium",
        href: "#/studio/commentary",
        cta: "React to a clip",
        note: "Needs OmniVoice + a source clip",
    },
];

#[component]
pub fn Studio() -> impl IntoView {
    // OmniVoice reachability: None = still checking (no banner yet — app.js
    // renders nothing until the fetch resolves), Some(false) = show the banner.
    let vo_available = RwSignal::new(None::<bool>);
    spawn_local(async move {
        let vo = get_json::<VoicesResp>("/api/voices")
            .await
            .unwrap_or_default();
        vo_available.set(Some(vo.available));
    });

    view! {
        <div class="page-header">
            <div>
                <h1 class="page-title">"Studio"</h1>
                <p class="page-sub">
                    "Three formats, one engine. Each is an end-to-end play from the playbook — pick one and ship a Short."
                </p>
            </div>
        </div>
        {move || {
            (vo_available.get() == Some(false))
                .then(|| {
                    view! {
                        <div class="alert alert-warn mb-24">
                            "⚠ OmniVoice Studio isn't reachable at localhost:3900. Storytelling + Commentary need it for voiceover. "
                            <a href="#/pipeline">"Start it →"</a>
                        </div>
                    }
                })
        }}
        <div class="format-grid">
            {FORMATS
                .iter()
                .map(|f| {
                    let pill_class = if f.difficulty == "Easy" {
                        "pill"
                    } else {
                        "pill pill-warn"
                    };
                    view! {
                        <a class="format-card" href=f.href>
                            <div class="format-icon">{f.icon}</div>
                            <div class="format-name">{f.name}</div>
                            <div class="format-blurb">{f.blurb}</div>
                            <div class="format-meta">
                                <span class=pill_class>{f.difficulty}</span>
                                <span class="muted" style="font-size:11px;">{f.note}</span>
                            </div>
                            // app.js appends " →" to every cta (ranking's cta already
                            // ends in "→", so it doubles there too — ported verbatim).
                            <div class="format-cta">{f.cta}" →"</div>
                        </a>
                    }
                })
                .collect_view()}
        </div>
    }
}
