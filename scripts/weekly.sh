#!/usr/bin/env bash
# Weekly Double-Down Brief. Add to cron, e.g. Monday 8am:
#   0 8 * * 1  "/Users/ecfromthedc/Desktop/Development/Youtube Clipping Workflow/scripts/weekly.sh"
set -euo pipefail
cd "$(dirname "$0")/.."
source .venv/bin/activate

echo "[$(date)] generating Double-Down Brief…"
# Add --post-slack once SLACK_BOT_TOKEN + SLACK_QC_CHANNEL are set in .env
ycp brief

echo "[$(date)] brief saved to data/latest-brief.md"
