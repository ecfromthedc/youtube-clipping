#!/usr/bin/env python3
"""Refinement-request builder for ONE clip — the form that opens when you drop a clip.

Stack atomic ops (started early/late · ended early/late · crop · captions · hook), hit send.
Writes a job to data/clips/.refine-queue/ that refine_loop.py runs through the DETERMINISTIC
refine engine (refine.apply) — same source, same moment, your adjustments. No guessing agent.

Usage:  refine_builder.py <clip.mp4>
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from ycp import notes, refine          # noqa: E402

QUEUE = ROOT / "data" / "clips" / ".refine-queue"
MAGENTA, YELLOW, DIM, RESET = "\033[95m", "\033[93m", "\033[90m", "\033[0m"

# type -> prompt. start/end take a signed nudge ('2 earlier'/'1 later'); rest take guidance text.
COMPONENTS = [
    ("start",    "started too early / late", "how much? e.g. '2 earlier' or '1 later'"),
    ("end",      "ended too early / late",   "how much? e.g. '1 later' or '3 earlier'"),
    ("crop",     "cropping issue",           "describe the fix (e.g. 'he drifts left, keep him centred')"),
    ("captions", "captions issue",           "describe the fix"),
    ("hook",     "hook refinement",          "type the NEW hook text"),
]


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: refine_builder.py <clip.mp4>")
        return 2
    clip = Path(sys.argv[1])
    cid = notes.clip_id_for(clip)
    prov = refine.provenance(cid)

    print(f"\n{MAGENTA}⚡ REFINE{RESET}  {YELLOW}{clip.name}{RESET}")
    if prov and prov["post_title"]:
        print(f"{DIM}  hook: {prov['post_title']}{RESET}")

    # No stored source = a pre-provenance backlog clip. NEVER ask the operator for a URL — that's
    # the machine's job. It can't be re-cut, so don't waste their time: say so and close.
    if not prov or prov["source_url"] is None:
        print(f"{DIM}  no source on file for this old clip — it can't be re-cut. Skip it; the "
              f"pipeline makes fresh clips (with source baked in) far faster than salvaging this.{RESET}\n")
        return 1
    print()

    ops: list[dict] = []
    while True:
        for i, (_, label, _) in enumerate(COMPONENTS, 1):
            print(f"  [{i}] {label}")
        print(f"  {YELLOW}[s]{RESET} send   {DIM}[q] cancel{RESET}"
              + (f"   {DIM}({len(ops)} queued){RESET}" if ops else ""))
        try:
            choice = input("> ").strip().lower()
        except (EOFError, KeyboardInterrupt):
            print("\ncancelled.")
            return 1
        if choice == "q":
            print("cancelled.")
            return 1
        if choice == "s":
            if not ops:
                print(f"{DIM}  nothing added yet.{RESET}")
                continue
            break
        if choice in {"1", "2", "3", "4", "5"}:
            key, label, hint = COMPONENTS[int(choice) - 1]
            val = input(f"  {label} — {hint}\n  → ").strip()
            if val:
                ops.append({"type": key, "value": val})
                print(f"  {MAGENTA}✓{RESET} {key}: {val}\n")
            else:
                print(f"{DIM}  skipped (empty).{RESET}\n")
        else:
            print(f"{DIM}  ? pick 1-5, s, or q.{RESET}")

    QUEUE.mkdir(parents=True, exist_ok=True)
    (QUEUE / f"{cid}.json").write_text(json.dumps({"clip_id": cid, "file": str(clip), "ops": ops}))
    print(f"\n{MAGENTA}▶ SENT{RESET} — {len(ops)} op(s) queued: "
          f"{', '.join(o['type'] for o in ops)}.  Watch the dashboard.\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
