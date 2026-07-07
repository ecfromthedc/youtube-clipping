//! Archive every produced clip + its metadata to durable storage.
//! Parity port of `src/ycp/archive.py`. Best-effort and decoupled: a failed archive NEVER
//! breaks the pipeline (the clip still posts from local). Autopilot wires these up.
//!
//! `settings.archive.dest`:
//!   - ""            → off (clips stay in local data/clips/).
//!   - absolute/~    → copy there (e.g. a Drive for Desktop synced folder).
//!   - "remote:path" → rclone copy (a Google Drive remote — headless, portable).
#![allow(dead_code)] // consumed by the autopilot orchestrator (last port row)

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use rusqlite::Connection;
use serde_json::Value;

use crate::config;

/// rclone remotes look like 'name:path'; local paths are absolute or ~/.-relative.
pub fn is_rclone(dest: &str) -> bool {
    dest.contains(':') && !(dest.starts_with('/') || dest.starts_with('~') || dest.starts_with('.'))
}

/// Expand a leading `~` to $HOME (mirrors Python `Path.expanduser`).
fn expand_tilde(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix('~') {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest.trim_start_matches('/'));
        }
    }
    PathBuf::from(p)
}

/// Copy a clip + a JSON sidecar to the configured drive. Returns the destination, or None
/// when archiving is off or fails (caller treats it as best-effort). Mirrors `archive_clip`.
pub fn archive_clip(
    settings: &serde_yaml::Value,
    clip_path: &Path,
    meta: &Value,
) -> Option<String> {
    let cfg = &settings["archive"];
    let dest = cfg
        .get("dest")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if dest.is_empty() || !clip_path.exists() {
        return None;
    }
    let sub = if cfg
        .get("subfolder_by_channel")
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
    {
        // Python: meta.get("channel") or "clips" — empty string falls through to "clips".
        meta.get("channel")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("clips")
            .to_string()
    } else {
        String::new()
    };

    // Sidecar (best-effort): None if the write fails, like Python's OSError branch.
    let sidecar_path = clip_path.with_extension("json");
    let sidecar = serde_json::to_string_pretty(meta)
        .ok()
        .filter(|s| std::fs::write(&sidecar_path, s).is_ok())
        .map(|_| sidecar_path);

    let mut files: Vec<&Path> = vec![clip_path];
    if let Some(ref sc) = sidecar {
        files.push(sc);
    }

    let target = if is_rclone(&dest) {
        let base = dest.trim_end_matches('/');
        let target = if sub.is_empty() {
            base.to_string()
        } else {
            format!("{base}/{sub}")
        };
        for f in &files {
            // ponytail: no watchdog — rclone copy of a single file is bounded; add a timeout
            // wrapper if a hung remote ever stalls the pipeline.
            let ok = Command::new("rclone")
                .args(["copy"])
                .arg(f)
                .arg(&target)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !ok {
                return None;
            }
        }
        target
    } else {
        let base = expand_tilde(&dest);
        let target_dir = if sub.is_empty() {
            base
        } else {
            base.join(&sub)
        };
        if std::fs::create_dir_all(&target_dir).is_err() {
            return None;
        }
        for f in &files {
            let name = f.file_name()?;
            if std::fs::copy(f, target_dir.join(name)).is_err() {
                return None;
            }
        }
        target_dir.to_string_lossy().to_string()
    };

    Some(format!(
        "{}/{}",
        target,
        clip_path.file_name()?.to_string_lossy()
    ))
}

/// Delete local clip files (+ sidecars) for clips already POSTED — they're live + archived,
/// so the local copy is redundant. Returns files removed. Mirrors `prune_local`.
pub fn prune_local(conn: &Connection, root: &Path) -> Result<i64> {
    let clips_dir = config::data_dir(root).join("clips");
    if !clips_dir.exists() {
        return Ok(0);
    }
    let ids: Vec<String> = {
        let mut stmt = conn.prepare("SELECT clip_id FROM clips WHERE status='posted'")?;
        let v = stmt
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        v
    };
    let mut removed = 0;
    for cid in ids {
        for f in [
            clips_dir.join(format!("{cid}.mp4")),
            clips_dir.join(format!("{cid}.json")),
        ] {
            if f.exists() && std::fs::remove_file(&f).is_ok() {
                removed += 1;
            }
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_rclone_routing() {
        // Mirrors test_archive.py::test_is_rclone_routing.
        assert!(is_rclone("drive:Channel/clips"));
        assert!(!is_rclone("/Users/x/Google Drive"));
        assert!(!is_rclone("~/Drive"));
        assert!(!is_rclone("")); // off → not a remote
    }

    #[test]
    fn archive_to_local_dir_copies_clip_and_sidecar() {
        let tmp = std::env::temp_dir().join("ycp_archive_local");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let clip = tmp.join("c1.mp4");
        std::fs::write(&clip, b"video-bytes").unwrap();
        let dest = tmp.join("drive");
        let settings: serde_yaml::Value = serde_yaml::from_str(&format!(
            "archive:\n  dest: \"{}\"\n  subfolder_by_channel: true\n",
            dest.to_string_lossy()
        ))
        .unwrap();
        let meta = serde_json::json!({"clip_id": "c1", "channel": "hot-seat", "hook": "h"});
        let out = archive_clip(&settings, &clip, &meta);
        assert!(out.is_some());
        assert_eq!(
            std::fs::read(dest.join("hot-seat").join("c1.mp4")).unwrap(),
            b"video-bytes"
        );
        assert!(dest.join("hot-seat").join("c1.json").exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn archive_off_returns_none() {
        let tmp = std::env::temp_dir().join("ycp_archive_off");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let clip = tmp.join("c.mp4");
        std::fs::write(&clip, b"v").unwrap();
        let settings: serde_yaml::Value = serde_yaml::from_str("archive:\n  dest: \"\"\n").unwrap();
        assert!(archive_clip(&settings, &clip, &serde_json::json!({"clip_id": "c"})).is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn prune_local_removes_only_posted() {
        // Mirrors test_archive.py::test_prune_local_removes_only_posted.
        let tmp = std::env::temp_dir().join("ycp_prune_local");
        let _ = std::fs::remove_dir_all(&tmp);
        let clips = tmp.join("data").join("clips");
        std::fs::create_dir_all(&clips).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init(&conn).unwrap();
        for (cid, status) in [("p1", "posted"), ("q1", "pending_qc")] {
            conn.execute(
                "INSERT INTO clips (clip_id, channel, platform, lane, status, created_at)
                 VALUES (?1,'c','youtube','owned',?2,'2026-01-01T00:00:00Z')",
                rusqlite::params![cid, status],
            )
            .unwrap();
            std::fs::write(clips.join(format!("{cid}.mp4")), b"v").unwrap();
            std::fs::write(clips.join(format!("{cid}.json")), "{}").unwrap();
        }
        let removed = prune_local(&conn, &tmp).unwrap();
        assert_eq!(removed, 2); // p1.mp4 + p1.json
        assert!(!clips.join("p1.mp4").exists());
        assert!(clips.join("q1.mp4").exists()); // un-posted kept
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
