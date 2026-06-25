# DIAL-GOAL — my operator loop prompt (I re-enter this each cycle until done)

**GOAL:** The YouTube clipping build is *provably* flawless — every box in `QA-CHECKLIST.md` is
`- [x]` with real evidence, `DIALED-DONE` is written, and I have *personally* verified the hard
items (rendered frames look right, autopilot runs clean end-to-end in a sandbox, the Rust render
matches Python). I own getting this to done — the headless Ralph loop is a worker, not the owner.

## Each cycle
1. **ASSESS.** Read `QA-CHECKLIST.md` (checked vs unchecked), `tail data/logs/ralph-dial.log`,
   `git log --oneline -12`, and whether `DIALED-DONE` exists + the dial loop (`ba28hvexi`) is alive.
   **Safety first:** confirm NO live post happened (distribution must be sandbox/mock only). If one
   did → stop the loop, purge it, fix the cause before anything else.
2. **DECIDE & ACT:**
   - **If `DIALED-DONE` exists** → don't trust it blind. Spot-check 3 evidence claims myself
     (re-run a gate, render a clip, Read a Rust frame *and* the Python frame). All solid → report
     to Eric with the filled checklist + before/after frame, stop the loop, **DONE**. Any bogus
     claim → uncheck it, keep going.
   - **If the headless loop is converging** (boxes checking off with real evidence, `pytest` green,
     commits landing) → let it work; verify a couple of its claims; re-arm.
   - **If it's STUCK / spinning** — especially the **Rust title-render** item (a fresh agent
     eyeballing pixel layout is the weak spot) → **take over hands-on**: diff `rust/src/captions.rs`
     title path against `src/ycp/captions.py`, fix the wrap/size/position math myself, `cargo build`,
     render one clip, Read the frame, iterate until the title matches Python. My visual judgment
     beats a fresh headless agent's here.
3. **GUARDRAILS (every change):** `ruff` clean + `pytest` green; NEVER post live; commit by
   explicit path (never `git add -A`); `src/ycp` is LIVE — fix forward; Rust work stays in `rust/`.
4. **LOOP:** not done → re-arm `ScheduleWakeup` with THIS prompt (`/loop` it). Interval: ~1200–1500s
   while monitoring; shorter if I'm actively hands-on fixing. **Done → report + `PushNotification` +
   stop (no re-arm).**

## What "done" looks like
Every QA-CHECKLIST box ✅ with evidence · a Rust-rendered frame that's indistinguishable from
Python's · a clean sandbox autopilot run (9/9, ≤ `max_per_run`, no broken clips) · `DIALED-DONE`
written · Eric has the recap. Then — and only then — stop.
