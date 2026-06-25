"""YouTube write/admin ops — currently just video DELETE (clean up our own mistakes).

Separated from read-only capture.py so the DESTRUCTIVE path is explicit + logged. Needs the
youtube.force-ssl scope — re-auth via scripts/yt_oauth.py (read-only creds → 403 here, handled).
Always deletes a SPECIFIC id passed in (never a blind sweep), and logs every deletion.
"""
from __future__ import annotations

from . import capture


def _yt():
    creds = capture._yt_creds()
    if creds is None:
        return None
    from googleapiclient.discovery import build
    return build("youtube", "v3", credentials=creds)


def delete_video(video_id: str, our_channel: str | None = None, yt=None) -> bool:
    """Delete ONE video — ONLY if it's on OUR channel. Returns True on success. Logs it.

    Safety: Postiz release ids can span multiple channels; we verify ownership before issuing
    a destructive delete so we never touch another channel's content (YouTube 403s anyway).
    """
    yt = yt or _yt()
    if yt is None:
        print("[yt-delete] no YouTube creds — run scripts/yt_oauth.py")
        return False
    mine = our_channel or yt.channels().list(part="id", mine=True).execute()["items"][0]["id"]
    items = yt.videos().list(part="snippet", id=video_id).execute().get("items", [])
    if not items:
        print(f"[yt-delete] ✗ {video_id}: not found")
        return False
    if items[0]["snippet"]["channelId"] != mine:
        print(f"[yt-delete] ✗ {video_id}: not on our channel — skipped (safety)")
        return False
    try:
        yt.videos().delete(id=video_id).execute()
        print(f"[yt-delete] ✓ removed {video_id}")
        return True
    except Exception as exc:  # noqa: BLE001
        print(f"[yt-delete] ✗ FAILED {video_id}: {str(exc)[:160]}")
        return False


def delete_videos(video_ids: list[str]) -> int:
    """Delete several videos (ours only). Resolves our channel once. Returns count removed."""
    yt = _yt()
    if yt is None:
        print("[yt-delete] no YouTube creds — run scripts/yt_oauth.py")
        return 0
    mine = yt.channels().list(part="id", mine=True).execute()["items"][0]["id"]
    return sum(1 for v in video_ids if delete_video(v, our_channel=mine, yt=yt))
