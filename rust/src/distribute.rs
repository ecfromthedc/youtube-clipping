//! Stage 7 — DISTRIBUTE. Approved clips → connected owned channels. Parity port of
//! `src/ycp/distribute.py`.
//!
//! PREFERRED: Postiz (public API) — upload the mp4, then create a post on the channel's
//! integration id. ALTERNATIVE: Repurpose.io (outbox watch-folder). Pick with
//! `distribution.provider`. Both sit behind the `Adapter` trait, so switching is config.
//!
//! Safety: every clip clears the publish gate again right before delivery, and the whole
//! stage is DISABLED by default (`distribution.enabled: false`) until creds are connected.
#![allow(dead_code)] // run()/adapters consumed by the autopilot orchestrator (last port row)

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, FixedOffset, SecondsFormat, TimeZone};
use chrono_tz::Tz;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{json, Value};
use serde_yaml::Value as Yaml;

use crate::db::{self, ClipRow};
use crate::{config, guardrails};

// ── auto-QC (Eric's call §9) ──────────────────────────────────────────────────

/// Auto-QC verdict for one clip. Pure. 'approve' only if it clears the publish gate.
/// `fmt == "auto-clip"` means it went through our cut+caption(+hook) pipeline → transformed.
pub fn qc_decision(clip: &ClipRow) -> (String, String) {
    let gate = guardrails::ClipGate {
        transformed: clip.fmt.as_deref() == Some("auto-clip"),
        // No has_music column; Python `clip.get("has_music", False)` on a DB row is always False.
        has_music: false,
        title: first_truthy(&[clip.post_title.as_deref(), clip.source_creator.as_deref()]),
    };
    let (ok, reason) = guardrails::publish_allowed(&gate);
    if ok {
        ("approve".into(), String::new())
    } else {
        ("reject".into(), reason)
    }
}

/// Apply the auto-QC verdict to every pending_qc clip. Returns (approved, rejected).
pub fn auto_qc(conn: &Connection) -> Result<(i64, i64)> {
    let (mut approved, mut rejected) = (0, 0);
    for clip in db::pending_qc_clips(conn)? {
        let (decision, reason) = qc_decision(&clip);
        db::record_qc(conn, &clip.clip_id, &decision, Some("auto-qc"), Some(&reason))?;
        if decision == "approve" {
            approved += 1;
        } else {
            rejected += 1;
        }
    }
    Ok((approved, rejected))
}

// ── distribution adapter ──────────────────────────────────────────────────────

/// Curated hashtags for a channel slug (from settings). Falls back to `default`, then #shorts.
pub fn hashtags_for(settings: &Yaml, channel: Option<&str>) -> Vec<String> {
    let tags = &settings["distribution"]["hashtags"];
    let pick = |key: &str| -> Option<Vec<String>> {
        let seq = tags.get(key)?.as_sequence()?;
        let v: Vec<String> = seq.iter().filter_map(|x| x.as_str().map(String::from)).collect();
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    };
    pick(channel.unwrap_or(""))
        .or_else(|| pick("default"))
        .unwrap_or_else(|| vec!["#shorts".to_string()])
}

/// The clip's hook — used as the YouTube video title (also burned on the video).
pub fn title_for(clip: &ClipRow) -> String {
    if let Some(t) = clip.post_title.as_deref() {
        if !t.is_empty() {
            return t.to_string();
        }
    }
    let creator = clip.source_creator.as_deref().unwrap_or("");
    // Python: f"{creator} — clip".strip(" —")
    format!("{creator} — clip").trim_matches(|c| c == ' ' || c == '—').to_string()
}

/// Post description: the hook + the channel's hashtags (for discovery).
pub fn caption_for(settings: &Yaml, clip: &ClipRow) -> String {
    let tags = hashtags_for(settings, clip.channel.as_deref()).join(" ");
    format!("{}\n\n{}", title_for(clip), tags).trim().to_string()
}

/// The next `n` posting slots (ISO strings) drawn from `times` (HH:MM, channel-local), at/after
/// `start`, rolling to following days. Pure. Mirrors `assign_slots`.
pub fn assign_slots(n: usize, times: &[String], tz: &str, start: DateTime<FixedOffset>) -> Vec<String> {
    if n == 0 {
        return vec![];
    }
    let zone: Tz = match tz.parse() {
        Ok(z) => z,
        Err(_) => return vec![],
    };
    let mut ordered: Vec<&String> = times.iter().collect();
    ordered.sort(); // lexicographic == chronological for zero-padded HH:MM
    let mut out: Vec<String> = Vec::new();
    let mut day = start.with_timezone(&zone).date_naive();
    while out.len() < n {
        for hhmm in &ordered {
            let mut parts = hhmm.split(':');
            let hh: u32 = parts.next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
            let mm: u32 = parts.next().and_then(|x| x.trim().parse().ok()).unwrap_or(0);
            let cand = match combine_local(zone, day, hh, mm) {
                Some(c) => c,
                None => continue,
            };
            if cand.timestamp() >= start.timestamp() {
                out.push(cand.to_rfc3339_opts(SecondsFormat::Secs, false));
                if out.len() == n {
                    break;
                }
            }
        }
        day = day.succ_opt().unwrap();
    }
    out
}

/// Combine a date + HH:MM in `zone` the way Python's `datetime.combine(..., tzinfo=ZoneInfo)`
/// does with the default `fold=0`:
///   • normal time → that instant;
///   • fall-back ambiguity → the FIRST (earlier) occurrence;
///   • spring-forward gap (a nonexistent wall time) → keep the wall time with the PRE-transition
///     offset (PEP 495 fold=0), which is the literal string Python emits.
fn combine_local(zone: Tz, day: chrono::NaiveDate, hh: u32, mm: u32) -> Option<DateTime<FixedOffset>> {
    let naive = day.and_hms_opt(hh, mm, 0)?;
    match zone.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => Some(dt.fixed_offset()),
        chrono::LocalResult::Ambiguous(earlier, _later) => Some(earlier.fixed_offset()),
        chrono::LocalResult::None => {
            // In the gap: borrow the offset from safely before the (≤1h) transition.
            let before = zone.from_local_datetime(&(naive - chrono::Duration::hours(2))).earliest()?;
            let off = *before.fixed_offset().offset();
            naive.and_local_timezone(off).single()
        }
    }
}

/// Metadata handed to an adapter for one delivery (mirrors the Python meta dict).
#[derive(Debug, Default, Serialize)]
pub struct DeliverMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip_id: Option<String>,
    pub caption: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// Pull the Postiz post id from a POST /posts response ([{postId,...}] or a dict). Mirrors `_post_id`.
pub fn post_id(out: &Value) -> String {
    let obj = match out.as_array() {
        Some(arr) => arr.first().unwrap_or(out),
        None => out,
    };
    if let Some(map) = obj.as_object() {
        // Python: out.get("postId") or out.get("id") or "posted" — falsy values fall through.
        let pick = map.get("postId").filter(|v| truthy(v)).or_else(|| map.get("id").filter(|v| truthy(v)));
        return match pick {
            Some(v) => val_str(v),
            None => "posted".to_string(),
        };
    }
    "posted".to_string()
}

pub trait Adapter {
    fn deliver(&self, clip_path: &Path, meta: &DeliverMeta) -> Result<String>;
}

/// ALTERNATIVE (Repurpose.io): drop clip + JSON sidecar into Repurpose's watch-folder.
pub struct OutboxAdapter {
    pub outbox: PathBuf,
}

impl Adapter for OutboxAdapter {
    fn deliver(&self, clip_path: &Path, meta: &DeliverMeta) -> Result<String> {
        std::fs::create_dir_all(&self.outbox)?;
        let name = clip_path.file_name().unwrap_or_default();
        let dest = self.outbox.join(name);
        if clip_path.exists() {
            std::fs::copy(clip_path, &dest)?;
        }
        let stem = clip_path.file_stem().unwrap_or_default().to_string_lossy();
        std::fs::write(self.outbox.join(format!("{stem}.json")), serde_json::to_string_pretty(meta)?)?;
        Ok(dest.to_string_lossy().to_string())
    }
}

/// PREFERRED: posts approved clips to channels connected in Postiz via its public API.
pub struct PostizAdapter {
    token: String,
    api_url: String,
    channels: std::collections::HashMap<String, String>,
    schedule: String,
}

impl PostizAdapter {
    /// Direct constructor (used by the editor's per-publish path: it picks the
    /// integration id at the call site rather than from settings.yaml).
    pub fn new(token: String, api_url: String, channels: std::collections::HashMap<String, String>, schedule: String) -> Self {
        Self {
            token,
            api_url: api_url.trim_end_matches('/').to_string(),
            channels,
            schedule,
        }
    }

    pub fn from_config(pz: &Yaml) -> Result<Self> {
        let token_env = pz.get("token_env").and_then(|v| v.as_str()).unwrap_or("POSTIZ_API_TOKEN");
        // Mirrors Python: reads os.environ only (not the .env loader).
        let token = std::env::var(token_env).unwrap_or_default();
        if token.is_empty() {
            bail!("POSTIZ_API_TOKEN not set — add it to .env (see DISTRIBUTION.md / SETUP §3).");
        }
        let channels = pz
            .get("channels")
            .and_then(|v| v.as_mapping())
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_str().unwrap_or("").to_string())))
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self {
            token,
            api_url: pz
                .get("api_url")
                .and_then(|v| v.as_str())
                .unwrap_or("https://api.postiz.com/public/v1")
                .trim_end_matches('/')
                .to_string(),
            channels,
            schedule: pz.get("schedule").and_then(|v| v.as_str()).unwrap_or("now").to_string(),
        })
    }
}

impl Adapter for PostizAdapter {
    fn deliver(&self, clip_path: &Path, meta: &DeliverMeta) -> Result<String> {
        let channel = meta.channel.as_deref().unwrap_or("");
        let integration_id = self.channels.get(channel).filter(|s| !s.is_empty()).ok_or_else(|| {
            anyhow!(
                "no Postiz integration id for channel {:?} — map it in distribution.postiz.channels \
                 (ids from GET /public/v1/integrations).",
                meta.channel
            )
        })?;
        let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(180)).build()?;
        let bytes = std::fs::read(clip_path)?;
        let part = reqwest::blocking::multipart::Part::bytes(bytes)
            .file_name(clip_path.file_name().unwrap_or_default().to_string_lossy().to_string())
            .mime_str("video/mp4")?;
        let form = reqwest::blocking::multipart::Form::new().part("file", part);
        let media: Value = client
            .post(format!("{}/upload", self.api_url))
            .header("Authorization", &self.token)
            .multipart(form)
            .send()?
            .error_for_status()?
            .json()?;
        let caption = &meta.caption;
        // Python: (meta.get("title") or caption[:100])[:100]
        let title_src = match meta.title.as_deref() {
            Some(t) if !t.is_empty() => t.to_string(),
            _ => caption.clone(),
        };
        let title: String = title_src.chars().take(100).collect();
        let body = json!({
            "type": self.schedule,
            "date": meta.date.clone().unwrap_or_else(db::now),
            "shortLink": false,
            "tags": [],
            "posts": [{
                "integration": {"id": integration_id},
                "value": [{"content": caption,
                           "image": [{"id": media.get("id"), "path": media.get("path")}]}],
                "settings": {
                    "__type": meta.platform.clone().unwrap_or_else(|| "youtube".into()),
                    "title": title,
                    "type": meta.privacy.clone().unwrap_or_else(|| "public".into()),
                },
            }],
        });
        let resp: Value = client
            .post(format!("{}/posts", self.api_url))
            .header("Authorization", &self.token)
            .json(&body)
            .send()?
            .error_for_status()?
            .json()?;
        Ok(post_id(&resp))
    }
}

fn resolve_outbox(cfg: &Yaml, root: &Path) -> PathBuf {
    let p = cfg.get("outbox").and_then(|v| v.as_str()).unwrap_or("data/outbox");
    let pb = PathBuf::from(p);
    if pb.is_absolute() {
        pb
    } else {
        root.join(pb)
    }
}

/// Select the distribution adapter. Postiz (API) preferred; Repurpose.io (outbox) alternative.
pub fn build_adapter(cfg: &Yaml, root: &Path) -> Result<Box<dyn Adapter>> {
    let provider = cfg
        .get("provider")
        .or_else(|| cfg.get("adapter"))
        .and_then(|v| v.as_str())
        .unwrap_or("postiz")
        .to_lowercase();
    if provider.starts_with("postiz") {
        return Ok(Box::new(PostizAdapter::from_config(&cfg["postiz"])?));
    }
    if provider.starts_with("repurpose") || provider.contains("outbox") {
        return Ok(Box::new(OutboxAdapter { outbox: resolve_outbox(cfg, root) }));
    }
    bail!("unknown distribution provider {provider:?} (use 'postiz' or 'repurpose').")
}

/// Outcome of a distribute run (mirrors the Python result dict; unused fields stay default).
#[derive(Debug, Default, Serialize)]
pub struct RunResult {
    pub enabled: bool,
    pub delivered: i64,
    pub blocked: i64,
    pub parked: i64,
    pub failed: i64,
    pub waiting: i64,
    pub note: String,
}

/// Hand approved clips to the distribution adapter, marking them posted. Gated by
/// `distribution.enabled`. Re-checks the publish gate per clip. Mirrors `run`.
pub fn run(conn: &Connection, root: &Path) -> Result<RunResult> {
    let settings = config::load_settings(root)?;
    let cfg = &settings["distribution"];
    if !cfg.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false) {
        let n = db::approved_clips(conn)?.len() as i64;
        return Ok(RunResult {
            enabled: false,
            waiting: n,
            note: "distribution OFF — set POSTIZ_API_TOKEN, connect channels in Postiz + map them \
                   in distribution.postiz.channels, then set distribution.enabled: true \
                   (see DISTRIBUTION.md)"
                .to_string(),
            ..Default::default()
        });
    }
    let adapter = build_adapter(cfg, root)?;
    let provider = cfg.get("provider").and_then(|v| v.as_str()).unwrap_or("postiz").to_string();
    let pz = &cfg["postiz"];

    // Postiz: only channels with a mapped integration id can post; others are PARKED.
    let mapped: Option<HashSet<String>> = if provider.starts_with("postiz") {
        Some(
            pz.get("channels")
                .and_then(|v| v.as_mapping())
                .map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| {
                            if v.as_str().unwrap_or("").is_empty() {
                                None
                            } else {
                                Some(k.as_str()?.to_string())
                            }
                        })
                        .collect()
                })
                .unwrap_or_default(),
        )
    } else {
        None
    };

    let mut postable: Vec<ClipRow> = Vec::new();
    let mut parked = 0i64;
    for clip in db::approved_clips(conn)? {
        let blocked_by_map = match &mapped {
            Some(set) => !clip.channel.as_deref().map(|c| set.contains(c)).unwrap_or(false),
            None => false,
        };
        if blocked_by_map {
            parked += 1;
        } else {
            postable.push(clip);
        }
    }

    // Schedule mode → assign each postable clip to the next free posting slot.
    let mut slots: Vec<String> = Vec::new();
    if provider.starts_with("postiz") && pz.get("schedule").and_then(|v| v.as_str()) == Some("schedule") {
        let tz = pz.get("timezone").and_then(|v| v.as_str()).unwrap_or("UTC");
        let times: Vec<String> = pz
            .get("posting_times")
            .and_then(|v| v.as_sequence())
            .map(|s| s.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        slots = assign_slots(postable.len(), &times, tz, chrono::Utc::now().fixed_offset());
    }

    let (mut delivered, mut blocked, mut failed) = (0i64, 0i64, 0i64);
    for (i, clip) in postable.iter().enumerate() {
        let gate = guardrails::ClipGate {
            transformed: clip.fmt.as_deref() == Some("auto-clip"),
            has_music: false,
            title: caption_for(&settings, clip),
        };
        let (ok, _reason) = guardrails::publish_allowed(&gate);
        if !ok {
            db::set_clip_status(conn, &clip.clip_id, "rejected", &[])?;
            blocked += 1;
            continue;
        }
        let meta = DeliverMeta {
            clip_id: Some(clip.clip_id.clone()),
            caption: caption_for(&settings, clip),
            title: Some(title_for(clip)),
            channel: clip.channel.clone(),
            platform: clip.platform.clone(),
            privacy: None,
            date: slots.get(i).cloned(),
        };
        let clip_path = PathBuf::from(clip.post_url.clone().unwrap_or_default());
        match adapter.deliver(&clip_path, &meta) {
            Ok(dest) => {
                let now = db::now();
                db::set_clip_status(
                    conn,
                    &clip.clip_id,
                    "posted",
                    &[("post_id", dest.as_str()), ("posted_at", now.as_str())],
                )?;
                delivered += 1;
            }
            Err(exc) => {
                let msg: String = exc.to_string().chars().take(140).collect();
                println!(
                    "  ! post failed for {} ({}): {}",
                    clip.clip_id,
                    clip.channel.as_deref().unwrap_or(""),
                    msg
                );
                failed += 1;
            }
        }
    }
    Ok(RunResult { enabled: true, delivered, blocked, parked, failed, ..Default::default() })
}

// ── small helpers mirroring Python truthiness ─────────────────────────────────

/// First non-empty string in order, else "" (mirrors Python `a or b or ""`).
fn first_truthy(opts: &[Option<&str>]) -> String {
    for s in opts.iter().flatten() {
        if !s.is_empty() {
            return s.to_string();
        }
    }
    String::new()
}

/// Python `str(value)` for a JSON scalar (no surrounding quotes on strings).
fn val_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Python truthiness for a JSON value (used to mirror `a or b`).
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    fn clip(fmt: &str, post_title: Option<&str>, creator: Option<&str>, channel: Option<&str>) -> ClipRow {
        ClipRow {
            fmt: Some(fmt.to_string()),
            post_title: post_title.map(String::from),
            source_creator: creator.map(String::from),
            channel: channel.map(String::from),
            ..Default::default()
        }
    }

    fn settings() -> Yaml {
        // The hashtags block from config/settings.yaml (the bits the tests touch).
        serde_yaml::from_str(
            "distribution:\n  hashtags:\n    default: [\"#shorts\"]\n    \
             money-fights: [\"#shorts\", \"#money\", \"#investing\"]\n",
        )
        .unwrap()
    }

    #[test]
    fn auto_qc_approves_transformed_clean_clip() {
        let (d, _) = qc_decision(&clip("auto-clip", None, Some("Ramit Sethi"), None));
        assert_eq!(d, "approve");
    }

    #[test]
    fn auto_qc_rejects_untransformed_clip() {
        let (d, reason) = qc_decision(&clip("raw", None, Some("x"), None));
        assert_eq!(d, "reject");
        assert!(reason.to_lowercase().contains("transform"));
    }

    #[test]
    fn caption_falls_back_to_creator() {
        let cap = caption_for(&settings(), &clip("auto-clip", None, Some("Codie Sanchez"), None));
        assert!(cap.starts_with("Codie Sanchez — clip"));
        assert!(cap.contains("#shorts")); // default hashtags when no channel
    }

    #[test]
    fn caption_includes_channel_hashtags() {
        let cap = caption_for(
            &settings(),
            &clip("auto-clip", Some("they make 400k and still fight:"), None, Some("money-fights")),
        );
        assert!(cap.starts_with("they make 400k and still fight:"));
        assert!(cap.contains("#money") && cap.contains("#investing"));
    }

    #[test]
    fn assign_slots_rolls_across_days_in_order() {
        let start = DateTime::parse_from_rfc3339("2026-06-24T07:00:00-04:00").unwrap(); // past 06:00
        let times = ["06:00", "12:30", "19:00"].map(String::from);
        let slots = assign_slots(4, &times, "America/New_York", start);
        assert_eq!(slots.len(), 4);
        let parsed: Vec<DateTime<FixedOffset>> =
            slots.iter().map(|s| DateTime::parse_from_rfc3339(s).unwrap()).collect();
        let mut sorted = parsed.clone();
        sorted.sort();
        assert_eq!(parsed, sorted); // strictly chronological
        assert_eq!((parsed[0].hour(), parsed[0].minute()), (12, 30)); // 06:00 today passed
        assert!(parsed[2].date_naive() > parsed[0].date_naive()); // 4th rolled to next day
    }

    #[test]
    fn assign_slots_empty_when_zero() {
        let start = DateTime::parse_from_rfc3339("2026-06-24T00:00:00+00:00").unwrap();
        assert!(assign_slots(0, &["06:00".to_string()], "UTC", start).is_empty());
    }

    #[test]
    fn outbox_adapter_writes_clip_and_sidecar() {
        let tmp = std::env::temp_dir().join("ycp_outbox_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let src = tmp.join("clip.mp4");
        std::fs::write(&src, b"fake mp4").unwrap();
        let adapter = OutboxAdapter { outbox: tmp.join("outbox") };
        let meta = DeliverMeta { clip_id: Some("abc".into()), caption: "Hook here".into(), ..Default::default() };
        let dest = adapter.deliver(&src, &meta).unwrap();
        assert!(Path::new(&dest).exists());
        let sidecar = tmp.join("outbox").join("clip.json");
        assert!(sidecar.exists());
        assert!(std::fs::read_to_string(&sidecar).unwrap().contains("Hook here"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn postiz_adapter_unknown_channel_raises() {
        // Channel-id check happens before any network call (Python test parity).
        let adapter = PostizAdapter {
            token: "t".into(),
            api_url: "x".into(),
            channels: std::collections::HashMap::new(),
            schedule: "now".into(),
        };
        let tmp = std::env::temp_dir().join("ycp_postiz_nochan.mp4");
        std::fs::write(&tmp, b"x").unwrap();
        let meta = DeliverMeta { channel: Some("nope".into()), caption: "h".into(), ..Default::default() };
        let err = adapter.deliver(&tmp, &meta).unwrap_err();
        assert!(err.to_string().contains("integration id"));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn postiz_from_config_requires_token() {
        // token_env points at a name nothing else uses → hermetic, no real-env mutation.
        let pz: Yaml = serde_yaml::from_str("token_env: YCP_TEST_NO_SUCH_TOKEN\n").unwrap();
        assert!(PostizAdapter::from_config(&pz).is_err());
    }

    #[test]
    fn build_adapter_selects_provider() {
        let tmp = std::env::temp_dir().join("ycp_outbox_sel");
        // repurpose → OutboxAdapter (absolute outbox).
        let cfg: Yaml = serde_yaml::from_str(&format!(
            "provider: repurpose\noutbox: \"{}\"\n",
            tmp.to_string_lossy()
        ))
        .unwrap();
        assert!(build_adapter(&cfg, Path::new("/")).is_ok());

        // postiz → PostizAdapter (unique token env name avoids clobbering real POSTIZ_API_TOKEN).
        std::env::set_var("YCP_TEST_BUILD_TOK", "tok");
        let cfg: Yaml =
            serde_yaml::from_str("provider: postiz\npostiz:\n  token_env: YCP_TEST_BUILD_TOK\n").unwrap();
        assert!(build_adapter(&cfg, Path::new("/")).is_ok());
        std::env::remove_var("YCP_TEST_BUILD_TOK");
    }

    #[test]
    fn post_id_handles_list_dict_and_fallback() {
        assert_eq!(post_id(&json!([{"postId": "p-1"}])), "p-1");
        assert_eq!(post_id(&json!({"id": "post-9"})), "post-9"); // postId missing → id
        assert_eq!(post_id(&json!([])), "posted"); // empty list → fallback
        assert_eq!(post_id(&json!({"postId": "", "id": "x"})), "x"); // falsy postId → id
        assert_eq!(post_id(&json!({})), "posted");
    }

    #[test]
    fn auto_qc_records_verdicts() {
        let conn = Connection::open_in_memory().unwrap();
        db::init(&conn).unwrap();
        // one transformed (approve), one raw (reject)
        for (cid, fmt) in [("ok", "auto-clip"), ("raw", "raw")] {
            conn.execute(
                "INSERT INTO clips (clip_id, channel, platform, lane, fmt, status, created_at)
                 VALUES (?1,'ch','youtube','owned',?2,'pending_qc','2026-01-01T00:00:00Z')",
                rusqlite::params![cid, fmt],
            )
            .unwrap();
        }
        let (approved, rejected) = auto_qc(&conn).unwrap();
        assert_eq!((approved, rejected), (1, 1));
        let status: String = conn
            .query_row("SELECT status FROM clips WHERE clip_id='ok'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(status, "approved");
    }
}
