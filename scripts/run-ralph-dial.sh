#!/usr/bin/env bash
# Ralph loop — dial the WHOLE build to flawless (DIAL-PROMPT.md + QA-CHECKLIST.md).
# Fresh Claude each iteration verifies/fixes one checklist item with evidence; repo = memory.
# Done-gate = the agent writes DIALED-DONE once every box is ✅. NEVER posts live (sandbox/mock).
#
#   bash scripts/run-ralph-dial.sh
set -uo pipefail
cd "$(dirname "$0")/.."
mkdir -p data/logs
LOG="data/logs/ralph-dial.log"
rm -f DIALED-DONE   # fresh run

CLAUDE_BIN="${CLAUDE_BIN:-/opt/homebrew/bin/claude}"
run_claude() {
  ALL_PROXY= HTTPS_PROXY= HTTP_PROXY= all_proxy= https_proxy= http_proxy= \
    "$CLAUDE_BIN" "$@"
}

ALLOW=(--permission-mode acceptEdits --allowedTools
  Edit Write Read Glob Grep
  "Bash(cd:*)" "Bash(ls:*)" "Bash(cat:*)" "Bash(grep:*)" "Bash(rg:*)" "Bash(find:*)" "Bash(sed:*)" "Bash(echo:*)" "Bash(mkdir:*)" "Bash(cp:*)" "Bash(ln:*)" "Bash(test:*)"
  "Bash(cargo:*)" "Bash(rustc:*)" "Bash(ffmpeg:*)" "Bash(ffprobe:*)" "Bash(yt-dlp:*)"
  "Bash(./scripts/setup.sh)" "Bash(bash scripts/setup.sh)"
  "Bash(git add:*)" "Bash(git commit:*)" "Bash(git push:*)" "Bash(git status:*)" "Bash(git diff:*)" "Bash(git log:*)"
  "Bash(rust/target/debug/ycp:*)" "Bash(rust/target/release/ycp:*)"
  "Bash(.venv/bin/python:*)" "Bash(.venv/bin/python3:*)" "Bash(.venv/bin/ycp:*)" "Bash(.venv/bin/pytest:*)" "Bash(.venv/bin/ruff:*)")

echo "=== Ralph (dial-the-build) started $(date) ===" | tee -a "$LOG"
for i in $(seq 1 50); do
  if [ -f DIALED-DONE ]; then
    echo "✓ Ralph: build DIALED after $((i-1)) iterations $(date) — $(cat DIALED-DONE)" | tee -a "$LOG"
    break
  fi
  echo "── iteration $i  $(date +%H:%M:%S) ──" | tee -a "$LOG"
  run_claude -p "$(cat DIAL-PROMPT.md)" "${ALLOW[@]}" >>"$LOG" 2>&1 \
    || echo "(iteration $i exited non-zero)" | tee -a "$LOG"
  sleep 3
done
[ -f DIALED-DONE ] || echo "⚠ Ralph stopped without DIALED-DONE — check $LOG + QA-CHECKLIST.md" | tee -a "$LOG"
echo "=== Ralph (dial-the-build) stopped $(date) ===" | tee -a "$LOG"
