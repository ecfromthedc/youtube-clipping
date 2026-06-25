#!/usr/bin/env python3
"""One-time YouTube OAuth → refresh token for the Phoenix Protocol channel.

Run this ONCE, interactively (it opens a browser for you to authorize the channel's
Google account). It then writes YT_CLIENT_ID / YT_CLIENT_SECRET / YT_REFRESH_TOKEN /
YT_CHANNEL_ID into .env so the pipeline can pull this channel's own analytics
(retention + revenue) — the data a public API key can't see.

    ! .venv/bin/python scripts/yt_oauth.py [path/to/client_secret*.json]

If no path is given, it grabs the newest client_secret*.json from ~/Downloads.
Nothing secret is printed; creds go straight to .env.
"""
from __future__ import annotations

import glob
import os
import sys
from pathlib import Path

from google_auth_oauthlib.flow import InstalledAppFlow
from googleapiclient.discovery import build

ROOT = Path(__file__).resolve().parents[1]
ENV_PATH = ROOT / ".env"
SCOPES = [
    "https://www.googleapis.com/auth/youtube.readonly",
    "https://www.googleapis.com/auth/yt-analytics.readonly",
    "https://www.googleapis.com/auth/yt-analytics-monetary.readonly",
]


def _find_client_json() -> str:
    if len(sys.argv) > 1:
        return sys.argv[1]
    hits = glob.glob(str(Path.home() / "Downloads" / "client_secret*.json"))
    if not hits:
        sys.exit("No client_secret*.json found in ~/Downloads — pass the path as an argument.")
    return max(hits, key=os.path.getmtime)


def _upsert_env(values: dict[str, str]) -> None:
    """Set/replace KEY=VALUE lines in .env without disturbing the rest. Never logs values."""
    lines = ENV_PATH.read_text().splitlines() if ENV_PATH.exists() else []
    out, seen = [], set()
    for line in lines:
        key = line.split("=", 1)[0].strip() if "=" in line else ""
        if key in values:
            out.append(f"{key}={values[key]}")
            seen.add(key)
        else:
            out.append(line)
    for key, val in values.items():
        if key not in seen:
            out.append(f"{key}={val}")
    ENV_PATH.write_text("\n".join(out) + "\n")


def main() -> int:
    client_json = _find_client_json()
    flow = InstalledAppFlow.from_client_secrets_file(client_json, scopes=SCOPES)
    # access_type=offline + prompt=consent guarantees a refresh_token comes back.
    creds = flow.run_local_server(port=0, access_type="offline", prompt="consent")
    if not creds.refresh_token:
        return print("No refresh token returned — re-run; ensure you fully consent.") or 1

    yt = build("youtube", "v3", credentials=creds)
    ch = yt.channels().list(part="snippet,statistics", mine=True).execute()
    items = ch.get("items") or []
    if not items:
        return print("Authorized, but this account owns no channel.") or 1
    c = items[0]
    _upsert_env({
        "YT_CLIENT_ID": creds.client_id,
        "YT_CLIENT_SECRET": creds.client_secret,
        "YT_REFRESH_TOKEN": creds.refresh_token,
        "YT_CHANNEL_ID": c["id"],
    })
    print(f"✓ Authorized: {c['snippet']['title']} ({c['id']})")
    print(f"  subs={c['statistics'].get('subscriberCount','?')} "
          f"views={c['statistics'].get('viewCount','?')}")
    print("  Wrote YT_CLIENT_ID / YT_CLIENT_SECRET / YT_REFRESH_TOKEN / YT_CHANNEL_ID to .env")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
