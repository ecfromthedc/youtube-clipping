# Dialed Checklist — every box ✅ *with evidence* before we call this 100%

The definition of "flawless" for this build. The Ralph loop works this top-to-bottom.

**Rules for the loop:**
- Check `- [x]` ONLY with concrete evidence appended inline (the command + its key output, an
  ffprobe number, "Read frame → hook at top, legible, one caption set"). No evidence = leave it.
- **NEVER post live during the audit.** Verify distribution via the unit tests / a mock adapter /
  a sandbox `YCP_HOME` with `distribution.enabled: false`. The real channel is not a test target.
- `ruff check src tests` clean + `pytest -q` green after EVERY change. Python `src/ycp` is LIVE —
  fix forward, never break it. Stage by explicit path; never `git add -A`.

## Gates (objective)
- [ ] `ruff check src tests` clean
- [ ] `pytest -q` all green (record the count)

## Pipeline stages — verify on REAL inputs
- [ ] **source**: `ycp source` returns a ranked queue from live yt-dlp (record N)
- [ ] **clip render**: Read a real rendered frame, confirm ALL — captions legible + lowercase;
      hook present, lowercase, held ≥7s; **NO double subtitles** (RULE #1: hook stays + ONE
      caption set); duration 20–35s (ffprobe); vertical 1080x1920
- [ ] **guardrails**: a clip with music / non-`auto-clip` fmt / avoid-list creator is REJECTED
- [ ] **qc**: auto-approves a transformed clip, rejects a bad one
- [ ] **distribute** (sandbox/mock, NO live post): posts only top `max_per_run`, marks the rest
      `skipped`, parks unconnected channels
- [ ] **capture**: resolves a Postiz post_id → YouTube videoId + pulls analytics (read-only, real)
- [ ] **optimize**: produces weights + appends IMPROVEMENT-LOG.md from real data
- [ ] **milestones**: reads real channel stats, correct progress line, no false crossings
- [ ] **archive**: a clip lands in the Phoenix Protocol Drive folder
- [ ] **cleanup**: prunes local files of posted clips only
- [ ] **delete-video**: refuses a video NOT on our channel (safety); accepts one that is

## Autonomy — the live loop
- [ ] all 3 crons loaded (autopilot / weekly-review / milestones)
- [ ] **autopilot end-to-end** (sandbox `YCP_HOME`, `distribution.enabled: false`): 9/9 stages,
      sane outputs, NO broken clips, ≤ `max_per_run` would post
- [ ] config coherence: posting_times, channel mapping, all secrets present (by name only)

## Rust port — folded in from the render-fix loop
- [ ] Rust clips clamped ≤ 38s (ffprobe a Rust-rendered clip)
- [ ] Rust moment scores in 0–1 (A/B gate fires selectively, not on every moment)
- [ ] Rust hook-title render matches Python (Read a Rust frame: wrapped, top, legible)

## Sign-off
- [ ] Every box above is ✅ with evidence → write `DIALED-DONE` (one-line summary). Loop stops.
