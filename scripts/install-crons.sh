#!/usr/bin/env bash
# Install the autonomous clip-factory crons as macOS launchd agents.
# Idempotent: re-run to update. Paths resolve from this repo, so the team can mirror it.
#
#   bash scripts/install-crons.sh          # install + load
#   bash scripts/install-crons.sh unload   # stop + remove
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
YCP="$REPO/.venv/bin/ycp"
AGENTS="$HOME/Library/LaunchAgents"
LOGS="$REPO/data/logs"
# Homebrew bin must be on PATH so yt-dlp / ffmpeg / whisper resolve under launchd.
BREW="$(brew --prefix 2>/dev/null || echo /opt/homebrew)/bin"
PATHENV="$BREW:/usr/bin:/bin:/usr/sbin:/sbin"

CONTENT="com.risingtides.ycp.autopilot"
WEEKLY="com.risingtides.ycp.weekly-review"
MILESTONES="com.risingtides.ycp.milestones"

if [[ "${1:-}" == "unload" ]]; then
  for lbl in "$CONTENT" "$WEEKLY" "$MILESTONES"; do
    launchctl unload "$AGENTS/$lbl.plist" 2>/dev/null || true
    rm -f "$AGENTS/$lbl.plist"
    echo "removed $lbl"
  done
  exit 0
fi

[[ -x "$YCP" ]] || { echo "ycp not found at $YCP — run scripts/setup.sh first"; exit 1; }
mkdir -p "$AGENTS" "$LOGS"

# $1=label  $2=args(space-sep)  $3=cron-block(plist XML for StartCalendarInterval)
write_plist() {
  local label="$1" args="$2" cal="$3"
  local progargs=""
  for a in $args; do progargs+="    <string>$a</string>
"; done
  cat > "$AGENTS/$label.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>$label</string>
  <key>ProgramArguments</key><array>
    <string>$YCP</string>
$progargs  </array>
  <key>WorkingDirectory</key><string>$REPO</string>
  <key>EnvironmentVariables</key><dict><key>PATH</key><string>$PATHENV</string></dict>
  <key>StandardOutPath</key><string>$LOGS/$label.out.log</string>
  <key>StandardErrorPath</key><string>$LOGS/$label.err.log</string>
$cal
</dict></plist>
PLIST
  launchctl unload "$AGENTS/$label.plist" 2>/dev/null || true
  launchctl load "$AGENTS/$label.plist"
  echo "installed + loaded $label"
}

# Content cycle: 05:00 and 13:00 daily (produces clips ahead of the 12:30/15:00/20:00 post slots).
write_plist "$CONTENT" "autopilot --max-videos 3" \
'  <key>StartCalendarInterval</key><array>
    <dict><key>Hour</key><integer>5</integer><key>Minute</key><integer>0</integer></dict>
    <dict><key>Hour</key><integer>13</integer><key>Minute</key><integer>0</integer></dict>
  </array>'

# Weekly review: Sunday 08:00 → posts the Double-Down Brief to #youtube-clipping.
write_plist "$WEEKLY" "brief --post-slack" \
'  <key>StartCalendarInterval</key><dict>
    <key>Weekday</key><integer>0</integer><key>Hour</key><integer>8</integer><key>Minute</key><integer>0</integer>
  </dict>'

# Milestone watcher: daily 09:00 → Slack alert on each new monetization threshold (YPP 500/3M, $15k/mo).
write_plist "$MILESTONES" "milestones" \
'  <key>StartCalendarInterval</key><dict>
    <key>Hour</key><integer>9</integer><key>Minute</key><integer>0</integer>
  </dict>'

echo "✓ crons installed. Logs → $LOGS/"
