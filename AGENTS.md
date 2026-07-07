# youtube-clipping — Agent Guide

> ⛔ **SHARED POSTIZ ACCOUNT — NEVER TOUCH ANOTHER CHANNEL.** This Postiz account can hold
> multiple team integrations. Every Postiz operation — posting AND any cleanup/delete — MUST be
> filtered to the explicitly mapped integration id for the active owned channel. **NEVER delete or
> act on Postiz posts by state** (e.g. "delete all QUEUE") — that hits teammates' scheduled
> content. The team's other accounts run independently.

This is the 60-second orientation for any agent working in this repo. The full cold-start
brief is **[HANDOFF.md](HANDOFF.md)** — read it before doing real work.

## The assignment
Build + run a **hands-off, closed-loop clip factory**: take big-creator videos → cut vertical
captioned clips → post them across **owned** faceless channels → monetize the owned stack
(YouTube Partner Program + TikTok Creator Rewards + affiliate + later brand deals).
**North star: $15,000/month**, durable, mostly hands-off — Eric taps ✅ in Slack and reads the
scoreboard; the system runs the channels.

## Start here (read order)
0. **STATUS.md** — ⭐ READ FIRST. Living snapshot of where we are right now, what just happened,
   the operating config, and the forward plan. The single clearest current-state doc.
1. **HANDOFF.md** — original cold-start brief: what's built vs. not, decisions made (§9), guardrails (§10).
2. **WIN-GOAL-AND-LOOP.md** — the operative build loop. Paste its `/loop` prompt to run a cycle.
   (It supersedes the older `*-GOAL-AND-LOOP.md` docs, kept as references.)
3. **CHANNEL-PLAYBOOK.md** — one card per channel: the sources to clip, the format, the go/no-go
   gate, and the metric each channel optimizes on. Each channel is a path one agent owns + optimizes.
   (Backed by `SOURCE-INTELLIGENCE.md` = source scores + `CHANNEL-CONCEPTS.md` = angle rationale.)
4. **RESEARCH-shorts-2026.md** + **CONTENT-STRATEGY.md** — **COME CORRECT: read before any content
   decision.** RESEARCH = the 2026 Shorts evidence base + the doctrine (research-first, ground every
   call in data — not vibes); CONTENT-STRATEGY = that research applied (formats, hooks, length,
   posting windows, monetization). Don't ship guesses; start from the evidence, let the loop tune it.

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
Owned-first, **single** revenue lane · **Whop cut entirely** · posting via **Postiz** (public API,
preferred — we hold the token; **Repurpose.io** is the swappable alternative — see DISTRIBUTION.md) ·
**QC is manual** in Slack until trust is earned
(`qc.auto: false`) · hook titles via **DeepSeek** (key in 1Password, transcript-heuristic fallback).
Launch order is reset. Pick one channel, map one Postiz integration, then run the gate.

## Guardrails — non-negotiable (these keep channels alive; HANDOFF §10)
Transform every owned-channel clip · **no copyrighted music** (instant Content-ID) · respect the
avoid-list as a *gate that runs before the score* · agitation attacks the position, never a
protected group/person · **channel health > raw output** · real numbers only · verify before "done."

## Where state lives
The **SQLite DB is the source of truth** for the closed loop. `SCOREBOARD.md` and the briefs are
**auto-generated** (`ycp scoreboard` / `ycp brief`) — don't hand-edit them. Secrets live in `.env`
(gitignored) and 1Password, **never in code**.

## Agents: running the templates & formats (Tides Tiller API)

The Tiller's render formats are agent-runnable end-to-end over plain HTTP — no browser needed.
**Start with `GET /api/formats`** (live server, default `ycp serve --port 8788`): it returns the
machine-readable registry of every format (clip · ranking · story · commentary) with params,
examples, the project workflow, and the guardrails. It is drift-tested (`rust/src/formats.rs`) —
if it says a route exists, it exists.

```bash
BASE=http://localhost:8788
curl -s $BASE/api/formats | jq '.formats[].format'        # discover
# clip + ranking need a transcribed project:
ID=$(curl -s -X POST $BASE/api/projects -H 'Content-Type: application/json' \
     -d '{"filename":"raw.mp4"}' | jq -r .id)
curl -s -X POST $BASE/api/projects/$ID/upload -F file=@raw.mp4
curl -s -X POST $BASE/api/projects/$ID/transcribe          # → ranked candidates[]
curl -s -X POST $BASE/api/projects/$ID/render \
     -H 'Content-Type: application/json' -d '{"start":12.4,"end":41.0,"title":"hook here"}'
# story / commentary are standalone (OmniVoice must be up for VO — check GET /api/voices):
curl -s -X POST $BASE/api/studio/render -H 'Content-Type: application/json' \
     -d '{"format":"story","script":"…","background":"data/backgrounds/minecraft.mp4"}'
```

Rules of engagement: studio renders **serialize server-side** (one OmniVoice call at a time —
don't parallelize studio calls); rendered files come back as `/api/projects/:id/files/...`
download paths; **publishing** (`/api/postiz/publish`) is behind the SHARED-POSTIZ guardrail at
the top of this file — integration id only, never by state. The in-page copilot (✨ bead, Ctrl+/)
drives the same surfaces from inside the UI and its capabilities are drift-tested too
(`rust/src/actions.rs`).
