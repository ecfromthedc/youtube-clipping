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
uv venv

echo "→ installing dev tooling + package (non-editable)…"
uv pip install pytest ruff           # dev tools
uv pip install . --reinstall-package youtube-clipping   # copy ycp into site-packages

echo "✓ setup complete."
echo "  smoke test:   .venv/bin/ycp demo"
echo "  or:           .venv/bin/python -m ycp demo"
