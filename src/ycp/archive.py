"""Archive every produced clip + its metadata to the 'Phoenix Protocol' drive.

So clips live in a central, durable library (audit + rotation) instead of piling up on
local disk — and that library is the same set of clips Postiz posts. Best-effort and
decoupled: a failed archive NEVER breaks the pipeline (the clip still posts from local).

`settings.archive.dest`:
  - ""                → off (clips stay in local data/clips/).
  - an absolute/~ path → copy there, e.g. a Google Drive for Desktop synced folder.
  - "remote:path"      → rclone copy (recommended: a Google Drive remote — headless,
                         portable, team-mirrorable). One-time: `rclone config` → Drive remote.
"""
from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path
from typing import Any

from . import db
from .config import DATA_DIR, settings
from .db import connect


def _is_rclone(dest: str) -> bool:
    """rclone remotes look like 'name:path'; local paths are absolute or ~-relative."""
    return ":" in dest and not dest.startswith(("/", "~", "."))


def archive_clip(clip_path: Path, meta: dict[str, Any]) -> str | None:
    """Copy a clip + a JSON sidecar to the configured drive. Returns the destination, or
    None when archiving is off or fails (caller treats it as best-effort)."""
    cfg = settings().get("archive", {})
    dest = (cfg.get("dest") or "").strip()
    if not dest or not clip_path.exists():
        return None
    sub = (meta.get("channel") or "clips") if cfg.get("subfolder_by_channel", True) else ""
    # Sidecar JSON lives in a `meta/` subfolder, NOT next to the mp4 — keeps data/clips/ (and the
    # drive folder) clean: just the videos, with metadata tucked aside.
    meta_dir = clip_path.parent / "meta"
    sidecar = meta_dir / f"{clip_path.stem}.json"
    try:
        meta_dir.mkdir(parents=True, exist_ok=True)
        sidecar.write_text(json.dumps(meta, indent=2, default=str))
    except OSError:
        sidecar = None
    try:
        if _is_rclone(dest):
            base = dest.rstrip("/")
            vid_target = "/".join(p for p in (base, sub) if p)
            subprocess.run(["rclone", "copy", str(clip_path), vid_target],
                           check=True, capture_output=True, timeout=300)
            if sidecar:
                subprocess.run(["rclone", "copy", str(sidecar),
                                "/".join(p for p in (base, sub, "meta") if p)],
                               check=True, capture_output=True, timeout=300)
            target = vid_target
        else:
            target_dir = Path(dest).expanduser() / sub
            target_dir.mkdir(parents=True, exist_ok=True)
            shutil.copy2(str(clip_path), str(target_dir / clip_path.name))
            if sidecar:
                (target_dir / "meta").mkdir(parents=True, exist_ok=True)
                shutil.copy2(str(sidecar), str(target_dir / "meta" / sidecar.name))
            target = str(target_dir)
        return f"{target}/{clip_path.name}"
    except (OSError, subprocess.SubprocessError):
        return None


def prune_local(db_path: Path | None = None) -> int:
    """Delete local clip files (+ sidecars) for clips already POSTED — they're live on
    YouTube and saved in the drive, so the local copy is redundant. Keeps data/clips/ from
    stacking up on the machine. Returns the number of files removed."""
    clips_dir = DATA_DIR / "clips"
    if not clips_dir.exists():
        return 0
    db.init_db(db_path)
    with connect(db_path) as conn:
        ids = [r["clip_id"] for r in
               conn.execute("SELECT clip_id FROM clips WHERE status='posted'").fetchall()]
    removed = 0
    for cid in ids:
        for f in (clips_dir / f"{cid}.mp4", clips_dir / "meta" / f"{cid}.json"):
            if f.exists():
                f.unlink()
                removed += 1
    return removed
