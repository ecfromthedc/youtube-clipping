//! Stage 2.1 — VISION MOMENT SELECTOR (Gemini). Mirrors src/ycp/vision.py.
//!
//! The transcript heuristic (clip::score_candidate) is blind to the footage; a video-native
//! model (Gemini 3.5 Flash) watches the video and picks the most clippable windows. Flag- and
//! key-gated: with `vision.enabled` false or no GEMINI_API_KEY, `rank_moments` returns [] and
//! the caller falls back to the transcript heuristic — exactly like Python.
//!
//! Parity note: `enabled` and `parse_moments` are ported byte-faithful (pure, unit-tested).
//! The live upload path uses the Google `genai` Files API (resumable upload → poll ACTIVE →
//! generateContent) — there is no native CLI for it, so the Rust binary documents it as the
//! ceiling and returns [] (heuristic fallback). This is the same call left by `reframe::face_track`
//! (no pure-Rust OpenCV): structurally identical to Python's disabled/key-absent path, which is
//! the path autopilot exercises without creds.
#![allow(dead_code)] // parse_moments/DEFAULT_MODEL are parity-carried (tests + documented ceiling)
use std::path::Path;

use crate::config;
use crate::util::round_to;

pub const DEFAULT_MODEL: &str = "gemini-3.5-flash";

/// A clippable window picked by the vision model (mirrors the `Moment` dataclass).
#[derive(Debug, Clone)]
pub struct Moment {
    pub start: f64,
    pub end: f64,
    pub score: f64,
    #[allow(dead_code)] // carried for parity; clip::run reads start/end/score
    pub reason: String,
}

fn cfg(settings: &serde_yaml::Value) -> &serde_yaml::Value {
    &settings["vision"]
}

fn api_key(root: &Path) -> Option<String> {
    config::env_var(root, "GEMINI_API_KEY")
}

pub fn vision_available(root: &Path) -> bool {
    api_key(root).is_some()
}

/// `vision.enabled` AND a key present (mirrors `enabled`).
pub fn enabled(root: &Path, settings: &serde_yaml::Value) -> bool {
    cfg(settings)["enabled"].as_bool().unwrap_or(false) && vision_available(root)
}

/// Parse the model's `moments` JSON array into ranked `Moment`s (mirrors `_parse_moments`):
/// skip entries missing/!numeric start/end, skip end<=start, clamp score to [0,1] (default 0.5),
/// round start/end to 2dp, truncate reason to 200 chars, sort by score desc. Pure.
pub fn parse_moments(raw: &serde_json::Value) -> Vec<Moment> {
    let mut out: Vec<Moment> = Vec::new();
    if let Some(arr) = raw.as_array() {
        for m in arr {
            let (s, e) = match (num_field(&m["start_sec"]), num_field(&m["end_sec"])) {
                (Some(s), Some(e)) => (s, e),
                _ => continue,
            };
            if e <= s {
                continue;
            }
            let score = num_field(&m["score"]).unwrap_or(0.5).clamp(0.0, 1.0);
            let reason: String = m["reason"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(200)
                .collect();
            out.push(Moment {
                start: round_to(s, 2),
                end: round_to(e, 2),
                score,
                reason,
            });
        }
    }
    // Stable sort by score descending (Python `sorted(..., reverse=True)` is stable).
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

/// float(x) tolerant of JSON number-or-numeric-string (mirrors Python `float(m["..."])`).
fn num_field(v: &serde_json::Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

/// Gemini picks the N most clippable windows. Returns [] when disabled/unavailable (caller
/// falls back to the transcript heuristic). See the module ceiling note: the live Files-API
/// upload path is not ported (no native client); [] keeps autopilot's no-creds path at parity.
pub fn rank_moments(
    root: &Path,
    _video: &Path,
    _n: usize,
    settings: &serde_yaml::Value,
) -> Vec<Moment> {
    if !enabled(root, settings) {
        return Vec::new();
    }
    // ponytail: live Gemini Files-API upload+poll not ported (no native client) — documented
    // ceiling, returns [] like Python's failure path. Upgrade: a reqwest port of the resumable
    // upload + generateContent flow if vision becomes a hard requirement on target machines.
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_sorts_clamps_and_filters() {
        let raw = json!([
            {"start_sec": 10, "end_sec": 30, "score": 0.4, "reason": "a"},
            {"start_sec": 5,  "end_sec": 5,  "score": 0.9},                 // end<=start → drop
            {"start_sec": 0,  "end_sec": 18, "score": 2.0, "reason": "b"},  // score clamps to 1.0
            {"start_sec": "x", "end_sec": 12, "score": 0.5},                // bad start → drop
            {"start_sec": 40, "end_sec": 60},                              // score default 0.5
        ]);
        let m = parse_moments(&raw);
        assert_eq!(m.len(), 3);
        assert_eq!(m[0].score, 1.0); // highest first
        assert_eq!((m[0].start, m[0].end), (0.0, 18.0));
        assert_eq!(m[1].score, 0.5);
        assert_eq!(m[2].score, 0.4);
    }

    #[test]
    fn parse_empty_on_non_array() {
        assert!(parse_moments(&json!({"moments": []})).is_empty());
    }
}
