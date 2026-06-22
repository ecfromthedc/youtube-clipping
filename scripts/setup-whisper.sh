#!/usr/bin/env bash
# One-time: install whisper.cpp + download a GGML model (the fast transcription engine).
#   ./scripts/setup-whisper.sh            # base.en  (fast, English — default)
#   ./scripts/setup-whisper.sh small.en   # better quality, a bit slower
#   ./scripts/setup-whisper.sh large-v3   # best quality, slowest
set -euo pipefail
cd "$(dirname "$0")/.."

MODEL="${1:-base.en}"

echo "→ ensuring whisper.cpp is installed…"
if ! command -v whisper-cli >/dev/null 2>&1 && ! command -v whisper-cpp >/dev/null 2>&1; then
  if command -v brew >/dev/null 2>&1; then
    brew install whisper-cpp
  else
    echo "✗ Homebrew not found. Install whisper.cpp manually:"
    echo "  https://github.com/ggml-org/whisper.cpp"
    exit 1
  fi
fi

mkdir -p models
OUT="models/ggml-${MODEL}.bin"
if [ -f "$OUT" ]; then
  echo "✓ model already present: $OUT"
else
  echo "→ downloading ggml-${MODEL}.bin (one-time)…"
  curl -L --fail -o "$OUT" \
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-${MODEL}.bin"
fi

echo "✓ whisper.cpp ready — model: $OUT"
echo "  (config/settings.yaml already points transcribe.model at models/ggml-base.en.bin)"
echo "  test:  .venv/bin/python -m ycp clip <url> --max 2"
