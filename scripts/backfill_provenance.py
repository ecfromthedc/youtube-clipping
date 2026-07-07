#!/usr/bin/env python3
"""Backfill clip provenance the old pipeline never persisted, from the paper trail:
  1. session transcripts  — exact `ycp clip <url> --start --window` command per clip_id (precise)
  2. AI-NEWS-SOURCING.md   — hook -> video id + rough timestamp (covers autopilot-made clips)

Writes source_url / clip_start / clip_end onto clips that lack them, so the refine loop can
re-cut them. Idempotent: skips clips that already have a source_url. Run:
    .venv/bin/python scripts/backfill_provenance.py            # report only
    .venv/bin/python scripts/backfill_provenance.py --write    # write to the DB
"""
from __future__ import annotations

import difflib
import glob
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from ycp.db import connect                       # noqa: E402

TRANSCRIPTS = str(Path.home() / ".claude/projects/-Users-risingtidesdev-youtube-clipping/*.jsonl")
SOURCING = ROOT / "AI-NEWS-SOURCING.md"
YT = "https://www.youtube.com/watch?v="


def _norm(s: str) -> str:
    return re.sub(r"[^a-z0-9 ]", "", (s or "").lower()).strip()


def from_transcripts() -> dict[str, dict]:
    """clip_id -> {url, start_s, win_s, len} from the exact ycp clip commands (most precise)."""
    id2cmd, id2out = {}, {}
    for f in glob.glob(TRANSCRIPTS):
        for ln in open(f, errors="ignore"):
            try:
                d = json.loads(ln)
            except json.JSONDecodeError:
                continue
            content = (d.get("message") or {}).get("content")
            tid = None
            if isinstance(content, list):
                for b in content:
                    if not isinstance(b, dict):
                        continue
                    if b.get("type") == "tool_use" and (b.get("input") or {}).get("command"):
                        id2cmd[b["id"]] = b["input"]["command"]
                    if b.get("type") == "tool_result":
                        tid = b.get("tool_use_id")
                        c = b.get("content")
                        if isinstance(c, list):
                            c = " ".join(x.get("text", "") for x in c if isinstance(x, dict))
                        if tid and c:
                            id2out[tid] = id2out.get(tid, "") + str(c)
            tr = d.get("toolUseResult")
            if isinstance(tr, dict) and tr.get("stdout") and tid:
                id2out[tid] = id2out.get(tid, "") + tr["stdout"]

    url_re = re.compile(r"(https?://(?:www\.)?(?:youtube\.com/watch\?v=|youtu\.be/)[A-Za-z0-9_-]+)")
    start_re, win_re = re.compile(r"--start\s+([0-9.]+)"), re.compile(r"--window\s+([0-9.]+)")
    cid_re = re.compile(r"·\s+([0-9a-f]{8}-\d\d)\s+([0-9.]+)s")
    arch_re = re.compile(r"Phoenix Protocol/([0-9a-f]{8}-\d\d)\.mp4")
    out: dict[str, dict] = {}
    for tid, cmd in id2cmd.items():
        if "ycp clip" not in cmd:
            continue
        u = url_re.search(cmd)
        if not u:
            continue
        body = id2out.get(tid, "")
        start_s = float((start_re.search(cmd) or [0, "0"])[1]) * 60
        win = win_re.search(cmd)
        win_s = float(win.group(1)) * 60 if win else None
        for m in list(cid_re.finditer(body)) or [None]:
            if m is None:
                for a in arch_re.finditer(body):
                    out.setdefault(a.group(1), {"url": u.group(1), "start_s": round(start_s, 2),
                                                "win_s": win_s, "len": None})
                continue
            out[m.group(1)] = {"url": u.group(1), "start_s": round(start_s, 2),
                               "win_s": win_s, "len": float(m.group(2))}
    return out


def sourcing_hooks() -> list[dict]:
    """[{vid, hooks:[norm...], ts_s}] parsed from the sourcing table."""
    rows = []
    for ln in open(SOURCING):
        if ln.count("|") < 6:
            continue
        cells = [c.strip(" `") for c in ln.strip().strip("|").split("|")]
        ids = re.findall(r"\b([A-Za-z0-9_-]{11})\b", ln)
        # real YouTube IDs mix case/digits/_/-; a pure-lowercase-letters token is a handle (@ycombinator)
        ids = [x for x in ids if not (x.isalpha() and x.islower())]
        if not ids:
            continue
        ts_cell = next((c for c in cells if re.search(r"\d+:\d+|cold|open|first|min|intro", c.lower())), "")
        rows.append({"vid": ids[0], "hooks": [_norm(c) for c in cells if len(c) > 8], "ts_s": _ts(ts_cell)})
    return rows


def _ts(s: str) -> int:
    s = s.lower()
    if any(w in s for w in ("cold-open", "opening", "first", "0:00", "~0", "intro")):
        return 0
    m = re.search(r"(\d+):(\d+):(\d+)", s)
    if m:
        return int(m[1]) * 3600 + int(m[2]) * 60 + int(m[3])
    m = re.search(r"(\d+):(\d+)", s)
    return int(m[1]) * 60 + int(m[2]) if m else 0


def main() -> int:
    write = "--write" in sys.argv
    tx = from_transcripts()
    hooks = sourcing_hooks()
    with connect() as c:
        clips = [dict(r) for r in c.execute(
            "select clip_id, post_title, length_sec, source_url from clips")]

    plan = []
    for cl in clips:
        cid = cl["clip_id"]
        if cl["source_url"]:
            continue
        length = cl["length_sec"] or 38
        if cid in tx:                                   # precise: transcript command
            r = tx[cid]
            end = r["start_s"] + (r["len"] or r["win_s"] or length)
            plan.append((cid, r["url"], r["start_s"], round(end, 2), "transcript", cl["post_title"]))
            continue
        t = _norm(cl["post_title"])                     # fuzzy: sourcing hook
        if not t:
            continue
        best, score = None, 0.0
        for row in hooks:
            for h in row["hooks"]:
                s = 1.0 if (t == h or (len(t) > 10 and (t in h or h in t))) else \
                    difflib.SequenceMatcher(None, t, h).ratio()
                if s > score:
                    best, score = row, s
        if best and score >= 0.6:
            start = best["ts_s"]
            plan.append((cid, YT + best["vid"], float(start), float(start + length),
                         f"sourcing/{score:.2f}", cl["post_title"]))

    print(f"{len(plan)} clips recoverable (of {sum(1 for c in clips if not c['source_url'])} missing source):\n")
    for cid, url, st, en, how, title in sorted(plan):
        print(f"  {cid}  [{how:>13}]  {url.split('=')[-1]}  {st:.0f}-{en:.0f}s  | {title}")

    if write:
        with connect() as c:
            for cid, url, st, en, _, _ in plan:
                c.execute("update clips set source_url=?, clip_start=?, clip_end=? where clip_id=?",
                          (url, st, en, cid))
        print(f"\n✓ wrote provenance for {len(plan)} clips")
    else:
        print("\n(report only — re-run with --write to persist)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
