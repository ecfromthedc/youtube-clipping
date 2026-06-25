# Improvement Log — Phoenix Protocol clip factory

_Auto-appended by the OPTIMIZE stage each cycle. Newest entries at the bottom._
_North star: 100M impressions / month. The loop doubles down on what wins._

## 2026-06-24 — Cycle 0: the foundation (how this came up)

Before the loop could improve itself, the machine had to exist and run hands-off. What's
in place as of go-live:

- **Pipeline:** source → clip → caption/hook → QC → capture → brief → scoreboard → optimize →
  distribute, chained by `ycp autopilot`.
- **Creative dialed in (Eric's calls):** lowercase subtitles AND hooks; captions sized to frame
  width (`size_pct` 10); hooks held ≥7s, written to each clip with cue punctuation; per-channel
  hashtags (`#shorts #health #longevity #fitness #wellness` for Phoenix Protocol).
- **Went autonomous:** QC flipped to auto — guardrails are the publish gate, the analytics loop
  is the quality control. No clip-by-clip approval.
- **Closed the loop:** added the OPTIMIZE actuator — the scoreboard's winners now get sourced
  harder and losers throttled on the *next* run (was measured-but-not-acted-on before).
- **Distribution live:** Postiz token wired, Phoenix Protocol integration verified, 3 scheduled
  slots/day (06:00 / 12:30 / 19:00 ET).
- **Measurement live:** public-view capture works today; YouTube Analytics OAuth wired for
  retention/revenue (enables once the first post gives us the clip→videoId linkage).
- **Scheduled:** content cycle 2×/day + weekly review to Slack via launchd.

**What I'm getting better at next:** (1) wire `yt_video_id` from the first Postiz post so the
loop learns from retention + revenue, not just views; (2) replicate this playbook across the
other channel slugs to scale toward 100M/mo. Every cron cycle from here appends below.
