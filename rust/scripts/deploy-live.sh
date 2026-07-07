#!/usr/bin/env bash
# Deploy the Tiller server to the TCC-safe live dir and bounce it.
#
# WHY A DEPLOY COPY: the live service must survive reboots, but launchd/cron
# hang any unsigned binary whose path/cwd sits under ~/Documents (dyld blocks
# on the TCC check before main() — diagnosed 2026-07-07 via `sample`: stack
# stuck in dyld getCWD → open). Everything the server needs is copied to
# ~/Projects/active/tidestiller-live — zero Documents access at runtime — so
# the cron keepalive (Desktop/automations/tidestiller-keepalive.sh) can boot
# it headlessly, forever. Run this after every UI or server change.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
LIVE="$HOME/Projects/active/tidestiller-live"

# Fresh build first (trunk → dist, cargo → binary).
"$REPO/rust/scripts/build-ui.sh"

mkdir -p "$LIVE/data/logs" "$LIVE/rust/ui" "$LIVE/config"
cp "$REPO/rust/target/release/ycp" "$LIVE/ycp.new" && mv "$LIVE/ycp.new" "$LIVE/ycp"
rsync -a --delete "$REPO/rust/ui/dist/" "$LIVE/rust/ui/dist/"
rsync -a "$REPO/config/" "$LIVE/config/"
# Secrets + analytics OAuth ride along (both gitignored in the repo).
cp "$REPO/.env" "$LIVE/.env" 2>/dev/null || true
[ -d "$REPO/data/oauth" ] && rsync -a "$REPO/data/oauth/" "$LIVE/data/oauth/"
# Seed editor projects once (live data then lives its own ephemeral life).
[ -d "$LIVE/data/editor" ] || { [ -d "$REPO/data/editor" ] && rsync -a "$REPO/data/editor/" "$LIVE/data/editor/"; }

pkill -f "ycp serve --port 8788" 2>/dev/null || true
sleep 1
cd "$LIVE"
nohup ./ycp serve --port 8788 >> "$LIVE/data/logs/tiller-serve.log" 2>&1 &
sleep 2
curl -sf -m 5 http://localhost:8788/api/health >/dev/null && echo "✓ tidestiller live from $LIVE" || {
    echo "✗ health check failed after deploy" >&2
    exit 1
}
