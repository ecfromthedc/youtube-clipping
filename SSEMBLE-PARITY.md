# Ssemble Parity — own the whole stack, no ceiling

**Goal:** reproduce every Ssemble feature with free, local, uncapped tools so the
operation never depends on the app, a credit balance, or a subscription. Ssemble
stays as a convenience (and its auto-poster), but we can produce unlimited output
without it.

**The toolbox (all free, local, already installed except OpenCV):**
`yt-dlp` (download) · `ffmpeg` (everything visual) · `whisper` (transcribe/translate)
· `Ollama`/`llama3.1:8b` (hook titles, translation) · **OpenCV** (face tracking — one
lightweight add). No per-clip cost, no cap.

---

## Feature-by-feature ownership map

| # | Ssemble feature | Our owned replacement | Tool | Status |
|---|---|---|---|---|
| 1 | **Auto Curation** (detect viral moments) | Transcript hook-score (✓ shipped) **+** audio-energy peak detection (laughs/emphasis/loudness spikes) blended into the ranking | whisper + ffmpeg + numpy | v1 shipped · energy-blend = cycle |
| 2 | **Face Tracking** (keep face centered in 9:16) | Detect face per sampled frame → smoothed crop path that follows the speaker → time-varying ffmpeg crop. Falls back to center-crop. | OpenCV + ffmpeg | build cycle |
| 3 | **Auto Captioning** | Whisper → SRT → burned captions (✓ shipped). Upgrade: word-level "karaoke" highlight via ASS styling | whisper + ffmpeg | **shipped** · karaoke = polish |
| 4 | **Caption Translation** (translate, keep original audio) | Translate the SRT text locally, keep audio, re-burn. Unlimited languages. | Ollama (or whisper `--task translate` for →EN) | build cycle |
| 5 | **Hook Title & CTA** | Generate a hook title + CTA from the transcript, burn a top title banner + a timed "Subscribe" CTA banner | Ollama + ffmpeg drawtext | build cycle |
| 6 | **Game Video** (gameplay strip for retention) | Stack the clip over a looping gameplay clip (Subway Surfers / parkour / GTA) — split-screen vstack | ffmpeg + a gameplay-loop library | build cycle (needs footage) |

**Quality vs Ssemble, honest:** captioning and gameplay-stack are *at parity* immediately
(same ffmpeg under the hood). Hook titles via local Llama are ~90% as slick and infinitely
cheaper. Face tracking and AI curation are the two where Ssemble's models are more polished
— our versions are "very good, improving each loop cycle," and good enough because the QC
gate + the Double-Down Brief filter for what actually performs. The win is **zero ceiling**:
unlimited clips, unlimited channels, $0 marginal cost.

---

## Architecture — composable enhancement stages

The `ycp clip` pipeline becomes a chain of optional, owned stages. Operator toggles what
they want; every stage is local and uncapped:

```
download → transcribe → CURATE(rank) → REFRAME(face-track|center) → CAPTION(burn|karaoke)
         → [TRANSLATE lang] → [HOOK+CTA overlay] → [GAMEPLAY stack] → register for QC
```

```bash
# examples once all stages land:
ycp clip <url> --max 8                          # curate + reframe + caption (default)
ycp clip <url> --face-track --gameplay subway   # podcast clip with retention strip
ycp clip <url> --hook-cta --translate es        # add hook banner + Spanish captions
```

Pure logic (ranking, crop-path smoothing, filter/command construction) is unit-tested
deterministically; ffmpeg/OpenCV execution is validated on the real machine + first runs.

---

## Build order (loop cycles, highest leverage first)

1. **Hook Title & CTA + Gameplay stack** — pure ffmpeg, biggest watch-time/CTR lift, easiest.
2. **Face tracking** — OpenCV crop-path; the marquee parity feature.
3. **Caption translation + karaoke captions** — reach + retention polish.
4. **Audio-energy curation** — smarter moment-ranking than transcript alone.

> Maintained by the optimization loop. Each cycle ships one stage, verified, logged in
> `LOOP-LOG.md`.
