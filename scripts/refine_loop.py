#!/usr/bin/env python3
"""Refinement loop — drop a clip in unusable/, a builder form opens, your request runs through
the DETERMINISTIC refine engine (same source + moment, your adjustments). No guessing agents.

Flow:
  drop clip in data/clips/unusable/  ->  kitty builder window opens (refine_builder.py)
  pick atomic ops, hit send          ->  job lands in .refine-queue/
  this loop runs refine.apply        ->  re-cut moment lands in unreviewed/, dashboard shows it
  drop another mid-job               ->  another builder opens; jobs process in parallel (cap)

Run it:  .venv/bin/python scripts/refine_loop.py     (Ctrl-C stops)
Needs kitty remote control on (it opens the builder + dashboard in kitty OS-windows).
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from ycp import refine                  # noqa: E402

CLIPS = ROOT / "data" / "clips"
WATCH = CLIPS / "unusable"
QUEUE = CLIPS / ".refine-queue"
RUNNING = CLIPS / ".refining"           # markers the dashboard reads (one per in-flight job)
LOG = CLIPS / ".refine-watch.log"
LEDGER = CLIPS / ".refine-ledger"
PY = str(ROOT / ".venv" / "bin" / "python")
CAP = int(os.environ.get("REFINE_CAP", "3"))


def log(msg: str) -> None:
    line = f"{time.strftime('%H:%M:%S')} {msg}"
    with open(LOG, "a") as f:
        f.write(line + "\n")
    print(line, flush=True)


def kitty(title: str, *cmd: str) -> bool:
    """Open a new kitty OS-window running cmd. False if kitty remote isn't reachable."""
    for launcher in (["kitten", "@"], ["kitty", "@"]):
        try:
            r = subprocess.run([*launcher, "launch", "--type=os-window", "--title", title, *cmd],
                               capture_output=True, timeout=10)
            if r.returncode == 0:
                return True
        except (OSError, subprocess.SubprocessError):
            continue
    return False


def open_builder(clip: Path) -> None:
    if not kitty(f"refine · {clip.stem}", PY, str(ROOT / "scripts" / "refine_builder.py"), str(clip)):
        log(f"✗ couldn't open builder window for {clip.name} (is kitty remote control on?)")


def process(job_path: Path) -> None:
    job = json.loads(job_path.read_text())
    cid, ops = job["clip_id"], job["ops"]
    marker = RUNNING / cid
    marker.write_text(" ".join(o["type"] for o in ops))
    job_path.unlink(missing_ok=True)
    log(f"▶ refining {cid}  ops={[o['type'] for o in ops]}")
    try:
        if job.get("pin"):                          # backlog salvage: pin pasted source first
            p = job["pin"]
            refine.pin(cid, p["url"], p["start"], p["end"], creator=p.get("creator"),
                       title=p.get("title"), channel=p.get("channel"))
            log(f"  pinned source for {cid}: {p['url'].split('=')[-1]} @ {p['start']:.0f}s")
        res = refine.apply(cid, ops)
        if res.get("ok"):
            log(f"✓ {cid} → {Path(res['file']).name}  bounds={res['bounds']}")
        else:
            log(f"✗ {cid}: {res.get('reason')}")
        with open(LEDGER, "a") as f:
            f.write(f"{time.strftime('%H:%M:%S')} {cid} ok={res.get('ok')}\n")
    except Exception as e:  # noqa: BLE001 — one bad job must not kill the loop
        log(f"✗ {cid} crashed: {e}")
    finally:
        marker.unlink(missing_ok=True)


def main() -> int:
    for d in (WATCH, QUEUE, RUNNING):
        d.mkdir(parents=True, exist_ok=True)
    LEDGER.touch()
    # Baseline: clips already in unusable/ are NOT reopened — only clips dropped AFTER start
    # pop a builder (this is the spam bug we already fixed once; keep it fixed).
    seen = {p.name for p in WATCH.glob("*.mp4")}
    log(f"watching {WATCH}  ({len(seen)} baselined, won't reopen)  cap={CAP}")
    kitty("Refine Loop", PY, str(ROOT / "scripts" / "refine_tui.py"))   # dashboard window

    pool = ThreadPoolExecutor(max_workers=CAP)
    try:
        while True:
            cur = {p.name for p in WATCH.glob("*.mp4")}
            for name in sorted(cur - seen):
                log(f"＋ dropped {name} → opening builder")
                open_builder(WATCH / name)
            seen = cur
            for job in QUEUE.glob("*.json"):
                taken = job.with_suffix(".taken")
                try:
                    job.rename(taken)          # claim it atomically (no double-submit)
                except OSError:
                    continue
                pool.submit(process, taken)
            time.sleep(1.0)
    except KeyboardInterrupt:
        log("stopped.")
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
