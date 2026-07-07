//! Self-serve channel connect — web OAuth + the connected-channels store.
//!
//! Any teammate opens the Tiller → "Connect a channel" → Google consent in
//! THEIR browser with THEIR login → their channel's refresh token lands in
//! `data/channels.json` and analytics flow per-channel. Scopes are the SAME
//! four the original yt_oauth.py flow used — the set that powered the verified
//! full analytics on the first channel (views + retention + MONETARY revenue):
//! youtube.force-ssl, youtube.readonly, yt-analytics.readonly,
//! yt-analytics-monetary.readonly.
//!
//! Secrets discipline: refresh tokens live ONLY in data/channels.json
//! (server-side, gitignored via data/*.json) and are never returned by any
//! API route. The web OAuth client id/secret come from .env
//! (YT_WEB_CLIENT_ID / YT_WEB_CLIENT_SECRET / YT_OAUTH_REDIRECT).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config;

pub const SCOPES: &str = "https://www.googleapis.com/auth/youtube.force-ssl \
https://www.googleapis.com/auth/youtube.readonly \
https://www.googleapis.com/auth/yt-analytics.readonly \
https://www.googleapis.com/auth/yt-analytics-monetary.readonly";

const DEFAULT_REDIRECT: &str = "https://tidestiller.risingtidesviral.com/api/oauth/yt/callback";
/// OAuth `state` tokens pending consent → issued-at epoch secs (CSRF guard).
static PENDING_STATES: Mutex<Vec<(String, u64)>> = Mutex::new(Vec::new());
const STATE_TTL_SECS: u64 = 600;

#[derive(Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub title: String,
    pub connected_at: String,
    /// Never serialized into API responses — see `public_list`.
    pub refresh_token: String,
}

fn store_path(root: &Path) -> PathBuf {
    config::data_dir(root).join("channels.json")
}

pub fn load(root: &Path) -> Vec<Channel> {
    std::fs::read_to_string(store_path(root))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(root: &Path, channels: &[Channel]) -> Result<()> {
    let path = store_path(root);
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(channels)?)?;
    Ok(())
}

/// Add or refresh a channel entry (reconnecting the same channel updates it).
pub fn upsert(root: &Path, id: &str, title: &str, refresh_token: &str) -> Result<()> {
    let mut all = load(root);
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    if let Some(c) = all.iter_mut().find(|c| c.id == id) {
        c.title = title.to_string();
        c.refresh_token = refresh_token.to_string();
        c.connected_at = now;
    } else {
        all.push(Channel {
            id: id.to_string(),
            title: title.to_string(),
            connected_at: now,
            refresh_token: refresh_token.to_string(),
        });
    }
    save(root, &all)
}

/// Token-free view for the API: [{id, title, connected_at}].
pub fn public_list(root: &Path) -> Value {
    let list: Vec<Value> = load(root)
        .iter()
        .map(|c| json!({ "id": c.id, "title": c.title, "connected_at": c.connected_at }))
        .collect();
    json!({ "channels": list })
}

/// The refresh token for one channel id, or the FIRST connected channel when
/// `id` is None (the store's default channel).
pub fn refresh_token_for(root: &Path, id: Option<&str>) -> Option<(String, String)> {
    let all = load(root);
    let c = match id {
        Some(want) => all.iter().find(|c| c.id == want)?,
        None => all.first()?,
    };
    Some((c.id.clone(), c.refresh_token.clone()))
}

// ── web OAuth flow ────────────────────────────────────────────────────────────

pub struct WebClient {
    pub client_id: String,
    pub client_secret: String,
    pub redirect: String,
}

/// Web OAuth client from .env — None until Eric drops YT_WEB_CLIENT_ID/SECRET in.
pub fn web_client(root: &Path) -> Option<WebClient> {
    Some(WebClient {
        client_id: config::env_var(root, "YT_WEB_CLIENT_ID")?,
        client_secret: config::env_var(root, "YT_WEB_CLIENT_SECRET")?,
        redirect: config::env_var(root, "YT_OAUTH_REDIRECT")
            .unwrap_or_else(|| DEFAULT_REDIRECT.to_string()),
    })
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Random state token, remembered for the callback to verify (CSRF).
pub fn issue_state() -> String {
    // ponytail: 16 bytes from the OS via getrandom-backed uuid; no extra dep.
    let state = uuid::Uuid::new_v4().simple().to_string();
    let mut pending = PENDING_STATES.lock().expect("state lock");
    let now = now_secs();
    pending.retain(|(_, t)| now.saturating_sub(*t) < STATE_TTL_SECS);
    pending.push((state.clone(), now));
    state
}

/// One-shot check: valid + unexpired state is consumed.
pub fn consume_state(state: &str) -> bool {
    let mut pending = PENDING_STATES.lock().expect("state lock");
    let now = now_secs();
    pending.retain(|(_, t)| now.saturating_sub(*t) < STATE_TTL_SECS);
    let before = pending.len();
    pending.retain(|(s, _)| s != state);
    pending.len() < before
}

/// The Google consent URL for the connect flow. `access_type=offline` +
/// `prompt=consent` guarantee a refresh token comes back (same as yt_oauth.py).
pub fn auth_url(client: &WebClient, state: &str) -> String {
    format!(
        "https://accounts.google.com/o/oauth2/auth?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&access_type=offline&prompt=consent",
        urlencode(&client.client_id),
        urlencode(&client.redirect),
        urlencode(SCOPES),
        urlencode(state),
    )
}

/// Exchange the callback code → (access_token, refresh_token). Blocking.
pub fn exchange_code(client: &WebClient, code: &str) -> Result<(String, String)> {
    let http = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let resp: Value = http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", code),
            ("client_id", client.client_id.as_str()),
            ("client_secret", client.client_secret.as_str()),
            ("redirect_uri", client.redirect.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .context("token exchange")?
        .json()
        .context("token exchange json")?;
    let access = resp
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            anyhow!(
                "no access_token: {}",
                resp.get("error_description")
                    .and_then(Value::as_str)
                    .unwrap_or("?")
            )
        })?;
    let refresh = resp
        .get("refresh_token")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("no refresh_token returned — re-run and fully consent"))?;
    Ok((access.to_string(), refresh.to_string()))
}

/// The authorized identity's own channel (id, title). Errors when the Google
/// account owns no channel — the exact failure the Phoenix re-auth hit, so the
/// message tells the user what to pick next time.
pub fn own_channel(access_token: &str) -> Result<(String, String)> {
    let http = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let resp: Value = http
        .get("https://www.googleapis.com/youtube/v3/channels?part=snippet&mine=true")
        .bearer_auth(access_token)
        .send()
        .context("channels.list")?
        .json()
        .context("channels.list json")?;
    let item = resp
        .get("items")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .ok_or_else(|| {
            anyhow!(
                "this Google identity owns no YouTube channel — on the consent screen pick the \
                 CHANNEL (brand account), not the bare email"
            )
        })?;
    let id = item.get("id").and_then(Value::as_str).unwrap_or_default();
    let title = item
        .pointer("/snippet/title")
        .and_then(Value::as_str)
        .unwrap_or("untitled channel");
    if id.is_empty() {
        bail!("channels.list returned no id");
    }
    Ok((id.to_string(), title.to_string()))
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scopes_cover_full_analytics_including_monetary() {
        for required in [
            "youtube.force-ssl",
            "youtube.readonly",
            "yt-analytics.readonly",
            "yt-analytics-monetary.readonly",
        ] {
            assert!(SCOPES.contains(required), "missing scope: {required}");
        }
    }

    #[test]
    fn state_is_single_use() {
        let s = issue_state();
        assert!(consume_state(&s));
        assert!(!consume_state(&s), "state must not be reusable");
        assert!(!consume_state("never-issued"));
    }

    #[test]
    fn auth_url_encodes_and_carries_offline_consent() {
        let c = WebClient {
            client_id: "id-123".into(),
            client_secret: "unused".into(),
            redirect: "https://example.com/cb".into(),
        };
        let url = auth_url(&c, "st4te");
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("prompt=consent"));
        assert!(url.contains("https%3A%2F%2Fexample.com%2Fcb"));
        assert!(url.contains("yt-analytics-monetary.readonly"));
        assert!(
            !url.contains("unused"),
            "client secret must never be in the URL"
        );
    }

    #[test]
    fn store_roundtrip_and_upsert() {
        let dir = std::env::temp_dir().join(format!("ycp-chan-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("data")).unwrap();
        upsert(&dir, "UC1", "First", "tok1").unwrap();
        upsert(&dir, "UC2", "Second", "tok2").unwrap();
        upsert(&dir, "UC1", "First Renamed", "tok1b").unwrap(); // reconnect updates
        let all = load(&dir);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].title, "First Renamed");
        assert_eq!(all[0].refresh_token, "tok1b");
        // default = first connected; explicit id resolves
        assert_eq!(refresh_token_for(&dir, None).unwrap().0, "UC1");
        assert_eq!(refresh_token_for(&dir, Some("UC2")).unwrap().1, "tok2");
        // the public view never leaks tokens
        let public = public_list(&dir).to_string();
        assert!(!public.contains("tok1b") && !public.contains("tok2"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
