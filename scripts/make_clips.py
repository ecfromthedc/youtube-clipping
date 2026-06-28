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
import yaml                                          # noqa: E402

from ycp import clip as clip_mod, goldmine          # noqa: E402
from ycp.db import source_queue                      # noqa: E402

TARGET = int(sys.argv[1]) if len(sys.argv) > 1 else 20
CHANNEL = "ai-frontier"
CAP = 3                                              # parallel cuts (whisper/opencv/ffmpeg are heavy)

# The source queue holds EVERY channel's roster (comedy/fitness/finance live in niches.yaml too),
# so scope to this channel's creators — and for broad interviewers who also post off-topic
# (Diary of a CEO does aliens/billionaire eps, Lex does non-AI guests), require an AI-ish title.
BROAD = {"Diary of a CEO", "Lex Fridman", "TED", "Dwarkesh Patel", "Y Combinator"}
AI_TERMS = ("ai", "a.i", "agi", "asi", "gpt", "llm", "openai", "anthropic", "claude", "gemini",
            "deepmind", "chatgpt", "neural", "robot", "automat", "superintellig", "machine learning",
            "language model", "artificial intelligence", "singularity", "nvidia", "datacenter",
            "data center", "agentic", "vibe cod", "coding", "programmer", "software")


def _roster() -> set[str]:
    """Creator names in the ai-frontier group of niches.yaml."""
    data = yaml.safe_load((ROOT / "config" / "niches.yaml").read_text())
    found: set[str] = set()

    def walk(node):
        if isinstance(node, dict):
            if node.get("name") == CHANNEL and isinstance(node.get("creators"), list):
                found.update(c.get("name") for c in node["creators"] if isinstance(c, dict))
            for v in node.values():
                walk(v)
        elif isinstance(node, list):
            for v in node:
                walk(v)
    walk(data)
    return {n for n in found if n}


def _on_topic(src: dict, roster: set[str]) -> bool:
    cr = src["creator"]
    if cr not in roster:
        return False
    if cr in BROAD:
        return any(k in (src["title"] or "").lower() for k in AI_TERMS)
    return True
LOG = ROOT / "data" / "clips" / ".make-clips.log"
PEAKS_PER_SOURCE = 3
FALLBACK_WINDOW_MIN = 8                              # no heatmap → scan the first 8 min (faster)
# Short-form AI-news creators: tiny videos = fast download+transcribe = clips land in ~2 min.
# Do these FIRST so progress is visible immediately, before the long 2-hour podcasts.
SHORT_FORM = {"Matthew Berman", "Wes Roth", "AI Explained", "bycloud", "Theo (t3.gg)",
              "ThePrimeagen"}

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
    roster = _roster()
    srcs = [s for s in source_queue(limit=200) if _on_topic(s, roster)]
    srcs.sort(key=lambda s: s["creator"] not in SHORT_FORM)   # short-form AI-news first (fast wins)
    if not srcs:
        log(f"no on-topic ({CHANNEL}) sources in the queue — run `ycp source` first.")
        return 1
    log(f"mission: {TARGET} {CHANNEL} clips · {len(srcs)} on-topic sources "
        f"(filtered from the full queue) · {CAP} parallel")
    log("  sources: " + ", ".join(f"{s['creator']}" for s in srcs[:12]))
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
