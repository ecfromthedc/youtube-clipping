# Operating Plan — Autonomous Clip Factory

**North star: 100M impressions / month.** Hands-off. The factory sources, clips,
captions, hooks, posts, measures, and **re-tunes itself every cycle** — no clip-by-clip
approval. A human reads one **weekly review**; the loop does the rest.

## The closed loop (one `ycp autopilot` run)

```
source ─→ clip ─→ QC(auto) ─→ capture ─→ brief ─→ scoreboard ─→ OPTIMIZE ─→ distribute
  ▲                                                                  │
  └──────────────── learned source weights (double down) ───────────┘
```

1. **source** — rank each creator's hottest recent videos (live view-velocity), biased by
   `data/learned-weights.json` so winners rise.
2. **clip** — Gemini picks the moments → vertical reframe → lowercase word-by-word captions
   + a lowercase, cue-punctuated hook (held ≥7s) written to that exact clip.
3. **QC (auto)** — no human. `guardrails.publish_allowed` is the gate: must be transformed
   (not a raw reupload), no copyrighted music, clean title. Quality control is the analytics
   loop, not a person — losers get starved next cycle.
4. **capture** — pull public views per posted clip (yt-dlp). Owned retention/revenue lands
   once the clip→videoId linkage is wired from the first Postiz post.
5. **brief** — the weekly Double-Down Brief (what to scale / kill), prose by DeepSeek.
6. **scoreboard** — Race to $15K game state → `SCOREBOARD.md`.
7. **OPTIMIZE** — the actuator. Turns scale/kill verdicts into per-creator source multipliers
   (boost 1.5× winners, throttle 0.4× losers, floor 0.1× so nothing dies forever) and journals
   the change to `IMPROVEMENT-LOG.md`.
8. **distribute** — approved clips → Postiz → the owned channel, scheduled across the day's slots.

## Cadence (launchd crons — `scripts/install-crons.sh`)

| Job | Schedule (ET) | Command |
|-----|---------------|---------|
| Content cycle | 05:00 & 13:00 daily | `ycp autopilot --max-videos 3` |
| Weekly review | Sun 08:00 | `ycp brief --post-slack` → `#youtube-clipping` |

Posts themselves go out at **06:00 / 12:30 / 19:00** (Postiz scheduling); the cycle just
produces clips ahead of the slots. Extra clips roll to later slots/days automatically.

## What you do: the weekly review

Every Sunday the Double-Down Brief posts to `#youtube-clipping`: top creators/formats/hooks,
what got scaled, what got killed, views trend, $ progress. That's the whole touchpoint — no
approving clips. `IMPROVEMENT-LOG.md` is the running record of every self-adjustment.

## The 100M math (honest)

100M/mo ≈ 3.3M views/day. Two levers, both compounding:
- **Volume** — channels × posts/day. Phoenix Protocol is channel 1 of 5 mapped slugs
  (hot-seat, money-fights, crash-out, phoenix-protocol, boardroom); `Carry the Fire` +
  `Marc Robinson` are also live in Postiz. 5 channels × 3/day = 450 posts/mo.
- **Hit rate** — the optimize loop raises average views/post over time by doubling down on
  what wins. 450 posts × ~220K avg = 100M. Early posts won't hit that; the loop closes the gap.

Getting to 100M = turn on more channels + let the loop compound. This plan is the engine; scale
is adding channels to it.

## Live vs. next

- **Live now:** full autonomous loop on public-view learning; Phoenix Protocol posting via Postiz.
- **Next (needs first post):** store each Short's `yt_video_id` from the Postiz publish response →
  enable `capture_full_analytics` (retention + revenue) → richer learning than views alone.
- **Then:** map + launch the other channel slugs; replicate this playbook per channel.
