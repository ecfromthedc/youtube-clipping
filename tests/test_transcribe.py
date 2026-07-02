"""Transcription helper tests — pure command/resolution logic (no whisper run)."""
from __future__ import annotations

from pathlib import Path

from ycp import transcribe
from ycp.srt import Segment


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


def test_fix_terms_corrects_ai_proper_nouns():
    segs = [Segment(0.0, 2.0, "we talked to entropic about their new model"),
            Segment(2.0, 4.0, "open ai and deep seek are competitors")]
    fixed = transcribe.fix_terms(segs)
    assert fixed[0].text == "we talked to Anthropic about their new model"
    assert fixed[1].text == "OpenAI and DeepSeek are competitors"
    assert fixed[0].start == 0.0 and fixed[0].end == 2.0  # timing preserved


def test_fix_terms_word_boundary_and_case():
    # word-boundary: no substring clobbering; case-insensitive match, canonical output
    segs = [Segment(0.0, 1.0, "ENTROPIC scaling and midjourney art")]
    assert transcribe.fix_terms(segs)[0].text == "Anthropic scaling and Midjourney art"
