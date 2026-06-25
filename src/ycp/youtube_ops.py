"""YouTube write/admin ops — currently just video DELETE (clean up our own mistakes).

Separated from read-only capture.py so the DESTRUCTIVE path is explicit + logged. Needs the
youtube.force-ssl scope — re-auth via scripts/yt_oauth.py (read-only creds → 403 here, handled).
Always deletes a SPECIFIC id passed in (never a blind sweep), and logs every deletion.
"""
from __future__ import annotations

from . import capture


def delete_video(video_id: str) -> bool:
    """Delete ONE YouTube video the channel owns, by id. Returns True on success. Logs it."""
    creds = capture._yt_creds()
    if creds is None:
        print("[yt-delete] no YouTube creds — run scripts/yt_oauth.py")
        return False
    try:
        from googleapiclient.discovery import build
    except ImportError:
        print("[yt-delete] google-api-python-client missing")
        return False
    yt = build("youtube", "v3", credentials=creds)
    try:
        yt.videos().delete(id=video_id).execute()
        print(f"[yt-delete] ✓ removed video {video_id}")
        return True
    except Exception as exc:  # noqa: BLE001
        msg = str(exc)[:200]
        hint = " (need write scope — re-run scripts/yt_oauth.py)" if "insufficient" in msg.lower() \
            or "403" in msg else ""
        print(f"[yt-delete] ✗ FAILED {video_id}: {msg}{hint}")
        return False


def delete_videos(video_ids: list[str]) -> int:
    """Delete several videos by id. Returns count removed."""
    return sum(1 for v in video_ids if delete_video(v))
