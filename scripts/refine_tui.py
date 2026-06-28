#!/usr/bin/env python3
"""Live TUI for the refinement loop — folder pipeline, active refine, activity feed.

Read-only dashboard over data/clips/ + the watcher log. Run it in its own kitty window:
    kitty @ launch --type=os-window --title "Refine Loop" .venv/bin/python scripts/refine_tui.py
"""
from __future__ import annotations

import time
from pathlib import Path

from rich.align import Align
from rich.console import Group
from rich.live import Live
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

ROOT = Path(__file__).resolve().parent.parent
CLIPS = ROOT / "data" / "clips"
LOG = CLIPS / ".refine-watch.log"
LOCK = CLIPS / ".refine-lock"
LEDGER = CLIPS / ".refine-ledger"
MAGENTA, YELLOW, DIM = "#E100C3", "#FFDE00", "grey50"
STAGES = (("queued", "grey50"), ("re-sourcing", "cyan"), ("cutting", YELLOW),
          ("QC gate", "magenta"), ("done", "green"))
SPIN = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"


def counts() -> dict[str, int]:
    return {f: len(list((CLIPS / f).glob("*.mp4"))) if (CLIPS / f).is_dir() else 0
            for f in ("unreviewed", "approved", "used", "unusable")}


def tail(path: Path, n: int) -> list[str]:
    if not path.exists():
        return []
    return [ln for ln in path.read_text(errors="ignore").splitlines() if ln.strip()][-n:]


def stage_from(line: str) -> int:
    low = line.lower()
    if "produced" in low or "→ unreviewed" in low or "done" in low:
        return 4
    if "gemini" in low or "qc" in low or "reject" in low:
        return 3
    if "ycp clip" in low or "download" in low or "whisper" in low or "cutting" in low:
        return 2
    if "websearch" in low or "goldmine" in low or "re-sourc" in low or "refining" in low:
        return 1
    return 0


def header(active: bool) -> Panel:
    dot = Text("● refining", style=f"bold {MAGENTA}") if active else Text("● watching", style="bold green")
    t = Text("  ⚡ AI FRONTIER", style=f"bold {YELLOW}")
    t.append("  ·  Refinement Loop      ", style="bold white")
    t.append(dot)
    t.append("\n  unusable/  →  agent re-sources + re-cuts  →  unreviewed/", style=DIM)
    return Panel(t, border_style=MAGENTA, padding=(0, 1))


def pipeline(c: dict[str, int]) -> Panel:
    hi = max(1, max(c.values()))
    rows = Table.grid(padding=(0, 1))
    rows.add_column(justify="right", width=11)
    rows.add_column()
    colors = {"unreviewed": YELLOW, "approved": "green", "used": "cyan", "unusable": "red"}
    for k in ("unreviewed", "approved", "used", "unusable"):
        bar = "█" * round(c[k] / hi * 34)
        rows.add_row(Text(k, style=colors[k]), Text(f"{bar} {c[k]}", style=colors[k]))
    return Panel(rows, title="[white]pipeline", title_align="left", border_style=DIM, padding=(1, 1))


def now_refining(active: bool, lines: list[str], frame: int) -> Panel:
    if not active:
        body = Align.center(Text("idle — drop a clip in unusable/ to start", style=DIM), vertical="middle")
        return Panel(body, title="[white]now refining", title_align="left", border_style=DIM, height=5)
    clip = next((ln.split("refining:", 1)[1].strip() for ln in reversed(lines) if "refining:" in ln), "?")
    elapsed = int(time.time() - LOCK.stat().st_mtime) if LOCK.exists() else 0
    si = max((stage_from(ln) for ln in lines[-6:]), default=0)
    spin = SPIN[frame % len(SPIN)]
    line = Text(f" {spin} ", style=f"bold {MAGENTA}")
    line.append(f"{clip}", style="bold white")
    line.append(f"   {elapsed // 60:d}:{elapsed % 60:02d}\n\n ", style=DIM)
    for i, (name, col) in enumerate(STAGES):
        done, cur = i < si, i == si
        mark = "●" if done else ("◉" if cur else "○")
        style = ("green" if done else (f"bold {col}" if cur else DIM))
        line.append(f"{mark} {name}", style=style)
        line.append("  →  " if i < len(STAGES) - 1 else "", style=DIM)
    return Panel(line, title="[white]now refining", title_align="left", border_style=MAGENTA, height=5)


def activity(lines: list[str]) -> Panel:
    t = Text()
    for ln in lines[-9:]:
        ts, _, rest = ln.partition(" ")
        t.append(f"{ts} ", style=DIM)
        t.append(rest[:90] + "\n", style="white")
    return Panel(t or Text("(no activity yet)", style=DIM), title="[white]activity",
                title_align="left", border_style=DIM)


def footer() -> Text:
    n = len([x for x in LEDGER.read_text().splitlines() if x.strip()]) if LEDGER.exists() else 0
    t = Text(f"  ledger: {n} attempted", style=DIM)
    t.append("          ", style=DIM)
    t.append("ctrl-c / q to quit", style=DIM)
    return t


def render(frame: int):
    lines = tail(LOG, 40)
    active = LOCK.exists()
    return Group(header(active), pipeline(counts()), now_refining(active, lines, frame),
                 activity(lines), footer())


def main() -> None:
    with Live(render(0), refresh_per_second=8, screen=True) as live:
        frame = 0
        while True:
            time.sleep(0.12)
            frame += 1
            live.update(render(frame))


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        pass
