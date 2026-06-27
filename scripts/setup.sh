#!/usr/bin/env bash
# One-time environment setup. Run after cloning / on a new machine:
#   ./scripts/setup.sh
#
# NOTE: this folder name has spaces ("Youtube Clipping Workflow"), which breaks
# hatchling's *editable* install finder. So we install NON-editable (the package
# is copied into site-packages — immune to the space). Tests still run against
# live src via pyproject's `pythonpath`. If you edit ycp source and want the
# `ycp` command (not tests) to reflect it, re-run this script.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "→ creating venv…"
[ -d .venv ] || uv venv   # idempotent: re-runs (after a src edit) reuse the existing venv

echo "→ installing dev tooling + package (non-editable)…"
uv pip install pytest ruff           # dev tools
uv pip install . --reinstall-package youtube-clipping   # copy ycp into site-packages

echo "→ fetching face models (YuNet detect + SFace identity-lock; OpenCV's own, ~40MB)…"
mkdir -p assets/models
_fetch_model() {  # url, dest — idempotent
  [ -f "$2" ] || curl -fsSL -o "$2" "$1" || echo "  ! model fetch failed: $2 (reframe falls back to Haar)"
}
_fetch_model "https://github.com/opencv/opencv_zoo/raw/main/models/face_detection_yunet/face_detection_yunet_2023mar.onnx" \
  "assets/models/face_detection_yunet_2023mar.onnx"
_fetch_model "https://github.com/opencv/opencv_zoo/raw/main/models/face_recognition_sface/face_recognition_sface_2021dec.onnx" \
  "assets/models/face_recognition_sface_2021dec.onnx"

echo "→ installing git hooks (commit-hygiene guard for concurrent agents)…"
git config core.hooksPath .githooks
chmod +x .githooks/* 2>/dev/null || true

echo "✓ setup complete."
echo "  smoke test:   .venv/bin/ycp demo"
echo "  or:           .venv/bin/python -m ycp demo"
