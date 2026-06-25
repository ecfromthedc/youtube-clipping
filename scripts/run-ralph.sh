#!/usr/bin/env bash
# Ralph loop — feed rust/PROMPT.md to a FRESH Claude each iteration until the Rust port is done.
# Stateless by design: the repo is the memory (Python = spec, rust/README.md = progress,
# `cargo build` = the gate, git = history). Survives this terminal staying open; Ctrl-C to stop.
#
#   bash scripts/run-ralph.sh
set -uo pipefail
cd "$(dirname "$0")/.."
mkdir -p data/logs
LOG="data/logs/ralph.log"

# `claude` is an interactive alias (clears proxy vars); call the real binary the same way.
CLAUDE_BIN="${CLAUDE_BIN:-/opt/homebrew/bin/claude}"
run_claude() {
  ALL_PROXY= HTTPS_PROXY= HTTP_PROXY= all_proxy= https_proxy= http_proxy= \
    "$CLAUDE_BIN" "$@"
}

# Scoped allowlist so a headless run can edit + build + commit unattended — WITHOUT the banned
# --dangerously-skip-permissions. If an iteration reports a denied command, add it here.
ALLOW=(--permission-mode acceptEdits --allowedTools
  Edit Write Read Glob Grep
  "Bash(cd:*)" "Bash(ls:*)" "Bash(cat:*)" "Bash(grep:*)" "Bash(rg:*)" "Bash(find:*)" "Bash(sed:*)" "Bash(echo:*)" "Bash(mkdir:*)"
  "Bash(cargo:*)" "Bash(rustc:*)"
  "Bash(git add:*)" "Bash(git commit:*)" "Bash(git push:*)" "Bash(git status:*)" "Bash(git diff:*)" "Bash(git log:*)"
  "Bash(rust/target/debug/ycp:*)" "Bash(rust/target/release/ycp:*)"
  "Bash(.venv/bin/python:*)" "Bash(.venv/bin/python3:*)")

echo "=== Ralph started $(date) ===" | tee -a "$LOG"
for i in $(seq 1 40); do
  # Done-gate: no pending modules AND a green release build → stop.
  if ! grep -q '⏳' rust/README.md && (cd rust && cargo build --release -q >/dev/null 2>&1); then
    echo "✓ Ralph: port COMPLETE after $((i-1)) iterations $(date)" | tee -a "$LOG"
    break
  fi
  echo "── iteration $i  $(date +%H:%M:%S) ──" | tee -a "$LOG"
  run_claude -p "$(cat rust/PROMPT.md)" "${ALLOW[@]}" >>"$LOG" 2>&1 \
    || echo "(iteration $i exited non-zero)" | tee -a "$LOG"
  sleep 3
done
echo "=== Ralph stopped $(date) ===" | tee -a "$LOG"
