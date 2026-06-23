# 🏁 WIN — Goal & Loop (the master build loop)

> **Paste the [/loop prompt](#4-the-loop-prompt) into `/loop` to run the engine.**
> One *verified* build per cycle until the clip factory ships clips that win and runs
> hands-off toward **$15K/month**. Operative loop for the current push; supersedes the
> older `*-GOAL-AND-LOOP.md` docs (kept as references). Read `HANDOFF.md` first.

---

## 1. North star
**$15,000/month**, durable, from **owned faceless YouTube/TikTok channels**, on a
**closed-loop automated** clip pipeline that feeds its own wins back in and hardens week
over week. Eric is mainly hands-off — he taps **approve** in Slack and reads the
**scoreboard**. Pure owned-first: a single revenue lane (YouTube Partner + TikTok Creator
Rewards + affiliate + later brand deals). The channels are the asset.

## 2. Current state (2026-06-23)
- `ycp` pipeline end-to-end: `source -> clip -> transcribe -> hooks -> captions -> QC ->
  distribute -> capture -> brief -> scoreboard`. **66 tests green, ruff clean.**
- `ycp source` works — **52 real videos queued**.
- Transcription ready (**whisper.cpp + base.en**). **DeepSeek hook agent VERIFIED LIVE**
  (key loaded; returns strong typed candidates).
- **Captions + hook titles SHIPPED (this session):** opus-style word-by-word captions
  (3-word chunks, active word highlighted yellow, fat outline) + the DeepSeek title now
  render on every clip — drawn with **Pillow** and composited via the ffmpeg **`overlay`**
  filter (this ffmpeg has no libass/freetype). **Self-contained, no network on the critical
  path.** Proven on a real render. The caption-less era is over.
- **Not live yet:** zero channels. Distribution (Repurpose.io) + channel accounts need
  Eric's **one-time connect**. Game state: **Day 0, Level 1, $0.**

## 3. The winning workflow
```
source -> clip(trim + reframe) -> transcribe(whisper.cpp) -> hooks(DeepSeek)
      -> captions(opus PNG overlay, LOCAL) -> [opt: color-correct . Pruna 9:16 b-roll]
      -> QC(Slack: manual->auto) -> distribute(Repurpose outbox) -> capture -> brief -> scoreboard
```
**`ycp` keeps the brain AND the render local** — ranked clip selection, DeepSeek hooks,
compliance guardrails, and now caption/title burn-in (Pillow + ffmpeg `overlay`). The owned
**Content Posting Lab** is an **OPTIONAL** enhancer (color-correct, AI vertical b-roll),
flag-gated, never on the critical path.

## 4. THE /loop PROMPT
*(paste everything in this block into `/loop`)*

```
You are hardening the youtube-clipping operation toward its North Star: $15K/month from
owned faceless channels on a hands-off, closed-loop clip factory. Work in
~/Documents/Development/youtube-clipping.

Each cycle:
1. Re-read WIN-GOAL-AND-LOOP.md (section 5 backlog), HANDOFF.md, and SCOREBOARD.md. Pick the
   SINGLE highest-leverage item — the one that most moves "clips that win" or "runs hands-off
   reliably." Prefer the lowest-numbered unfinished section-5 item unless a sharper bottleneck exists.
2. Build exactly that — one focused, verified change. Use TDD where it fits.
3. GATE (no build is "done" until all pass):
     - uv run pytest -q        -> green
     - uv run ruff check .      -> clean
     - if it touches the pipeline/editing: run a REAL clip through it end-to-end and eyeball
       the output (vertical 1080x1920, hook title + captions burned, audio intact).
4. Log the cycle to LOOP-LOG.md: what / why / result / the next bottleneck you see. Update
   SCOREBOARD.md if state changed.
5. Stop. Surface to Eric anything that needs his accounts or a decision (channel connect,
   Repurpose auth, flipping QC manual->auto).

Non-negotiable constraints:
- Transform every owned-channel clip (your cut/hook/captions) — raw reuploads get the channel
  demonetized channel-wide. No copyrighted music. Respect the avoid-list (SOURCE-INTELLIGENCE.md)
  as a gate. Agitation attacks the position, never a protected group/person. Channel health >
  raw output. Real numbers only. Owned-first; don't relitigate decided strategy (HANDOFF.md).
```

## 5. Build backlog (highest-leverage first)
1. ✅ **DONE — Captions + hook titles resurrected.** `captions.py` renders opus-style
   word-by-word captions + the DeepSeek title as a transparent overlay (Pillow), composited
   with ffmpeg `overlay`. No libass needed. *The #1 quality lever — shipped + render-verified.*
2. ✅ **DONE — Opus word-by-word styling.** 3-word chunks, 0.8s min dwell, non-overlapping,
   active-word yellow highlight, fat black outline. Built into `captions.py`.
3. **End-to-end real-clip gate.** Run a real YouTube source through `ycp clip`
   (download -> whisper -> hook -> caption) and eyeball; wire as the standing production gate.
   (Editing layer + DeepSeek already proven on a synthetic render + live API this session.)
4. **Face-pan reframe.** Port `clipify`'s speaker-tracking (`analyze.py` + `build_pan.py`)
   to replace center-crop. Next big retention win.
5. **`content_lab.py` adapter (OPTIONAL).** Color-correct + Pruna 9:16 AI b-roll behind
   flags — owned RT infra (https://risingtides-content-lab-production.up.railway.app). Not
   on the critical path. (Add a shared-secret header before leaning on it — it's currently open.)
6. **Autopilot orchestrator hardening** — one command/cron chains the full daily loop.
7. **QC auto-flip criteria** — define the track record that earns `qc.auto: true`.
8. **Channel launch** — Concept 1 "Hot Seat" + Concept 2 "Money Fights" (needs Eric's accounts).
9. **Weekly compounding** — the Double-Down Brief re-ranks sources/angles on real results.

## 6. Verification gates
Done only when: `uv run pytest -q` green · `uv run ruff check .` clean · `uv run ycp demo`
prints a sane Double-Down Brief · and anything touching the pipeline is proven on a **real
clip**, not just unit tests.

## 7. Scorecard — Race to $15K
`uv run ycp scoreboard` renders the live game from the DB. Day-0 is Level 1, $0. The loop
climbs it: First Blood -> Signal -> Traction -> ... -> $15K/mo, hands-off.
