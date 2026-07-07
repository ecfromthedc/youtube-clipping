//! Shared fetch helpers — the one utility page agents share (mirrors the `api`
//! object in app.js: same-origin /api/* calls, `{error}` JSON surfaced as the
//! error message).
#![allow(dead_code)] // consumed by pages as they land (P1+)

use gloo_net::http::Request;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Pull a human message out of an error response: prefer JSON `{"error": …}`,
/// fall back to the status line (mirrors app.js `api` error handling).
async fn error_message(resp: gloo_net::http::Response) -> String {
    let status = format!("{} {}", resp.status(), resp.status_text());
    match resp.json::<serde_json::Value>().await {
        Ok(v) => v
            .get("error")
            .and_then(|e| e.as_str())
            .map(str::to_string)
            .unwrap_or(status),
        Err(_) => status,
    }
}

pub async fn get_json<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let resp = Request::get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(error_message(resp).await);
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

pub async fn post_json<B: Serialize, T: DeserializeOwned>(url: &str, body: &B) -> Result<T, String> {
    let resp = Request::post(url)
        .json(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(error_message(resp).await);
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

/// POST with no body (e.g. /transcribe) — returns parsed JSON.
pub async fn post_empty<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let resp = Request::post(url).send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(error_message(resp).await);
    }
    resp.json::<T>().await.map_err(|e| e.to_string())
}

pub async fn delete(url: &str) -> Result<(), String> {
    let resp = Request::delete(url).send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(error_message(resp).await);
    }
    Ok(())
}
