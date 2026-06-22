#!/usr/bin/env bash
# Daily closed-loop tick. Add to cron, e.g. 7am:
#   0 7 * * *  "/Users/ecfromthedc/Desktop/Development/Youtube Clipping Workflow/scripts/daily.sh"
set -euo pipefail
cd "$(dirname "$0")/.."            # cwd = repo root (ycp resolves config/ + data/ from here)
YCP=".venv/bin/python -m ycp"

echo "[$(date)] sourcing today's queue…"
$YCP source || echo "  source step had issues (check creator handles)"

echo "[$(date)] capturing public views…"
$YCP capture || echo "  capture step had issues"

# If Jay exported a Whop payout CSV to data/whop-latest.csv, import it.
if [ -f data/whop-latest.csv ]; then
  echo "[$(date)] importing Whop payouts…"
  $YCP capture --whop-csv data/whop-latest.csv
fi

echo "[$(date)] done."
