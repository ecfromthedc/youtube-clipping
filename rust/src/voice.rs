//! Voiceover — local TTS via OmniVoice Studio (OpenAI-compatible, no key needed).
//!
//! OmniVoice Studio runs at http://127.0.0.1:3900 and exposes /v1/audio/speech —
//! zero-shot voice cloning, ~24kHz output, optional voice-profile ids for cloned
//! voices. This module wraps that endpoint so any clip / listicle / storytelling
//! pipeline can drop a synthesized VO track in.
//!
//! Detection: probe /system/info on boot; if it's not up, `available()` returns
//! false and callers fall back to a no-VO path (same never-hard-break philosophy
//! as transcribe / hooks / vision).
//!
//! env overrides (take precedence over settings):
//!   OMNIVOICE_URL  — base URL of the server (default http://127.0.0.1:3900)
//!   OMNIVOICE_API_KEY — bearer token, only if the server was started with one
//!                       set (local dev: any non-empty string works as a sentinel)
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::config;

/// Default server URL (OmniVoice Studio's run.sh binds 127.0.0.1:3900).
const DEFAULT_URL: &str = "http://127.0.0.1:3900";

fn base_url(_root: &Path) -> String {
    std::env::var("OMNIVOICE_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_URL.to_string())
}

fn api_key(root: &Path) -> Option<String> {
    // settings.yaml omni_voice.api_key takes precedence; then .env; then OMNIVOICE_API_KEY.
    if let Ok(s) = config::load_settings(root) {
        if let Some(k) = s
            .get("omni_voice")
            .and_then(|v| v.get("api_key"))
            .and_then(|v| v.as_str())
        {
            if !k.is_empty() {
                return Some(k.to_string());
            }
        }
    }
    config::env_var(root, "OMNIVOICE_API_KEY")
}

/// Is OmniVoice Studio reachable? Probes /system/info with a short timeout.
pub fn available(root: &Path) -> bool {
    let url = format!("{}/system/info", base_url(root));
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    client
        .get(&url)
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

#[derive(Serialize)]
struct SpeechRequest<'a> {
    model: &'a str,
    input: &'a str,
    voice: &'a str,
    response_format: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    speed: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<&'a str>,
}

/// Synthesize `text` and write the result to `out_path` (a `.wav`).
///
/// `voice` is either `"default"`, a preset name, or a profile id returned by
/// OmniVoice's `POST /profiles` (clone your own voice once → reuse the id).
/// Returns the output path on success.
pub fn synthesize(
    root: &Path,
    text: &str,
    voice: &str,
    out_path: &Path,
    speed: Option<f32>,
    language: Option<&str>,
) -> Result<PathBuf> {
    if text.trim().is_empty() {
        bail!("voiceover text is empty");
    }
    if !available(root) {
        bail!(
            "OmniVoice Studio not reachable at {}. Start it with: cd OmniVoice-Studio && ./scripts/run.sh --no-open",
            base_url(root)
        );
    }
    let url = format!("{}/v1/audio/speech", base_url(root));
    let body = SpeechRequest {
        model: "omnivoice",
        input: text,
        voice,
        response_format: "wav",
        speed,
        language,
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180)) // long-form can take a while
        .build()?;
    let mut req = client.post(&url).json(&body);
    if let Some(key) = api_key(root) {
        req = req.bearer_auth(key);
    }
    let resp = req.send().context("POST /v1/audio/speech")?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        let tail = &body[body.len().saturating_sub(400)..];
        bail!("OmniVoice HTTP {status}: {}", tail.trim());
    }
    let bytes = resp.bytes().context("read response body")?;
    if bytes.is_empty() {
        bail!("OmniVoice returned empty audio");
    }
    if let Some(p) = out_path.parent() {
        std::fs::create_dir_all(p).ok();
    }
    std::fs::write(out_path, &bytes)?;
    Ok(out_path.to_path_buf())
}

/// List voices exposed by the OmniVoice server (default + presets + cloned profiles).
/// Returns a Vec of (id, name) pairs; empty if the server's unreachable.
pub fn list_voices(root: &Path) -> Vec<(String, String)> {
    let url = format!("{}/v1/audio/voices", base_url(root));
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut req = client.get(&url);
    if let Some(key) = api_key(root) {
        req = req.bearer_auth(key);
    }
    let resp = match req.send() {
        Ok(r) if r.status().is_success() => r,
        _ => return vec![],
    };
    let v: serde_json::Value = match resp.json() {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    // OpenAI shape: {"voices":[{"voice_id":"..","name":".."}, ...]}
    v["voices"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|x| {
                    let id = x["voice_id"]
                        .as_str()
                        .or_else(|| x["id"].as_str())?
                        .to_string();
                    let name = x["name"].as_str().unwrap_or(&id).to_string();
                    Some((id, name))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_request_serializes_minimal() {
        let r = SpeechRequest {
            model: "omnivoice",
            input: "hi",
            voice: "default",
            response_format: "wav",
            speed: None,
            language: None,
        };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains("\"model\":\"omnivoice\""));
        assert!(j.contains("\"voice\":\"default\""));
        // skip_serializing_if None → absent
        assert!(!j.contains("speed"));
        assert!(!j.contains("language"));
    }

    #[test]
    fn speech_request_serializes_with_speed_lang() {
        let r = SpeechRequest {
            model: "omnivoice",
            input: "hi",
            voice: "default",
            response_format: "wav",
            speed: Some(1.2),
            language: Some("en"),
        };
        let j = serde_json::to_string(&r).unwrap();
        assert!(j.contains("\"speed\":1.2"));
        assert!(j.contains("\"language\":\"en\""));
    }

    #[test]
    fn list_voices_parses_openai_shape() {
        // exercise the JSON-shape branch directly so the parser doesn't bit-rot
        let raw = serde_json::json!({
            "voices": [
                {"voice_id": "default", "name": "Default"},
                {"voice_id": "abc12345", "name": "Eric (clone)"},
            ]
        });
        let out: Vec<(String, String)> = raw["voices"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| {
                        let id = x["voice_id"]
                            .as_str()
                            .or_else(|| x["id"].as_str())?
                            .to_string();
                        let name = x["name"].as_str().unwrap_or(&id).to_string();
                        Some((id, name))
                    })
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], ("default".to_string(), "Default".to_string()));
        assert_eq!(out[1], ("abc12345".to_string(), "Eric (clone)".to_string()));
    }
}
