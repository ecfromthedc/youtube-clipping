# Status & Plan — YouTube Clipping (Phoenix Protocol)

> ⛔ **SHARED POSTIZ ACCOUNT.** The team grows MULTIPLE accounts on this one Postiz login
> (Phoenix Protocol = ours, `cmqsakb8z0123ml0yd61m7h9g`; Carry the Fire + Marc Robinson =
> teammates'). EVERY Postiz op (post + cleanup) filters to OUR integration id only. NEVER delete
> posts by state. Other accounts run independently — we never touch them.

_Living snapshot. Last updated 2026-06-25, after the first live autonomous cycle + fixes._
_Read this first for current state; the doc map at the bottom has the detailed plans._

## Where we are
- **Phoenix Protocol** (health/longevity, faceless clip channel) is the first LIVE owned channel. The autonomous factory ran its first real cron cycle this morning and posted to it.
- The pipeline is **fully autonomous + self-improving**: source → Gemini moment-pick (20–35s) → vertical reframe → word-by-word captions + hook → auto-QC (guardrails) → post the best clip → measure → optimize. Cron-driven, no human approval.
- **Self-improving loop is live**: scoreboard → optimize (boost winners) → A/B the top hook → retention curves (swipe-away) → by-hour timing. Every lever tunes on real data each cycle.
- **Watchers**: weekly Double-Down Brief (Sun 8am → Slack) + monetization milestone watcher (daily 9am → Slack alert on every threshold up to $15k/mo).
- Clips archived to the Google Drive "Phoenix Protocol" library; local disk auto-pruned. 121 tests green.

## This morning, honestly (first real run)
- ✅ **First autonomous cron cycle fired and posted live** — the engine works end-to-end, unattended.
- 🔴 **It over-posted** (38 clips/run ≈ 2-week backlog — "post every clip + A/B every moment"). **FIXED:** quality-selection — post only the single highest-score clip per cycle, skip the rest; A/B only the top moment. **Backlog purged** (38 scheduled posts deleted from Postiz).
- 🟡 **Rust cutover = NO-GO.** The binary runs the whole pipeline, but the hook-title render is broken + durations/scores off. **Python stays production**; Rust fixes queued (low priority — Python works).
- ✅ **Only 1 clip is live on Phoenix Protocol** — the on-brand zone-2 cardio validation clip. The "flood" was 38 *scheduled* (not published) posts, now purged. (Postiz's published list also showed other channels' videos — verified via the API; only ours counts.)
- 🧹 **Autonomous video cleanup wired** — `ycp delete-video <id>` (YouTube write scope), hardened to delete ONLY videos on our channel, so the operator can clean its own mistakes without touching anything else.

## Operating config (current)
- **Posting:** 12:30 / 3:00 / 8:00 PM ET (prime Shorts windows). Content cron produces at 5am + 1pm.
- **Cadence:** 1 best clip/cycle → ~2/day (cold-start; scales via the ladder as the channel warms — quality over volume by design).
- **Length:** 20–35s · **auto-QC on** · only **credentialed-expert** sources (YMYL advertiser-safety + the transformed-clips monetization rule).

## The plan forward
1. **Let it run + accrue data** — real retention/timing data lands over 24–48h; the loop starts tuning itself.
2. **Gemini final-clip QC (next build)** — review the *rendered* output (not just the moment), the one quality gap left.
3. **Scale on data (the ladder)** — bump posts/day as retention holds; clone the engine to the next channel (the real lever to 100M).
4. **Monetization** — watcher alerts at 500 subs + 3M views/90d → apply (steps in MONETIZATION.md). North star: $15k/mo.
5. **Rust (lower priority)** — fix renderer + clamp + score-scale, then cut over for lean single-binary team distribution.

## Doc map
- **CONTENT-STRATEGY.md** — formats, hooks, length, cadence, the scaling ladder.
- **RESEARCH-shorts-2026.md** — 2026 Shorts evidence base + the "come correct" doctrine.
- **MONETIZATION.md** — YPP path, readiness checklist, the transformed-clips eligibility rule.
- **OPERATING-PLAN.md** — the closed-loop system + cron cadence.
- **IMPROVEMENT-LOG.md** — what the loop changed each cycle (auto-appended).
- **LEARNINGS.md** — mistakes & fixes, so the team doesn't repeat them.
- **HANDOFF.md** — original cold-start brief. **rust/README.md** — Rust port status.
