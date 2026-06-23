# CLAUDE.md — read this first

This file is auto-loaded into every Claude Code session in this repo. It's the 60-second
orientation. The full cold-start brief is **[HANDOFF.md](HANDOFF.md)** — read it before doing
real work.

## The assignment
Build + run a **hands-off, closed-loop clip factory**: take big-creator videos → cut vertical
captioned clips → post them across **owned** faceless channels → monetize the owned stack
(YouTube Partner Program + TikTok Creator Rewards + affiliate + later brand deals).
**North star: $15,000/month**, durable, mostly hands-off — Eric taps ✅ in Slack and reads the
scoreboard; the system runs the channels.

## Start here (read order)
1. **HANDOFF.md** — current state, what's built vs. not, what to build next (§8), decisions
   already made (§9), guardrails (§10).
2. **WIN-GOAL-AND-LOOP.md** — the operative build loop. Paste its `/loop` prompt to run a cycle.
   (It supersedes the older `*-GOAL-AND-LOOP.md` docs, kept as references.)
3. **SOURCE-INTELLIGENCE.md** / **CHANNEL-CONCEPTS.md** — what to clip / which channels to launch.

## Run + verify
```bash
./scripts/setup.sh                      # first time, and after ANY src/ycp edit (see gotcha)
.venv/bin/python -m ycp demo            # closed loop, no creds — prints a Double-Down Brief
.venv/bin/python -m ycp scoreboard      # real Day-0 game state
.venv/bin/python -m ycp source          # ranked clip queue (live yt-dlp, ~2–3 min)
.venv/bin/python -m ycp autopilot       # chain the whole daily loop end-to-end
```
**Verify gates — nothing is "done" until both pass:** `ruff check src tests` clean ·
`pytest -q` green (**72 on a real machine**; the `cut_vertical` ffmpeg smoke only hangs under an
agent sandbox, not on a real Mac).

**Install gotcha:** the package is installed **non-editable** (the build's history has a
spaced-path quirk). After editing `src/ycp/`, re-run `./scripts/setup.sh` or the `ycp` command
won't see your change. `pytest` *does* reflect edits without reinstall (via pyproject `pythonpath`).

## Commit discipline — IMPORTANT (multiple agents run here)
Other `/loop` sessions may edit and commit this repo **at the same time as you.**
- **Stage only the files you changed, by explicit path:** `git add path/a path/b`.
- **NEVER `git add -A`, `git add .`, or `git commit -am`** — a blanket add bundles another
  session's in-progress work into your commit under the wrong message. (This has happened.)
- If `git status --porcelain` lists files you didn't touch, **leave them** — don't commit, stash,
  or `git checkout` them.
- One conventional commit, scoped to your change, then push.

A `pre-commit` hook enforces this (blocks the code-plus-many-docs "sweep" shape); `./scripts/setup.sh`
installs it. Intentional mixed commit? `YCP_ALLOW_MIXED=1 git commit ...`.

## Decisions already made — don't relitigate (HANDOFF §9)
Owned-first, **single** revenue lane · **Whop cut entirely** · posting via **Repurpose.io** (thin,
swappable adapter; Eric connects accounts once) · **QC is manual** in Slack until trust is earned
(`qc.auto: false`) · hook titles via **DeepSeek** (key in 1Password, transcript-heuristic fallback).
Launch order: **Hot Seat** (debate) + **Money Fights** (finance).

## Guardrails — non-negotiable (these keep channels alive; HANDOFF §10)
Transform every owned-channel clip · **no copyrighted music** (instant Content-ID) · respect the
avoid-list as a *gate that runs before the score* · agitation attacks the position, never a
protected group/person · **channel health > raw output** · real numbers only · verify before "done."

## Where state lives
The **SQLite DB is the source of truth** for the closed loop. `SCOREBOARD.md` and the briefs are
**auto-generated** (`ycp scoreboard` / `ycp brief`) — don't hand-edit them. Secrets live in `.env`
(gitignored) and 1Password, **never in code**.
