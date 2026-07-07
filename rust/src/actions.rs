//! Copilot action map — single source of truth for what the Page Agent can do.
//!
//! The LLM proxy (server::llm_proxy_chat) prepends `system_prompt()` to every
//! copilot conversation, and the tests below assert that every control the
//! prompt claims exists is still present in the Leptos UI source. A UI change
//! that renames or removes a claimed control fails `cargo test` — the prompt
//! can no longer drift away from the real page (TILLER-LOOP-PROMPT.md P4).

pub struct Action {
    /// What the system prompt tells the agent it can do here.
    pub surface: &'static str,
    /// Literal string that must exist in the UI source for the claim to hold.
    #[allow(dead_code)] // read by the drift tests only
    pub marker: &'static str,
    /// File under rust/ui/src/ that must contain `marker`.
    #[allow(dead_code)] // read by the drift tests only
    pub file: &'static str,
}

pub const ACTIONS: &[Action] = &[
    Action {
        surface: "navigate via the topbar links: Projects (#/), Studio (#/studio), Analytics (#/analytics), Pipeline (#/pipeline)",
        marker: "topbar-nav",
        file: "main.rs",
    },
    Action {
        surface: "on Projects, click '+ New project' to start a project, then drop/pick a video to upload it",
        marker: "+ New project",
        file: "pages/dashboard.rs",
    },
    Action {
        surface: "on a project page, run 'Transcribe' to rank the best moments, then 'Render 9:16 clip' on a selected moment",
        marker: "Transcribe",
        file: "pages/project.rs",
    },
    Action {
        surface: "on a project page, use the '\u{1F3C6} Ranking compilation' sidebar section and click 'Compile ranking video' to compile the top moments",
        marker: "Compile ranking video",
        file: "pages/project.rs",
    },
    Action {
        surface: "on Studio, pick a format (Ranking / Storytelling / Commentary), fill the form, and click 'Render Short'",
        marker: "Render Short",
        file: "pages/studio_format.rs",
    },
    Action {
        surface: "on any rendered card, click the \u{1F4E4} button to publish to a Postiz YouTube channel",
        marker: "\u{1F4E4}",
        file: "pages/project.rs",
    },
    Action {
        surface: "on Analytics, read channel rollups and top posts ('Refresh (1h cache)' refetches)",
        marker: "Refresh (1h cache)",
        file: "pages/analytics.rs",
    },
    Action {
        surface: "on Analytics, click '\u{FF0B} Connect a channel' to link a YouTube channel via its Google login, and toggle between connected channels with the \u{1F4FA} chips",
        marker: "Connect a channel",
        file: "pages/analytics.rs",
    },
];

/// Build the copilot system prompt from the action map.
pub fn system_prompt() -> String {
    let mut s = String::from(
        "You are Tides Tiller Copilot, an in-page assistant inside an internal team video \
         editor. You can click, type, and navigate the page on the user's behalf. \
         Available actions:\n",
    );
    for (i, a) in ACTIONS.iter().enumerate() {
        s.push_str(&format!("{}. {}\n", i + 1, a.surface));
    }
    s.push_str(
        "Be terse — say what you're doing in one line, then do it. If the user asks for \
         something not on the page, say so.",
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// UI drift gate: every control the prompt claims must exist in the source.
    #[test]
    fn every_action_marker_exists_in_ui_source() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/src");
        for a in ACTIONS {
            let p = base.join(a.file);
            let src = std::fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("copilot drift check: read {}: {e}", p.display()));
            assert!(
                src.contains(a.marker),
                "copilot drift: {} no longer contains {:?} — the UI changed out from \
                 under the copilot prompt. Update rust/src/actions.rs to match reality.",
                a.file,
                a.marker
            );
        }
    }

    #[test]
    fn system_prompt_covers_every_action() {
        let sp = system_prompt();
        for a in ACTIONS {
            assert!(
                sp.contains(a.surface),
                "prompt missing surface: {}",
                a.surface
            );
        }
    }
}
