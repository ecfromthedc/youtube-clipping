# Clip Pipeline — Handoff

_State as of 2026-06-28. Written after a long, bad session that produced slop. No spin below — just what's true, what's broken, and what to do next._

## TL;DR
- **Goal:** make 20 vertical Shorts (founders/VCs, AI topics) that are actually good enough to post.
- **Status:** nothing is running. The auto-pipeline produces slop (duplicates, empty-room crops, wrong-name hooks, the same 3–4 interviews on a loop).
- **Root cause:** too many contradictory rules stacked reactively on top of a pipeline that re-uses a frozen source list and has no taste layer.
- **The real fix:** strip rules, refresh sourcing *before* clipping (or wire it into the loop), and — most importantly — give the operator manual selection instead of auto-sourcing roulette.

## What it is
- A Python CLI (`ycp`) + scripts. `scripts/make_clips.py` is the batch "mission." Pipeline modules live in `src/ycp/`.
- **Flow:** `ycp source` (live yt-dlp → fills the `source_videos` DB table) → `make_clips.py` reads that queue → per video: download a bounded window → Whisper transcribe → goldmine heatmap picks the most-replayed moment → `cut_vertical` (face-pan 9:16 crop) → DeepSeek writes the hook → Pillow burns captions → **Gemini gate** approves/rejects → lands in `data/clips/unreviewed/` or `unusable/`.

## What's actually broken (root causes, ranked)
1. **Sourcing is a one-time command the clip loop never re-calls.** `source_videos` is **107 rows all frozen at one timestamp** (`06:04:02` today). Every run chews the exact same list → it never gets new material. This is the #1 reason it "regurgitates instead of researching."
2. **The clipper camps on the top rows.** The founder/VC sort puts All-In (Chamath) and 20VC (Perplexity CEO) first, so every run starts there → you see the same interviews over and over.
3. **No taste / editorial layer.** It clips "most-rewatched 30s," not "worth watching." The machine has zero concept of who or what is worth a clip. *This is the deepest problem and no rule fixes it.*
4. **`entropic` → it's `Anthropic`.** Whisper mishears AI proper nouns and the hook parrots the broken word. **NOT fixed.** Fix = a term-correction map applied once to the transcript (`segments`) right after `segments = transcribe(...)` in `src/ycp/clip.py:283`. Map e.g. `entropic→Anthropic`, `open ai→OpenAI`, `chat gpt→ChatGPT`, `deep seek→DeepSeek`, `nvidia→Nvidia`, `perplexity→Perplexity`. One place fixes both hook and captions.

## Fixed this session (committed)
- **A/B hook tester OFF** (`config/settings.yaml` → `ab.enabled: false`). It was cutting the same clip 3× as `-v0/-v1/-v2` with different hooks. The operator never asked for A/B.
- **Dedup back ON / `force` removed; one clip per distinct video.** No more re-cutting the same moment.
- **Gemini gate, fail-closed.** Nothing reaches `unreviewed/` without a visual sign-off. The contact sheet now **forces the true first frame (t=0)** so empty-room openings get caught (`src/ycp/vision.py:gate`).
- **Thread-safe face detection** (`src/ycp/reframe.py:_detect`). A race across parallel clips was crashing detection → dumb center-crops (walls).
- **Open-on-speaker trim** (require a close face, not a wide establishing shot) + **bad-framing now hard-rejects** in QC.

> ⚠ These are real fixes, but they're patches on an over-ruled machine. Don't keep stacking more.

## What to actually do next (don't repeat this session)
1. **Stop adding rules.** The pipeline already has too many that fight each other.
2. **Refresh sourcing before any clip run** — or wire `ycp source` into the mission so the queue is never stale.
3. **Build the thing the operator actually wants:** *operator points at a source/moment → tool just cuts it.* Manual selection, mechanical execution. No auto-sourcing, no heatmap roulette, no "loop until 20."
4. **Fix the transcript term map** (entropic→Anthropic etc.).
5. If you keep the auto-mission: make it walk **distinct** videos and not camp on the top rows; verify output by pulling a frame from each before declaring done.

## Where state lives
- **DB:** `data/clips.db` — `source_videos` (the queue) + `clips` (95 produced rows).
- **Clips:** `data/clips/{unreviewed,approved,used,unusable}/` + `meta/*.json` sidecars. `unusable/` is currently full of this session's slop (dupes, walls) — safe to wipe.
- **Config:** `config/niches.yaml` (roster), `config/settings.yaml`, `config/hook-playbook.md` (the only real creative prompt — DeepSeek hook writer). Gemini prompts live in `src/ycp/vision.py`.
- **Secrets:** `.env` (gitignored) — `DEEPSEEK_API_KEY` (recovered from `~/dev/html-only-agent/.env`), `GEMINI_API_KEY`.

## Git
- **`origin` = `ecfromthedc/youtube-clipping`** (Eric's; this machine's `gh` is `Risingtides-dev`, no push access there → 403).
- All this session's work is pushed to the fork **`Risingtides-dev/youtube-clipping`** (remote `fork`, branch `main`).

## Run commands
```bash
./scripts/setup.sh                              # reinstall after ANY src/ycp edit (non-editable install)
.venv/bin/python -m ycp source                  # refresh the source queue (the missing "research" step)
.venv/bin/python scripts/make_clips.py 20       # batch mission (current behavior: one clip/video, gated)
ruff check src tests && .venv/bin/python -m pytest -q   # 142 tests green
```
