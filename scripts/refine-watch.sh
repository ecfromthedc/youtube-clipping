#!/usr/bin/env bash
# Refinement loop — watch data/clips/unusable/ and auto-fix NOTED clips.
#
# Flow: drop ONE clip in unusable/ → TextEdit opens with a note template → write why it's
# bad → Cmd+S → a headless Claude agent re-sources a real talking-head moment, re-cuts via
# `ycp clip` (auto-framed + QC-gated), and the good result lands in unreviewed/.
#
# Safe by design:
#   - Clips ALREADY in the folder at startup are baselined as "seen" — NO editor opens for them
#     (this is the bug that spammed before). Only clips dropped AFTER startup pop an editor, once.
#   - A clip gets exactly one editor (its `.note.txt` sidecar marks it seen; orphan sidecars are
#     removed safely when their video leaves).
#   - Only clips with a real (non-template) note are sent to the agent; a ledger blocks retries;
#     a lock prevents concurrent agent runs. Operator-started; uses tokens per refine.
#
# Use:  empty unusable/ first if you like, then:  bash scripts/refine-watch.sh   (Ctrl-C stops)
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$(pwd)"
WATCH="$ROOT/data/clips/unusable"
LEDGER="$ROOT/data/clips/.refine-ledger"
RUNNING="$ROOT/data/clips/.refining"      # one marker file per in-flight agent
LOG="$ROOT/data/clips/.refine-watch.log"
CAP="${REFINE_CAP:-3}"                     # max agents refining at once (rate-limit guard)
mkdir -p "$WATCH" "$RUNNING"; touch "$LEDGER"
command -v fswatch >/dev/null || { echo "✗ fswatch not found (brew install fswatch)"; exit 1; }
command -v claude  >/dev/null || { echo "✗ claude CLI not found"; exit 1; }

_id() { local b; b="$(basename "$1" .mp4)"; printf '%s' "${b%% -- *}"; }

cleanup_orphans() {   # remove .note.txt whose video is gone — safe (no glob-error deletes)
  for sc in "$WATCH"/*.note.txt; do
    [ -e "$sc" ] || continue
    local cid hit=""; cid="$(basename "$sc" .note.txt)"
    for m in "$WATCH"/*.mp4; do [ -e "$m" ] || continue; [ "$(_id "$m")" = "$cid" ] && { hit=1; break; }; done
    [ -n "$hit" ] || rm -f "$sc"
  done
}

baseline() {          # mark every CURRENT clip seen WITHOUT opening an editor (no startup spam)
  for m in "$WATCH"/*.mp4; do
    [ -e "$m" ] || continue
    case "$(basename "$m")" in *" -- "*) continue ;; esac
    local sc="$WATCH/$(_id "$m").note.txt"; [ -f "$sc" ] || : > "$sc"
  done
}

open_new_editors() {  # a clip with no sidecar = newly dropped → template + ONE TextEdit
  for m in "$WATCH"/*.mp4; do
    [ -e "$m" ] || continue
    case "$(basename "$m")" in *" -- "*) continue ;; esac
    local sc="$WATCH/$(_id "$m").note.txt"
    [ -f "$sc" ] && continue
    printf '# Why is this clip wrong? Write your note here, then press Cmd+S.\n# e.g. "shows the interviewer, not the guest" / "hook not related" / "cuts off before the point"\n\n' > "$sc"
    open -e "$sc"
    echo "$(date '+%T') note editor opened → $(_id "$m")" | tee -a "$LOG"
  done
}

refine() {            # clips with a real note, not yet attempted → run the agent once
  local pending
  pending=$(.venv/bin/python -m ycp notes 2>/dev/null | sed -n 's/^  \([^ ]*\)  →.*/\1/p' \
            | grep -vxF -f "$LEDGER" || true)
  [ -z "$pending" ] && return 0
  for cid in $pending; do
    [ -f "$RUNNING/$cid" ] && continue                       # already has an agent
    while [ "$(find "$RUNNING" -maxdepth 1 -type f | wc -l | tr -d ' ')" -ge "$CAP" ]; do
      sleep 3                                                # at the cap — wait for a slot
    done
    spawn_one "$cid"
  done
}

spawn_one() {            # launch ONE background agent for a single clip (parallel up to CAP)
  local cid="$1"
  touch "$RUNNING/$cid"
  echo "$(date '+%T') refining: $cid" | tee -a "$LOG"
  ( claude -p "Refine ONE broken AI-news clip ($cid) for the factory at $ROOT (cwd). Read its note
with '.venv/bin/python -m ycp notes'. Then: (1) the note says why it's bad; (2) delete the old
clip + its .note.txt sidecar + its DB row; (3) re-source a clip where the SUBJECT is a close-up
talking head actually saying the point (use the note; WebSearch / 'ycp goldmine <url>' for the
moment; NEVER Joe Rogan/JRE); (4) re-cut: .venv/bin/python -m ycp clip \"<url>\" --max 1
--start <MIN> --window <~1.2> --creator \"<who>\" --channel ai-frontier --title \"<hook>\"
(auto-frames/trims/QC-gates); (5) confirm it routed to data/clips/unreviewed/. Max 2 attempts.
Be concise." --dangerously-skip-permissions </dev/null >>"$LOG" 2>&1 \
        || echo "$(date '+%T') agent error $cid" >>"$LOG"
    echo "$cid" >> "$LEDGER"            # mark attempted so failures don't loop
    rm -f "$RUNNING/$cid"
    echo "$(date '+%T') done $cid" >>"$LOG"
  ) &
}

echo "👀 watching $WATCH — drop ONE clip, note it in TextEdit, Cmd+S.  (Ctrl-C to stop)  log: $LOG"
cleanup_orphans
baseline                                   # existing clips: seen, no editor
refine                                     # process anything already noted via filename suffix
fswatch -o "$WATCH" | while read -r _; do
  sleep 2                                  # debounce Finder's burst of write events
  cleanup_orphans
  open_new_editors
  refine
done
