# events.md — repository devlog ledger

Single append-only chronological ledger for this repo, shared by all agents/harnesses/worktrees.
Newest entries at the bottom. Entry schema: time / agent / worktree (when not on main) / type / area,
then one plain-prose paragraph.

---
time:      [4:16pm] [07-04-26]
agent:     [claude] [opus 4.8]
worktree:  claude/sleepy-rosalind-a28c25
type:      [feature-request]
area:      [backend]

Drove a dev cycle on the clip pipeline after a 5-agent recon fan-out. Shipped four changes:
(1) a transcript term-map in `transcribe.py` that corrects Whisper mis-hearings of AI proper
nouns (entropic→Anthropic, open ai→OpenAI, etc.) — one map fixes both the hook and the burned
captions since both derive from `Segment.text`; (2) wired `sourcing.run()` into `make_clips.py`
so the mission refreshes the source queue instead of chewing a frozen list, plus a
`clipped_source_ids()` exclusion so runs advance to fresh videos; (3) **active-speaker tracking**
in `reframe.py` — the "supervision" stage now follows who is TALKING (lip/mouth-motion scored
against Whisper speech intervals), not just who is on-screen most, pure OpenCV, no new deps;
(4) an operator-pointed exact cut `ycp clip <url> --from MM:SS --to MM:SS`. All verified: ruff
clean, 156 tests pass.
_________________________________________________________________________________

time:      [4:16pm] [07-04-26]
agent:     [claude] [opus 4.8]
worktree:  claude/sleepy-rosalind-a28c25
type:      [bug-report]
area:      [infra]

Live smoke revealed the real blocker to the whole factory: **yt-dlp was never a managed
dependency** — the pipeline shelled out to whatever `yt-dlp` was on PATH, which had rotted to a
>90-day-old build and silently HUNG a run for ~11.8 hours (blocked on I/O, zero clips). Fixed:
added `yt-dlp>=2026.6.9` to pyproject deps and an upgrade step to `setup.sh`, so the downloader
stays fresh. Also fixed a latent crash where `make_clips.py` opened its log before creating
`data/clips/`. After the fix, a bounded 1-clip smoke produced a clean, gate-passed clip
end-to-end (My First Million, 38s) with stable active-speaker framing. NOTE for future agents:
"supervision" = the OpenCV speaker-tracking stage in `reframe.py` (see its docstring), NOT the
Roboflow library — the term lived nowhere durable, which is why fresh sessions never recognized
it. FOLLOW-UP: add subprocess timeouts to the yt-dlp/download calls so a stall can't hang for
hours regardless of version.
_________________________________________________________________________________
