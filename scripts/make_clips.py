#!/usr/bin/env python3
"""Mission: make N clips in our format from the ranked source queue.

For each source (hottest first) it pulls the most-replayed peaks (goldmine heatmap), then cuts a
bounded clip at each peak — bounded = fast download + the moment people actually re-watch. Every
clip carries provenance (source_url + in/out), lands in unreviewed/, hook + captions burned on.
Runs a few in parallel, stops once N clips are produced.

    .venv/bin/python scripts/make_clips.py [N]      # default 20
"""
from __future__ import annotations

import sys
import threading
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
from ycp import clip as clip_mod, goldmine          # noqa: E402
from ycp.db import source_queue                      # noqa: E402

TARGET = int(sys.argv[1]) if len(sys.argv) > 1 else 20
CHANNEL = "ai-frontier"
CAP = 3                                              # parallel cuts (whisper/opencv/ffmpeg are heavy)
LOG = ROOT / "data" / "clips" / ".make-clips.log"
PEAKS_PER_SOURCE = 3
FALLBACK_WINDOW_MIN = 12                             # no heatmap → scan the first 12 min

_lock = threading.Lock()
_made = 0
_stop = threading.Event()


def log(msg: str) -> None:
    import time
    line = f"{time.strftime('%H:%M:%S')} {msg}"
    with _lock:
        with open(LOG, "a") as f:
            f.write(line + "\n")
    print(line, flush=True)


def jobs_for(src: dict) -> list[dict]:
    """A clip job per most-replayed peak (bounded window), or one first-N-min fallback job."""
    url, creator = src["url"], src["creator"]
    try:
        peaks, _ = goldmine.run(url, top=PEAKS_PER_SOURCE)
    except Exception as e:  # noqa: BLE001
        log(f"  heatmap failed for {creator}: {e}")
        peaks = []
    if peaks:
        out = []
        for pk in peaks:
            start_min = max(0.0, (pk.start - 20) / 60.0)         # 20s of lead-in before the peak
            out.append({"url": url, "creator": creator, "start_min": round(start_min, 2),
                        "window_min": 3})
        return out
    return [{"url": url, "creator": creator, "start_min": 0.0, "window_min": FALLBACK_WINDOW_MIN}]


def cut(job: dict) -> None:
    global _made
    if _stop.is_set():
        return
    try:
        created = clip_mod.run(
            job["url"], max_clips=1, source_creator=job["creator"], channel=CHANNEL,
            hook_cta=True, captions_on=True,
            start_sec=int(job["start_min"] * 60), window_sec=int(job["window_min"] * 60))
    except Exception as e:  # noqa: BLE001 — one bad source must not kill the mission
        log(f"✗ {job['creator']} @ {job['start_min']:.1f}m: {e}")
        return
    with _lock:
        for c in created:
            _made += 1
            log(f"✓ [{_made}/{TARGET}] {job['creator']}  {Path(c['file']).name}  “{c.get('post_title') or ''}”")
        if _made >= TARGET:
            _stop.set()


def main() -> int:
    srcs = source_queue(limit=60)
    if not srcs:
        log("no sources in the queue — run `ycp source` first.")
        return 1
    log(f"mission: {TARGET} clips in our format · {len(srcs)} sources · {CAP} parallel")
    pool = ThreadPoolExecutor(max_workers=CAP)
    seen_urls = set()
    for src in srcs:
        if _stop.is_set():
            break
        if src["url"] in seen_urls:
            continue
        seen_urls.add(src["url"])
        for job in jobs_for(src):
            if _stop.is_set():
                break
            pool.submit(cut, job)
    pool.shutdown(wait=True)
    log(f"done — {_made} clips in data/clips/unreviewed/")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
