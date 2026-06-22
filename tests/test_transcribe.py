"""Transcription helper tests — pure command/resolution logic (no whisper run)."""
from __future__ import annotations

from pathlib import Path

from ycp import transcribe


def test_whisper_cpp_cmd_builder():
    cmd = transcribe.whisper_cpp_cmd(
        "whisper-cli", Path("models/ggml-base.en.bin"),
        Path("/tmp/audio.wav"), Path("/tmp/out"), "en")
    assert cmd[0] == "whisper-cli"
    j = " ".join(cmd)
    assert "-m models/ggml-base.en.bin" in j
    assert "-f /tmp/audio.wav" in j
    assert "-l en" in j
    assert "--output-srt" in cmd
    assert "--output-file /tmp/out" in j


def test_model_path_env_override(monkeypatch):
    monkeypatch.setenv("WHISPER_CPP_MODEL", "/abs/model.bin")
    assert transcribe.model_path() == Path("/abs/model.bin")


def test_model_path_default_absolute_under_root(monkeypatch):
    monkeypatch.delenv("WHISPER_CPP_MODEL", raising=False)
    p = transcribe.model_path()
    assert p.is_absolute() and p.name.startswith("ggml-")


def test_find_cpp_binary_returns_str_or_none(monkeypatch):
    monkeypatch.setenv("WHISPER_CPP_BIN", "definitely-not-real-xyz")
    result = transcribe.find_cpp_binary()  # missing explicit -> known names -> maybe None
    assert result is None or isinstance(result, str)
