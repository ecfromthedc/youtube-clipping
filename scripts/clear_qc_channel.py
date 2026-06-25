#!/usr/bin/env python3
"""One-off: clear a Slack QC channel of the bot's own clip messages + uploaded files.

Scoped + safe by construction:
- Deletes ONLY messages authored by our own bot (chat.delete can't touch anyone
  else's messages anyway). Human messages are left alone.
- Dry-run by default; pass --yes to actually delete.
- Token is read from .env via the package config — never passed on the CLI.

Usage:
    python scripts/clear_qc_channel.py [CHANNEL_ID] [--yes]
    # CHANNEL_ID defaults to SLACK_QC_CHANNEL from .env
"""
from __future__ import annotations

import sys
import time

from slack_sdk import WebClient
from slack_sdk.errors import SlackApiError

from ycp.config import env


def _retry(fn, *a, **kw):
    # ponytail: naive 1-retry on Slack rate-limit; fine for a one-off sweep of one channel.
    try:
        return fn(*a, **kw)
    except SlackApiError as exc:
        if exc.response.get("error") == "ratelimited":
            time.sleep(int(exc.response.headers.get("Retry-After", 1)) + 1)
            return fn(*a, **kw)
        raise


def main() -> int:
    args = [a for a in sys.argv[1:] if not a.startswith("-")]
    do_it = "--yes" in sys.argv

    e = env()
    token = e["slack_bot_token"]
    if not token:
        print("SLACK_BOT_TOKEN missing in .env"); return 1
    channel = args[0] if args else e["slack_qc_channel"]
    if not channel:
        print("No channel id (pass one or set SLACK_QC_CHANNEL)"); return 1

    client = WebClient(token=token)
    me = client.auth_test()["user_id"]

    msgs, files, cursor = [], [], None
    while True:
        resp = _retry(client.conversations_history, channel=channel, limit=200, cursor=cursor)
        for m in resp.get("messages", []):
            if m.get("user") == me:                       # only our bot's own messages
                msgs.append(m["ts"])
                files += [f["id"] for f in m.get("files", []) if f.get("id")]
        cursor = resp.get("response_metadata", {}).get("next_cursor")
        if not cursor:
            break

    print(f"channel {channel}: {len(msgs)} bot message(s), {len(files)} uploaded file(s)")
    if not do_it:
        print("dry-run — re-run with --yes to delete"); return 0

    for fid in files:
        try:
            _retry(client.files_delete, file=fid)
        except SlackApiError as exc:
            print(f"  file {fid}: {exc.response.get('error')}")
    deleted = 0
    for ts in msgs:
        try:
            _retry(client.chat_delete, channel=channel, ts=ts)
            deleted += 1
        except SlackApiError as exc:
            print(f"  msg {ts}: {exc.response.get('error')}")
    print(f"deleted {deleted}/{len(msgs)} messages, {len(files)} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
