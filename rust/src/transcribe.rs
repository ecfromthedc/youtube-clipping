//! Transcription — whisper.cpp by default (3–5× faster), openai-whisper fallback.
//! Parity port of `src/ycp/transcribe.py`.
//!
//! whisper.cpp wants a GGML model + 16 kHz mono WAV, so we extract audio with ffmpeg first.
//! Binary + model are configurable (settings.yaml `transcribe:` or WHISPER_CPP_BIN /
//! WHISPER_CPP_MODEL env). No whisper.cpp binary found → fall back to the openai-whisper CLI.
//!
//! The pure resolution/command builders (`find_cpp_binary`, `model_path`, `whisper_cpp_cmd`)
//! are unit-tested; the subprocess runners shell out exactly as the Python does (real machine).
#![allow(dead_code)] // shell runners wired by the clip/autopilot rows

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Result};

use crate::config;
use crate::srt::{parse_srt, Segment};

/// whisper.cpp binary names, newest naming first (mirrors `_CPP_BINARIES`).
const CPP_BINARIES: &[&str] = &["whisper-cli", "whisper-cpp", "main"];

fn cfg(root: &Path) -> serde_yaml::Value {
    config::load_settings(root)
        .ok()
        .and_then(|s| s.get("transcribe").cloned())
        .unwrap_or(serde_yaml::Value::Null)
}

fn cfg_str(cfg: &serde_yaml::Value, key: &str) -> Option<String> {
    cfg.get(key).and_then(|v| v.as_str()).map(String::from)
}

/// True if `name` resolves to an existing file (absolute path) or is found on PATH.
/// Mirrors `shutil.which` as a boolean existence check (ponytail: skips the exec-bit test).
fn which(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let p = Path::new(name);
    if p.is_absolute() || name.contains('/') {
        return p.is_file();
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            if dir.join(name).is_file() {
                return true;
            }
        }
    }
    false
}

/// Locate a whisper.cpp binary: explicit config/env, then known names. Mirrors `find_cpp_binary`.
pub fn find_cpp_binary(root: &Path) -> Option<String> {
    let c = cfg(root);
    let explicit = std::env::var("WHISPER_CPP_BIN")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| cfg_str(&c, "binary"));
    if let Some(b) = explicit {
        if which(&b) {
            return Some(b);
        }
    }
    CPP_BINARIES
        .iter()
        .find(|n| which(n))
        .map(|s| s.to_string())
}

/// Resolve the GGML model path (env > settings > default), absolute. Mirrors `model_path`.
pub fn model_path(root: &Path) -> PathBuf {
    let c = cfg(root);
    let p = std::env::var("WHISPER_CPP_MODEL")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| cfg_str(&c, "model"))
        .unwrap_or_else(|| "models/ggml-base.en.bin".to_string());
    let path = PathBuf::from(p);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

/// Pure whisper.cpp command builder (unit-tested). Mirrors `whisper_cpp_cmd`.
pub fn whisper_cpp_cmd(
    binary: &str,
    model: &Path,
    wav: &Path,
    out_stem: &Path,
    language: &str,
) -> Vec<String> {
    vec![
        binary.to_string(),
        "-m".into(),
        model.display().to_string(),
        "-f".into(),
        wav.display().to_string(),
        "-l".into(),
        language.to_string(),
        "--output-srt".into(),
        "--output-file".into(),
        out_stem.display().to_string(),
    ]
}

/// ffmpeg → 16 kHz mono PCM WAV, the input whisper.cpp expects. Mirrors `extract_wav`.
pub fn extract_wav(video: &Path, workdir: &Path) -> Result<PathBuf> {
    let wav = workdir.join("audio.wav");
    let out = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(video)
        .args(["-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le"])
        .arg(&wav)
        .output()?;
    if !out.status.success() || !wav.exists() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("audio extract failed: {}", tail(&err, 300));
    }
    Ok(wav)
}

fn run_cpp(root: &Path, video: &Path, workdir: &Path, binary: &str) -> Result<Vec<Segment>> {
    let model = model_path(root);
    if !model.exists() {
        bail!(
            "whisper.cpp model not found: {}. Run `scripts/setup-whisper.sh` or set WHISPER_CPP_MODEL.",
            model.display()
        );
    }
    let wav = extract_wav(video, workdir)?;
    let out_stem = workdir.join("transcript");
    let lang = cfg_str(&cfg(root), "language").unwrap_or_else(|| "en".to_string());
    let cmd = whisper_cpp_cmd(binary, &model, &wav, &out_stem, &lang);
    let out = Command::new(&cmd[0]).args(&cmd[1..]).output()?;
    let srt = out_stem.with_extension("srt");
    if !out.status.success() || !srt.exists() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("whisper.cpp failed: {}", tail(&err, 300));
    }
    Ok(parse_srt(&std::fs::read_to_string(srt)?))
}

fn run_openai(root: &Path, video: &Path, workdir: &Path) -> Result<Vec<Segment>> {
    let model = cfg_str(&cfg(root), "openai_model").unwrap_or_else(|| "small".to_string());
    let out = Command::new("whisper")
        .arg(video)
        .args(["--model", &model, "--output_format", "srt", "--output_dir"])
        .arg(workdir)
        .args(["--language", "en", "--verbose", "False"])
        .output()?;
    let stem = video
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("source");
    let srt = workdir.join(format!("{stem}.srt"));
    if !out.status.success() || !srt.exists() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("openai-whisper failed: {}", tail(&err, 300));
    }
    Ok(parse_srt(&std::fs::read_to_string(srt)?))
}

/// whisper.cpp if available (fast), else openai-whisper fallback. Mirrors `transcribe`.
pub fn transcribe(root: &Path, video: &Path, workdir: &Path) -> Result<Vec<Segment>> {
    match find_cpp_binary(root) {
        Some(binary) => run_cpp(root, video, workdir, &binary),
        None => {
            println!(
                "  · whisper.cpp not found — using openai-whisper fallback \
                 (run scripts/setup-whisper.sh for 3–5× faster transcription)"
            );
            run_openai(root, video, workdir)
        }
    }
}

/// Last `n` chars of a stderr blob (mirrors Python `[-300:]` trailing-context slices).
fn tail(s: &str, n: usize) -> String {
    let s = s.trim();
    let start = s
        .char_indices()
        .rev()
        .nth(n.saturating_sub(1))
        .map(|(i, _)| i)
        .unwrap_or(0);
    s[start..].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whisper_cpp_cmd_builder() {
        let cmd = whisper_cpp_cmd(
            "whisper-cli",
            Path::new("models/ggml-base.en.bin"),
            Path::new("/tmp/audio.wav"),
            Path::new("/tmp/out"),
            "en",
        );
        assert_eq!(cmd[0], "whisper-cli");
        let j = cmd.join(" ");
        assert!(j.contains("-m models/ggml-base.en.bin"));
        assert!(j.contains("-f /tmp/audio.wav"));
        assert!(j.contains("-l en"));
        assert!(cmd.iter().any(|a| a == "--output-srt"));
        assert!(j.contains("--output-file /tmp/out"));
    }

    #[test]
    fn model_path_default_is_absolute_ggml() {
        std::env::remove_var("WHISPER_CPP_MODEL");
        let p = model_path(Path::new("/some/root"));
        assert!(p.is_absolute());
        assert!(p
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("ggml-"));
    }

    #[test]
    fn which_finds_sh_not_garbage() {
        assert!(which("sh") || which("/bin/sh"));
        assert!(!which("definitely-not-real-xyz-binary"));
    }
}
