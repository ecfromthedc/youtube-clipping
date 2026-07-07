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
RUNNING = CLIPS / ".refining"          # one marker file per in-flight agent
LEDGER = CLIPS / ".refine-ledger"
MAGENTA, YELLOW, DIM = "#E100C3", "#FFDE00", "grey50"
CAP = int(__import__("os").environ.get("REFINE_CAP", "3"))
SPIN = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"


def counts() -> dict[str, int]:
    return {f: len(list((CLIPS / f).glob("*.mp4"))) if (CLIPS / f).is_dir() else 0
            for f in ("unreviewed", "approved", "used", "unusable")}


def tail(path: Path, n: int) -> list[str]:
    if not path.exists():
        return []
    return [ln for ln in path.read_text(errors="ignore").splitlines() if ln.strip()][-n:]


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


def refining_markers() -> list[Path]:
    return sorted(RUNNING.glob("*"), key=lambda p: p.stat().st_mtime) if RUNNING.is_dir() else []


def now_refining(markers: list[Path], frame: int) -> Panel:
    if not markers:
        body = Align.center(Text("idle — drop a clip in unusable/ to start", style=DIM), vertical="middle")
        return Panel(body, title="[white]now refining", title_align="left", border_style=DIM, height=4)
    t = Text()
    for i, m in enumerate(markers[:6]):
        el = int(time.time() - m.stat().st_mtime)
        spin = SPIN[(frame + i * 2) % len(SPIN)]
        t.append(f" {spin} ", style=f"bold {MAGENTA}")
        t.append(f"{m.name:<14}", style="bold white")
        t.append(f"  {el // 60:d}:{el % 60:02d}  ", style=DIM)
        t.append("re-sourcing → cutting → QC\n", style=DIM)
    title = f"[white]now refining · [bold {MAGENTA}]{len(markers)}[/] in parallel (cap {CAP})"
    return Panel(t, title=title, title_align="left", border_style=MAGENTA, height=min(8, 2 + len(markers)))


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
    markers = refining_markers()
    return Group(header(bool(markers)), pipeline(counts()), now_refining(markers, frame),
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
