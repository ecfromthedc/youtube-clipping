# Loop Log — optimization toward "any operator clears $10K/month"

Append-only. Newest cycle on top. Each entry: what changed · evidence · new bottleneck.
The loop reads this first every cycle (see `GOAL-AND-LOOP.md`).

---

## 📊 Operator economics scorecard (target: $10K/operator/month)

| | Value | Notes |
|---|---|---|
| Path to $10K | **Owned stack**: TikTok Creator Rewards + YPP + affiliate + brand deals across owned channels | Pure YouTube Shorts ad rev alone would need ~200M+ views/mo — the owned stack gets there at a fraction. |
| Current operators live | 0 | System built; no one running it yet. |
| Biggest lever right now | **Account infra + niches** | Volume engine + Ssemble-parity now owned (cycles 1–2). Next gate is real channels + `niches.yaml`. |
| Gap to close next | Finalize `config/niches.yaml` + face-tracking/translation cycles | Niche research pending; parity features 3–4 (face-track, translate) still to build. |

---

## Cycle 2 — Ssemble parity: own the output (Hook/CTA + Gameplay)

**Diagnosed lens:** dependency/ceiling — Eric flagged not wanting to depend on Ssemble.
**Shipped (`src/ycp/enhance.py` + wired into `ycp clip`):**
- **Hook Title & CTA** → ffmpeg `drawtext` top banner (title auto-picked from transcript)
  + timed CTA banner. Replaces Ssemble's "Hook Title & CTA". `--hook-cta`.
- **Game Video** → ffmpeg `vstack` of the clip over a looping gameplay file. Replaces
  Ssemble's "Game Video". `--gameplay <path>`.
- `SSEMBLE-PARITY.md` maps all 6 Ssemble features → owned local tools (yt-dlp/ffmpeg/
  whisper/Ollama/OpenCV). Captioning + gameplay at parity now; face-track + translate next.
**Evidence:** ruff clean · `pytest` 25 passed (6 new enhance builder tests: drawtext escape,
title/cta filters, vstack command, title heuristic) · `ycp clip --help` shows all flags.
**New bottleneck:** face tracking (OpenCV) + caption translation (Ollama) — parity cycles 3–4.

## Cycle 1 — Hybrid clip pipeline (break the Ssemble credit ceiling)

**Diagnosed lens:** THROUGHPUT — Ssemble's ~7 clips/day can't feed 75–100 posts/day.
**Shipped (`src/ycp/clip.py`, `src/ycp/srt.py`, `ycp clip`):** yt-dlp download → Whisper
transcribe → rank candidate moments by a hook heuristic → ffmpeg cut to 9:16 with burned
captions → register as `pending_qc`. Free, local, uncapped. The volume engine.
**Evidence:** ruff clean · `pytest` 25 passed (srt parse/slice/roundtrip, clip planning,
scoring heuristic; ffmpeg reframe smoke deselected — sandbox ffmpeg hangs, validate on the
real machine via `pytest -k cut_vertical`).
**Hardening surfaced during verify (fixed):** spaced folder name ("Youtube Clipping Workflow")
broke hatchling's *editable* install → switched to **non-editable install** + **CWD-based
project-root resolution** (`config._find_root`) so config/data resolve from the repo, not
site-packages. Added `scripts/setup.sh` (reproducible), `__main__.py` (`python -m ycp`), and
pointed cron scripts at `.venv/bin/python -m ycp`. Also: a `kill -9` during a parallel install
corrupted `config.py` mid-write — restored from git (git is the safety net; avoid `-9` during installs).
**New bottleneck:** owning the *remaining* Ssemble features (→ cycle 2).

## Cycle 0 — Baseline (system built + verified)

**What exists** (all in one repo root now: `~/Desktop/Development/Youtube Clipping Workflow/`):
- Strategy + math: `YOUTUBE-CLIPPING-WORKFLOW.md`, `LAUNCH-CHECKLIST.md`
- Live system: the `ycp` CLI with stages: `source` (yt-dlp ranked queue, no key needed) ·
  `qc-post`/`qc-listen` (Slack approval) · `capture` (public views) ·
  `brief` (deterministic Double-Down Brief).
- Turnkey runbook: `OPERATOR-PLAYBOOK.md` (seeded v1).
- Self-hardening engine: `GOAL-AND-LOOP.md`.

**Evidence it works (verified this cycle):**
- `ruff check src tests` → All checks passed.
- `pytest` → 13 passed (scoring math, DB lifecycle, sourcing parser/ranker).
- `ycp demo` → produced a correct brief: crowned `Flagrant·debate-moment·question` (score
  97.6) as 🟢 Scale, killed `RandomVlogger·reaction·pattern-interrupt` (8.1), and reported
  ad revenue by platform on identical clips — the closed loop, demonstrated in numbers.

**Known gaps / failure modes not yet guarded (work for upcoming cycles):**
1. `config/niches.yaml` not finalized — sourcing has no real creator list yet (research landing).
2. Full analytics (retention/RPM/ad revenue) needs YouTube Analytics OAuth — stubbed honestly;
   public views cover the early loop.
3. Slack QC needs a Slack app (bot+app tokens, Socket Mode) — code complete, creds not set.
4. No real operator has run the playbook end-to-end — operability unproven against a human.
5. Ssemble credit ceiling (~7/day) vs volume targets — hybrid pipeline documented, not yet
   scripted as a one-command flow.
6. Distribution (Stage 4) is a documented Ssemble handoff — no `ycp` export of approved clips yet.

**Next cycle target:** finalize `niches.yaml` from the niche research, then pick the single
highest-leverage gap above (likely #1 or #4) and close it fully with verification.

---

## Cycle — 2026-06-23 · Captions + hook titles resurrected (self-contained)
**What:** Built `src/ycp/captions.py` — opus-style word-by-word captions + the DeepSeek
hook title, rendered with Pillow and composited via the ffmpeg `overlay` filter. Rewired
`clip.py` (`cut_vertical` is now clean scale/crop; `run()` burns captions + title with a
graceful plain-clip fallback). Added `tests/test_captions.py` + updated `tests/test_clip.py`.
**Why:** This ffmpeg has no libass/freetype, so the old `subtitles=`/drawtext burn silently
dropped EVERY caption + title — the single biggest quality gap (the hook is the #1 virality
lever). Pillow→overlay sidesteps libass entirely and keeps the critical path local/owned.
**Result:** 66 tests green, ruff clean. Real render verified (1080x1920, audio intact, title
+ word-highlight caption visible). DeepSeek hook agent verified live (typed candidates).
Also: Whop fully scrubbed (pure owned-first); whisper.cpp + base.en installed.
**Next bottleneck:** face-pan reframe (still dumb center-crop) → backlog #4. Then a standing
real-YouTube end-to-end gate (#3) and the autopilot orchestrator (#6).
