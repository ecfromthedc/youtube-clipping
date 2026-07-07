//! Agent-facing format registry — served at GET /api/formats.
//!
//! The machine-readable "how do I render things here" manifest, so ANY agent
//! (the autopilot, a Claude session, the in-page copilot, ocean agents) can
//! run every template/format headlessly without reading source. Same doctrine
//! as actions.rs: single source of truth + drift tests — if a route or param
//! is renamed in server.rs, `cargo test` fails here before an agent 404s.

use serde_json::{json, Value};

/// Route strings the manifest advertises — also what the drift test greps for.
pub const EP_PROJECTS: &str = "/api/projects";
pub const EP_RENDER: &str = "/api/projects/:id/render";
pub const EP_COMPILE: &str = "/api/projects/:id/compile";
pub const EP_STUDIO: &str = "/api/studio/render";
pub const EP_VOICES: &str = "/api/voices";
pub const EP_PUBLISH: &str = "/api/postiz/publish";

pub fn formats_manifest() -> Value {
    json!({
        "service": "tides-tiller",
        "formats": [
            {
                "format": "clip",
                "description": "Single 9:16 captioned clip cut from an uploaded video's best moment. Requires a transcribed project (GET /api/projects/:id → candidates[] has ranked {start,end,score,text} windows).",
                "method": "POST",
                "endpoint": EP_RENDER,
                "params": {
                    "start": { "type": "number", "required": true, "unit": "seconds" },
                    "end":   { "type": "number", "required": true, "unit": "seconds" },
                    "title": { "type": "string", "required": false, "note": "hook title burned at the top of the frame" }
                },
                "example": { "start": 12.4, "end": 41.0, "title": "he wasn't ready for this" }
            },
            {
                "format": "ranking",
                "description": "Ranking/listicle compilation — N ranked moments concatenated with big rank badges, countdown or countup reveal. The IG-reel countdown format.",
                "method": "POST",
                "endpoint": EP_COMPILE,
                "params": {
                    "items": { "type": "array", "required": true, "items": { "start": "number (s)", "end": "number (s)", "label": "string?" } },
                    "title": { "type": "string", "required": false },
                    "order": { "type": "string", "required": false, "enum": ["countdown", "countup"], "default": "countdown", "note": "countup saves the best for last (reference-reel reveal)" }
                },
                "example": { "items": [ { "start": 10.0, "end": 32.0, "label": "the comeback" }, { "start": 95.5, "end": 118.0 } ], "title": "top 5 moments", "order": "countup" }
            },
            {
                "format": "story",
                "description": "Storytelling short — script → OmniVoice VO → looping background footage (gameplay etc.) → captions → 9:16. Background may be a URL (resolved via yt-dlp) or a local path.",
                "method": "POST",
                "endpoint": EP_STUDIO,
                "params": {
                    "format":     { "type": "string", "required": true, "const": "story" },
                    "script":     { "type": "string", "required": true },
                    "background": { "type": "string", "required": true, "note": "URL or local path to background footage" },
                    "voice":      { "type": "string", "required": false, "note": "from GET /api/voices" },
                    "title":      { "type": "string", "required": false },
                    "speed":      { "type": "number", "required": false },
                    "language":   { "type": "string", "required": false },
                    "project":    { "type": "string", "required": false, "note": "project id to file the output under" }
                },
                "example": { "format": "story", "script": "a teacher changed everything for one kid…", "background": "data/backgrounds/minecraft.mp4", "title": "this teacher got the last laugh" }
            },
            {
                "format": "commentary",
                "description": "Commentary short — source clip + commentary script VO over ducked original audio + captions. Source may be a URL or local path.",
                "method": "POST",
                "endpoint": EP_STUDIO,
                "params": {
                    "format":      { "type": "string", "required": true, "const": "commentary" },
                    "source":      { "type": "string", "required": true, "note": "URL or local path to the source clip" },
                    "script":      { "type": "string", "required": true },
                    "voice":       { "type": "string", "required": false },
                    "title":       { "type": "string", "required": false },
                    "speed":       { "type": "number", "required": false },
                    "language":    { "type": "string", "required": false },
                    "duck_volume": { "type": "number", "required": false, "default": 0.25, "note": "original-audio volume under the VO" },
                    "project":     { "type": "string", "required": false }
                },
                "example": { "format": "commentary", "source": "data/editor/abc123/source.mp4", "script": "watch what happens at the 20 second mark…", "duck_volume": 0.2 }
            }
        ],
        "workflow": {
            "note": "clip + ranking need a project with an uploaded, transcribed video; story + commentary are standalone.",
            "steps": [
                { "step": "create",     "call": format!("POST {EP_PROJECTS} {{\"filename\": \"raw.mp4\"}} → {{id}}") },
                { "step": "upload",     "call": "POST /api/projects/:id/upload (multipart form field 'file')" },
                { "step": "transcribe", "call": "POST /api/projects/:id/transcribe with Content-Type: application/json and body {} → ranked candidates[]" },
                { "step": "inspect",    "call": "GET /api/projects/:id → {candidates, renders, compiles, stories, commentary}" },
                { "step": "render",     "call": "one of the formats above → {path} (download via the returned /api/projects/:id/files/... path)" },
                { "step": "voices",     "call": format!("GET {EP_VOICES} → OmniVoice availability + voice list (story/commentary need the local Studio server)") }
            ],
            "concurrency": "studio renders serialize server-side (one OmniVoice call at a time) — fire-and-wait, don't parallelize studio calls"
        },
        "guardrails": [
            format!("PUBLISH ({EP_PUBLISH}): SHARED Postiz account — only ever the mapped integration id for the active owned channel; NEVER act on posts by state"),
            "no copyrighted music beds (instant Content-ID); transform every owned-channel clip; clips ≤38s, vision targets 20-35s",
            "channel health > raw output — selective A/B (single top hero moment), not volume"
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Drift gate: every endpoint the manifest advertises must exist in the
    /// server router, and every param name in the handlers' body types.
    #[test]
    fn manifest_endpoints_and_params_exist_in_server_source() {
        let src =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/server.rs"))
                .expect("read server.rs");
        for ep in [
            EP_PROJECTS,
            EP_RENDER,
            EP_COMPILE,
            EP_STUDIO,
            EP_VOICES,
            EP_PUBLISH,
        ] {
            assert!(
                src.contains(&format!("\"{ep}\"")),
                "formats drift: route {ep} not found in server.rs — update formats.rs"
            );
        }
        for param in [
            "start",
            "end",
            "title",
            "items",
            "order",
            "script",
            "background",
            "voice",
            "speed",
            "language",
            "duck_volume",
            "source",
            "project",
        ] {
            assert!(
                src.contains(param),
                "formats drift: param {param:?} not found in server.rs body types"
            );
        }
    }

    #[test]
    fn manifest_serializes_with_all_formats() {
        let m = formats_manifest();
        let names: Vec<&str> = m["formats"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["format"].as_str().unwrap())
            .collect();
        assert_eq!(names, vec!["clip", "ranking", "story", "commentary"]);
    }
}
