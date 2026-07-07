//! Configuration: locate the project root, load settings.yaml, resolve paths.
//! Mirrors src/ycp/config.py.
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

/// Locate the project root (holds config/ and data/).
/// Order: $YCP_HOME → nearest ancestor of CWD with config/settings.yaml.
pub fn find_root() -> Result<PathBuf> {
    if let Ok(home) = std::env::var("YCP_HOME") {
        return Ok(PathBuf::from(home));
    }
    let cwd = std::env::current_dir().context("current_dir")?;
    for dir in cwd.ancestors() {
        if dir.join("config").join("settings.yaml").is_file() {
            return Ok(dir.to_path_buf());
        }
    }
    bail!(
        "could not find config/settings.yaml above {}",
        cwd.display()
    )
}

pub fn db_path(root: &Path) -> PathBuf {
    root.join("data").join("clips.db")
}

pub fn data_dir(root: &Path) -> PathBuf {
    root.join("data")
}

/// Load settings.yaml as a generic value — typed views are pulled lazily per module so we
/// don't have to model the whole schema up front (it's ported incrementally).
pub fn load_settings(root: &Path) -> Result<serde_yaml::Value> {
    let p = root.join("config").join("settings.yaml");
    let text = std::fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
    Ok(serde_yaml::from_str(&text)?)
}

/// One `.env` var by name (loads `.env` lazily; real env takes precedence). Mirrors env().
pub fn env_var(root: &Path, key: &str) -> Option<String> {
    if let Ok(v) = std::env::var(key) {
        if !v.is_empty() {
            return Some(v);
        }
    }
    let env_path = root.join(".env");
    let text = std::fs::read_to_string(env_path).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || !line.contains('=') {
            continue;
        }
        let (k, v) = line.split_once('=').unwrap();
        if k.trim() == key {
            let v = v.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}
