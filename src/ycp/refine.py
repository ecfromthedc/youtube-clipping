"""Atomic refinement — re-cut a clip's EXACT moment with adjustments, never re-source.

A refinement request is a list of components, each {type, value}:
  start   value = seconds to shift the IN point  (negative = earlier, positive = later)
  end     value = seconds to shift the OUT point (negative = earlier, positive = later)
  crop    value = guidance text (the pipeline re-frames the same moment)
  captions value = guidance text (re-rendered, current style)
  hook    value = the new hook text

All components collapse into ONE re-cut of the same source URL at the (possibly nudged) in/out —
so "the crop is bad" fixes the crop on the clip we already have, instead of sourcing a new video.
Needs the clip's stored provenance (source_url + clip_start/clip_end); clips cut before provenance
tracking can't be re-cut this way and need a re-source.
"""
from __future__ import annotations

import re
from pathlib import Path

from . import db
from .db import connect

MIN_LEN = 3.0   # a re-cut shorter than this is almost certainly a mistake


def _secs(value) -> float:
    """Signed seconds from a number or text ('-2', '2 earlier', '1.5 later')."""
    if isinstance(value, (int, float)):
        return float(value)
    s = str(value or "")
    m = re.search(r"-?\d+(?:\.\d+)?", s)
    n = float(m.group()) if m else 0.0
    low = s.lower()
    if any(w in low for w in ("earl", "sooner", "back")):
        n = -abs(n)
    elif "late" in low:
        n = abs(n)
    return n


def provenance(clip_id: str, db_path: Path | None = None):
    db.init_db(db_path)
    with connect(db_path) as c:
        return c.execute(
            "SELECT source_url, clip_start, clip_end, source_creator, channel, post_title "
            "FROM clips WHERE clip_id=?", (clip_id,)).fetchone()


def pin(clip_id: str, url: str, start: float, end: float, *, creator: str | None = None,
        title: str | None = None, channel: str | None = None, db_path: Path | None = None) -> dict:
    """Pin provenance onto a clip that predates tracking (the backlog salvage path): paste the
    source URL + in/out and it becomes re-cuttable. Updates the row if present, else inserts a
    minimal one (so the loop can apply ops right after)."""
    db.init_db(db_path)
    with connect(db_path) as c:
        exists = c.execute("SELECT 1 FROM clips WHERE clip_id=?", (clip_id,)).fetchone()
        if exists:
            c.execute("UPDATE clips SET source_url=?, clip_start=?, clip_end=? WHERE clip_id=?",
                      (url, round(start, 2), round(end, 2), clip_id))
            return {"ok": True, "inserted": False}
    db.insert_clip({"clip_id": clip_id, "channel": channel or "ai-frontier", "platform": "youtube",
                    "lane": "owned", "fmt": None, "hook_type": None, "length_sec": None,
                    "source_creator": creator, "post_title": title,
                    "source_url": url, "clip_start": round(start, 2), "clip_end": round(end, 2)},
                   db_path)
    return {"ok": True, "inserted": True}


def plan(clip_id: str, ops: list[dict], db_path: Path | None = None) -> dict:
    """Resolve a refinement request to a concrete re-cut spec (pure given provenance). Returns
    {ok, url, start, end, title, creator, channel, notes} or {ok: False, reason}."""
    r = provenance(clip_id, db_path)
    if not r or not r["source_url"] or r["clip_start"] is None:
        return {"ok": False, "reason": "this clip has no stored source/timestamp (cut before "
                "provenance tracking) — it needs a re-source, not a re-cut"}
    start, end, title = float(r["clip_start"]), float(r["clip_end"]), r["post_title"]
    notes: list[str] = []
    for o in ops:
        t, v = str(o.get("type", "")).lower(), o.get("value")
        if t in ("start", "started"):
            start += _secs(v)
        elif t in ("end", "ended"):
            end += _secs(v)
        elif t == "hook":
            title = str(v).strip() or title
        else:                                   # crop / captions / other → guidance, same bounds
            notes.append(f"{t}: {v}")
    if end - start < MIN_LEN:
        return {"ok": False, "reason": f"those adjustments leave a <{MIN_LEN:g}s clip"}
    return {"ok": True, "url": r["source_url"], "start": round(start, 2), "end": round(end, 2),
            "title": title, "creator": r["source_creator"] or "unknown",
            "channel": r["channel"] or "ai-frontier", "notes": notes}


def apply(clip_id: str, ops: list[dict], db_path: Path | None = None) -> dict:
    """Apply a refinement request: ONE re-cut of the same moment with the adjustments."""
    from . import clip as clip_mod
    spec = plan(clip_id, ops, db_path)
    if not spec.get("ok"):
        return spec
    created = clip_mod.run(spec["url"], max_clips=1, source_creator=spec["creator"],
                           channel=spec["channel"], title=spec["title"],
                           exact=(spec["start"], spec["end"]), db_path=db_path)
    if not created:
        return {"ok": False, "reason": "re-cut produced no clip"}
    return {"ok": True, "from": clip_id, "clip_id": created[0]["clip_id"],
            "file": created[0]["file"], "bounds": [spec["start"], spec["end"]],
            "title": spec["title"], "applied": [o.get("type") for o in ops]}


if __name__ == "__main__":   # pure-logic self-check (no db/cut)
    assert _secs(-2) == -2.0
    assert _secs("2 earlier") == -2.0
    assert _secs("1.5 later") == 1.5
    print("refine self-check OK")
