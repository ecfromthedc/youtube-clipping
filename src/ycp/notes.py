"""Per-clip review notes — so the operator can tell the fix loop WHY a clip is bad, from Finder.

Two zero-friction ways to attach a note to a clip, both read here:
  1. Rename the file with a ` -- ` suffix:  `f2ad2e30-00 -- shows lex not jensen.mp4`
     (Finder: select, Return, type. Reliable — no permissions, no indexing.)
  2. macOS Finder "Comments" (Get Info / ⌘I → Comments). Read via `mdls`.

`note_for` returns the note (filename suffix wins over a Finder comment). `clip_id_for` strips
the suffix so the id still resolves. `collect` walks a folder → [(clip_id, note, path)].
"""
from __future__ import annotations

import subprocess
from pathlib import Path

SEP = " -- "


def clip_id_for(path: Path) -> str:
    """The clip id, ignoring any ` -- note` the operator appended to the filename."""
    return path.stem.split(SEP, 1)[0].strip()


def _finder_comment(path: Path) -> str:
    """macOS Finder comment via Spotlight metadata; '' if none/unavailable (non-mac)."""
    try:
        out = subprocess.run(["mdls", "-name", "kMDItemFinderComment", "-raw", str(path)],
                             capture_output=True, text=True, timeout=10).stdout.strip()
        return "" if out in ("(null)", "") else out
    except (OSError, subprocess.SubprocessError):
        return ""


def sidecar_for(path: Path) -> Path:
    """The `<clip_id>.note.txt` sidecar a TextEdit window writes into."""
    return path.parent / f"{clip_id_for(path)}.note.txt"


def note_for(path: Path) -> str:
    """The operator's note for a clip. Priority: the `.note.txt` sidecar (what the drop-to-edit
    flow writes), then a filename ` -- suffix`, then a macOS Finder comment. '#' lines in the
    sidecar are treated as the template prompt and ignored."""
    sc = sidecar_for(path)
    if sc.exists():
        body = "\n".join(ln for ln in sc.read_text().splitlines()
                         if not ln.lstrip().startswith("#")).strip()
        if body:
            return body
    if SEP in path.stem:
        return path.stem.split(SEP, 1)[1].strip()
    return _finder_comment(path)


def collect(folder: Path) -> list[tuple[str, str, Path]]:
    """[(clip_id, note, path)] for every .mp4 in `folder` that carries a note. Best-effort."""
    out: list[tuple[str, str, Path]] = []
    for p in sorted(folder.glob("*.mp4")):
        note = note_for(p)
        if note:
            out.append((clip_id_for(p), note, p))
    return out


if __name__ == "__main__":  # tiny self-check (pure filename parsing, no I/O)
    assert clip_id_for(Path("ab12cd34-00 -- shows the host.mp4")) == "ab12cd34-00"
    assert note_for(Path("ab12cd34-00 -- hook is wrong.mp4")) == "hook is wrong"
    assert clip_id_for(Path("ab12cd34-00.mp4")) == "ab12cd34-00"
    print("notes self-check OK")
