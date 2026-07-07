"""Archive adapter — drive routing + local copy (no network/rclone in tests)."""
from __future__ import annotations

from ycp import archive


def test_is_rclone_routing():
    assert archive._is_rclone("phoenix:Phoenix Protocol/clips")
    assert not archive._is_rclone("/Users/x/Google Drive")
    assert not archive._is_rclone("~/Drive")
    assert not archive._is_rclone("")          # off → not a remote


def test_archive_to_local_dir_copies_clip_and_sidecar(tmp_path, monkeypatch):
    clip = tmp_path / "c1.mp4"
    clip.write_bytes(b"video-bytes")
    dest = tmp_path / "drive"
    monkeypatch.setattr(archive, "settings",
                        lambda: {"archive": {"dest": str(dest), "subfolder_by_channel": True}})
    out = archive.archive_clip(clip, {"clip_id": "c1", "channel": "phoenix-protocol", "hook": "h"})
    assert out is not None
    assert (dest / "phoenix-protocol" / "c1.mp4").read_bytes() == b"video-bytes"
    assert (dest / "phoenix-protocol" / "meta" / "c1.json").exists()   # sidecar in meta/ subfolder


def test_archive_off_returns_none(tmp_path, monkeypatch):
    clip = tmp_path / "c.mp4"
    clip.write_bytes(b"v")
    monkeypatch.setattr(archive, "settings", lambda: {"archive": {"dest": ""}})
    assert archive.archive_clip(clip, {"clip_id": "c"}) is None


def test_prune_local_removes_only_posted(tmp_path, monkeypatch):
    from ycp import db
    dbp = tmp_path / "t.db"
    clips = tmp_path / "clips"
    clips.mkdir()
    monkeypatch.setattr(archive, "DATA_DIR", tmp_path)   # clips_dir → tmp_path/"clips"
    db.init_db(dbp)
    for cid, status in (("p1", "posted"), ("q1", "pending_qc")):
        db.insert_clip({"clip_id": cid, "channel": "c", "platform": "youtube", "lane": "owned",
                        "fmt": "x", "hook_type": "q", "length_sec": 30, "status": status}, dbp)
        (clips / f"{cid}.mp4").write_bytes(b"v")
        (clips / "meta").mkdir(exist_ok=True)
        (clips / "meta" / f"{cid}.json").write_text("{}")
    removed = archive.prune_local(dbp)
    assert removed == 2                                   # p1.mp4 + meta/p1.json
    assert not (clips / "p1.mp4").exists()
    assert (clips / "q1.mp4").exists()                    # un-posted clip kept (still needs to post)
