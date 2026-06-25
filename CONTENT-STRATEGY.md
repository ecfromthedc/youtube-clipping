# Phoenix Protocol — Content Strategy

Faceless health/longevity YouTube Shorts clip factory. This is the day-1 strategy, grounded in
2026 Shorts research (sources at bottom) + RT's Undertow hook framework. The closed loop
(scoreboard → optimize → A/B → retention → timing) re-tunes every number here on real data —
but we come in the gate with a real plan, not guesses.

## The one metric that rules everything
**Swipe-away in the first 1–2 seconds.** YouTube seeds a new Short to a small batch (hundreds–
few thousand) and watches the first 30–60 min: if they don't swipe, it expands; if they swipe,
distribution stops — often before your real audience sees it. The "good" bar is **~70% average
view duration**. We capture `swipe_away_pct` per clip (retention curve) — it is our north-star
QUALITY metric, ranked above raw views.

## Positioning
Clips of **credentialed** experts (Dr Mike Israetel, Bryan Johnson, Jeff Nippard, Dr Gabrielle
Lyon, Diary of a CEO health eps). Borrowed authority = instant trust AND advertiser-safety: for
YMYL health content, the claim comes from an MD/PhD, not us. Only clip credentialed sources.

## Formats that win (ranked)
1. **Myth-busting** — the #1 *shareable* health format (viewers send it to someone who believes the myth). "X is a lie," "stop doing Y."
2. **Actionable protocol tips** — replicable + time-respecting ("the 10-min habit that…"). Saves drive reach.
3. **Counterintuitive / shocking** — "the guy who trains less lives longer."
4. **Research-cited** — "a 2024 study found…" / "research shows…" in the HOOK spikes credibility + save rate.
5. **Transformation / visual payoff.**
Trending sub-topics: longevity routines, zone-2 cardio, sleep, protein/muscle (muscle = organ of longevity), realistic/replicable fitness, 60+ strength, cortisol, fasting.

## Hooks (first 3 seconds = everything)
- Lowercase, cue-punctuated, written to THIS clip (see `config/hook-playbook.md` — Undertow: 16 principles + 13 formulas from 1,153 analyzed reels).
- **Citation-in-hook** for health ("a 2024 study found…") — boosts trust + saves. [pipeline nudge queued]
- **85% watch muted** → the burned word-by-word caption + hook ARE the hook. Captions non-negotiable. One subtitle track only (Rule #1).

## Length
- **Target 20–35s** for tips/myth-busts (15–30s optimal, 20–25s algorithm-favored, >45s drops off hard). Allow up to ~45s only when a moment truly needs it. (General-Shorts data claims 50–60s maximizes completion — but the health-niche signal is shorter; we bias short and let A/B settle it.)
- Current output ran 37–44s → slightly long. [pipeline tuning #1]

## Cadence, timing & the scaling ladder
- **Times (ET): 12:30 PM · 3:00 PM · 8:00 PM.** Prime Shorts windows are 12–3 PM and 7–9 PM; mornings are dead. 8 PM ET = 5 PM PT = evening-prime on both coasts. Consistency beats perfect timing — same slots daily so the algorithm learns the cadence. Starting points; `by_hour` re-tunes per channel.

- **Frequency scales aggressively WITH the channel — gated on quality, never ahead of it.** More posts only help if each still clears the swipe-away bar; volume of *skipped* clips actively THROTTLES the whole channel. So we ramp the moment it's cooking, and back off a tier if retention dips:

  | Tier | When (gates) | Posts/day |
  |------|--------------|-----------|
  | **Cold-start** (now, 0 subs) | protecting the initial signal | **3/day** |
  | **Warming** | avg retention holding ≥ ~60% (swipe-away controlled) + clips regularly clearing a view floor + subs climbing | **5/day** |
  | **Cooking** | near/at monetization, consistent multi-K-view clips, retention HOLDS as volume rose | **8/day** |
  | **Aggressive** | proven channel, quality holds under load | **10–12/day** |

  The ramp is data-gated (the retention/`swipe_away` signal decides each step up or down), so we get to push hard without choking the algorithm. Cold-start stays disciplined *because* that's what earns the right to go aggressive fast.

- **Two axes of scale — push both when warm:** (1) posts/channel (above), and (2) **# of channels** (the bigger lever to 100M — replicate the proven engine). Aggression = ramp the winner's frequency AND clone the engine to the next channel.

- **[Queued] auto-ramp:** wire the frequency tier to fire automatically off the retention gates so the channel scales itself; until then I manage the ramp in the weekly review.

## Monetization
- 2026 YPP entry tier: **500 subs + 3 public posts/90d → 3M valid Shorts views/90d** (or 3k watch hrs). Full YPP: 1k subs + 4k hrs OR 10M Shorts views/90d.
- Realistic: **3–6 months** at 2–3/day. Stack AdSense + affiliates (health products in description) + licensing. Owned-first (we hold the YPP asset).
- **Full onboarding steps + readiness checklist + the transformed-clips eligibility rule → `MONETIZATION.md`.** Progress tracked automatically by `ycp milestones` (daily cron → Slack alert on every threshold up to $15k/mo).

## Mistakes we don't make
Dead-window posting (killed 6 AM) · weak first-frame hook / slow open · >45s drag · two subtitle tracks · inconsistent schedule · unsafe YMYL claims (credentialed sources only).

## Implementation queue (priority)
1. **Clip length → bias 20–35s** (vision picker prompt + `MAX_CLIP_SEC` 45→38) ✅ done.
2. **Posting times → 12:30/15:00/20:00 ET** ✅ done.
3. **Quality-selection**: overproduce, post the best N/day (vs posting every approved clip on a rolling schedule) — kills stale-clip dilution. ⏳ next.
4. **Citation-in-hook** nudge for health clips in the hook agent prompt. ⏳
5. **Auto-ramp frequency** off the retention gates (scaling ladder above) so the channel scales itself once warm. ⏳

## Sources (2026 research)
- Best times / frequency: miraflow.ai, hopperhq.com, flowshorts.app, socialpilot.co, fluxnote.io
- Shorts algorithm / swipe-away / retention: nexora-ai.org, vidiq.com, metricool.com, humbleandbrag.com (benchmarks)
- Faceless growth / monetization (YPP tiers, timeline): virvid.ai, nexlev.io, unkoa.com, shortvids.co
- Health-niche formats / hooks: athletechnews.com, fluxnote.io, vidiq.com, creatorsjet.com
