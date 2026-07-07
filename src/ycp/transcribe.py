"""Transcription — whisper.cpp by default (3–5× faster), openai-whisper fallback.

whisper.cpp wants a GGML model file + 16 kHz mono WAV, so we extract audio with
ffmpeg first. Binary + model are configurable (settings.yaml `transcribe:` block
or WHISPER_CPP_BIN / WHISPER_CPP_MODEL env). If no whisper.cpp binary is found we
fall back to the openai-whisper CLI so the pipeline never hard-breaks.

The command builder + resolution helpers are pure and unit-tested; the subprocess
runners are validated on a real machine (sandbox can't run whisper/ffmpeg).
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
from pathlib import Path

from .config import ROOT, settings
from .srt import Segment, parse_srt

# whisper.cpp binary names, newest naming first
_CPP_BINARIES = ("whisper-cli", "whisper-cpp", "main")


def _cfg() -> dict:
    return settings().get("transcribe", {})


def find_cpp_binary() -> str | None:
    """Locate a whisper.cpp binary: explicit config/env, then known names."""
    explicit = os.getenv("WHISPER_CPP_BIN") or _cfg().get("binary")
    if explicit and shutil.which(explicit):
        return explicit
    for name in _CPP_BINARIES:
        if shutil.which(name):
            return name
    return None


def model_path() -> Path:
    """Resolve the GGML model path (env > settings > default), absolute."""
    p = os.getenv("WHISPER_CPP_MODEL") or _cfg().get("model") or "models/ggml-base.en.bin"
    path = Path(p)
    return path if path.is_absolute() else ROOT / path


def whisper_cpp_cmd(binary: str, model: Path, wav: Path, out_stem: Path,
                    language: str = "en") -> list[str]:
    """Pure whisper.cpp command builder (unit-tested)."""
    return [binary, "-m", str(model), "-f", str(wav), "-l", language,
            "--output-srt", "--output-file", str(out_stem)]


def extract_wav(video: Path, workdir: Path) -> Path:
    """ffmpeg → 16 kHz mono PCM WAV, the input whisper.cpp expects."""
    wav = workdir / "audio.wav"
    cmd = ["ffmpeg", "-y", "-i", str(video), "-ar", "16000", "-ac", "1",
           "-c:a", "pcm_s16le", str(wav)]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    if proc.returncode != 0 or not wav.exists():
        raise RuntimeError(f"audio extract failed: {proc.stderr.strip()[-300:]}")
    return wav


def _run_cpp(video: Path, workdir: Path, binary: str) -> list[Segment]:
    model = model_path()
    if not model.exists():
        raise FileNotFoundError(
            f"whisper.cpp model not found: {model}. Run `scripts/setup-whisper.sh` "
            "or set WHISPER_CPP_MODEL.")
    wav = extract_wav(video, workdir)
    out_stem = workdir / "transcript"
    proc = subprocess.run(
        whisper_cpp_cmd(binary, model, wav, out_stem, _cfg().get("language", "en")),
        capture_output=True, text=True, timeout=3600)
    srt = out_stem.with_suffix(".srt")
    if proc.returncode != 0 or not srt.exists():
        raise RuntimeError(f"whisper.cpp failed: {proc.stderr.strip()[-300:]}")
    return parse_srt(srt.read_text())


def _run_openai(video: Path, workdir: Path) -> list[Segment]:
    model = _cfg().get("openai_model", "small")
    cmd = ["whisper", str(video), "--model", model, "--output_format", "srt",
           "--output_dir", str(workdir), "--language", "en", "--verbose", "False"]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=3600)
    srt = workdir / f"{video.stem}.srt"
    if proc.returncode != 0 or not srt.exists():
        raise RuntimeError(f"openai-whisper failed: {proc.stderr.strip()[-300:]}")
    return parse_srt(srt.read_text())


# Whisper mishears AI proper nouns and the hook/captions parrot the broken word
# ("entropic" → Anthropic). One correction on `segments` fixes both surfaces, since
# hook and captions both derive from Segment.text. Longest keys first so multi-word
# terms win over substrings; word-boundary + case-insensitive.
_TERM_FIXES = {
    "an entropic": "Anthropic", "entropic": "Anthropic",
    "open ai": "OpenAI", "openai": "OpenAI",
    "chat gpt": "ChatGPT", "chatgpt": "ChatGPT",
    "deep seek": "DeepSeek", "deepseek": "DeepSeek",
    "mid journey": "Midjourney", "midjourney": "Midjourney",
    "hugging face": "Hugging Face",
    "nvidia": "Nvidia", "perplexity": "Perplexity", "gemini": "Gemini",
}
_TERM_RE = re.compile(
    r"\b(" + "|".join(re.escape(k) for k in sorted(_TERM_FIXES, key=len, reverse=True)) + r")\b",
    re.IGNORECASE)


def fix_terms(segments: list[Segment]) -> list[Segment]:
    """Correct common Whisper mis-hearings of AI proper nouns. Pure, no I/O.
    Rebuilds each Segment (frozen dataclass); timing preserved."""
    def _fix(text: str) -> str:
        return _TERM_RE.sub(lambda m: _TERM_FIXES[m.group(0).lower()], text)
    return [Segment(s.start, s.end, _fix(s.text)) for s in segments]


def transcribe(video: Path, workdir: Path) -> list[Segment]:
    """whisper.cpp if available (fast), else openai-whisper fallback."""
    binary = find_cpp_binary()
    if binary:
        return fix_terms(_run_cpp(video, workdir, binary))
    print("  · whisper.cpp not found — using openai-whisper fallback "
          "(run scripts/setup-whisper.sh for 3–5× faster transcription)")
    return fix_terms(_run_openai(video, workdir))
