# Research Goal + Discovery Loop — "Find the fuel that clears $15K/month"

The operations loop (`GOAL-AND-LOOP.md`) hardens **how** the clipping machine runs. This is the
other half: the engine that finds **what goes into it** — the source pages with the highest
virality velocity and the best repurpose-earnings, plus the channel concepts (content angles)
worth spinning up. It directly fills the #1 gap every cycle of `LOOP-LOG.md` has flagged:
sourcing has no real creator list yet.

Same shape as the ops loop: a **Goal** (north star + exit criteria) and a **Loop Prompt** you
paste into `/loop`. The loop runs discovery cycle after cycle until you have a ranked, validated
portfolio of sources and angles — then re-ranks them forever on the real performance data the
live pipeline feeds back.

---

## 🎯 THE GOAL

> **Build and continuously sharpen a ranked, evidence-backed portfolio of (a) YouTube source
> pages with the highest virality-velocity × repurpose-earnings, and (b) 3–5 distinct channel
> concepts — including at least one debate/agitation angle — each with a clear editorial thesis,
> proven-demand evidence, and a go/no-go test plan, so Rising Tides can stand up the channels
> with the highest likelihood of carrying the operation to $15,000+/month on the closed-loop
> pipeline.**

**Done when ALL of these hold (the loop's exit criteria):**
1. **A real ranked list exists** — `SOURCE-INTELLIGENCE.md` has ≥30 scored candidate source pages, each with a real virality-velocity number (from yt-dlp), a monetization path (owned-niche RPM / owned monetization stack), and a Repurpose Opportunity Score. No placeholders.
2. **The angles are defined and testable** — `CHANNEL-CONCEPTS.md` has 3–5 channel concepts, each with a one-line editorial thesis, its feeder source pages, format + hook pattern, lane, and a concrete go/no-go test. **At least one is an agitation/debate concept**, and it stays inside platform policy.
3. **It feeds the live system** — the validated winners are written into `config/niches.yaml`, so `ycp source` produces a real queue. `ycp source` runs clean against it.
4. **Demand is proven, not asserted** — every concept the loop greenlights cites real evidence: actual velocity numbers, a confirmed live campaign, or ≥1 comparable channel already winning with that angle.
5. **The loop closes** — once channels are live, the loop reads the weekly Double-Down Brief (`data/latest-brief.md`) and demonstrably re-ranks sources/angles on real results: winners get more feeder pages, losers get cut.

**Hard constraints (never optimize past these):** every greenlit source must have a credible
clip-monetization path · debate/agitation angles must drive *opinion and engagement*, never
cross into hate, harassment, or dangerous misinformation (that trips platform policy and zeroes a
channel — **channel health > engagement**) · owned-channel concepts must have a real
transformation thesis (not "repost viral clips") · no concept ships to `niches.yaml` unvalidated.

---

## 🧮 The Repurpose Opportunity Score (ROS) — the ranking spine

Score every candidate source page 0–100. This is the research analog of the pipeline's
`virality_score`. Each dimension is computable from tools already on hand.

| Dimension | Weight | What it measures | How to get it |
|---|---|---|---|
| **Virality Velocity** | 0.30 | Peak views/hour on recent uploads — *the* "velocity of virality" | `yt-dlp --flat-playlist --dump-json` → views ÷ hours since publish (the exact logic in `sourcing.parse_entries`) |
| **Repurpose Earnings** | 0.25 | The $ path: a high-RPM niche for an owned YPP asset (owned monetization stack — TikTok Creator Rewards + YPP + affiliate + brand deals) | WebSearch niche-RPM tables; confirm realistic $/1K by niche |
| **Clip Yield** | 0.15 | Extractable viral moments per source video — long-form, multi-topic, high-emotion = many clips/credit | Format + length check (podcast/stream/debate = high; 8-min vlog = low) |
| **Agitation / Engagement** | 0.15 | Does it provoke opinion, debate, sides-taken → comment & share velocity (your specific ask) | Comment-to-view ratio on recent uploads + topic polarity |
| **Headroom** | 0.10 | How *unsaturated* it is by existing clippers — more open lane = more of the spike is yours | Search for existing clip channels on that creator; fewer = higher |
| **Compliance Fit** | 0.05 | Clip-friendly / music-light → low strike risk | Public clip program? Clipping encouraged? Heavy music bed? |

**ROS = 100 × Σ(weight × dimension_normalized).** Rank descending = your "good list of options."
Tune the weights as the live Double-Down Brief reveals which dimensions actually predicted winners.

---

## 🔁 THE LOOP PROMPT  (paste this after `/loop`)

```
You are the discovery engine for the Rising Tides YouTube clipping operation. Your job: find the
source pages and channel concepts with the highest likelihood of carrying the operation to
$15,000+/month, rank them on real evidence, and feed the validated winners into the live pipeline.
Run ONE discovery cycle, fully and verified.

NORTH STAR: a ranked, evidence-backed portfolio of (a) source pages with the highest virality-
velocity × repurpose-earnings, and (b) 3–5 testable channel concepts — at least one a debate/
agitation angle — each with a thesis, proof of demand, and a go/no-go test, so we can stand up
the channels most likely to clear $15K/month on the closed-loop pipeline.

INPUTS — everything is in this one repo root:
  ~/Documents/Development/youtube-clipping/
Read these first, every cycle:
- RESEARCH-LOG.md         (what's been discovered/scored + the current top discovery gap)
- SOURCE-INTELLIGENCE.md  (the ranked, scored source-page list you maintain)
- CHANNEL-CONCEPTS.md     (the 3–5 angle bets you maintain, each with a go/no-go test)
- RESEARCH-GOAL-AND-LOOP.md (the ROS rubric + exit criteria — this file)
- config/niches.yaml + config/niches.example.yaml (where validated winners get promoted to)
- data/latest-brief.md    (the live Double-Down Brief — REAL results once channels run; this is
                           what closes the loop and re-ranks everything)
- YOUTUBE-CLIPPING-WORKFLOW.md (the two-lane model, the money math, the compliance guardrails)
If RESEARCH-LOG.md / SOURCE-INTELLIGENCE.md / CHANNEL-CONCEPTS.md don't exist yet, creating them
(with a real first batch, not stubs) is your first cycle.

EACH CYCLE, DO EXACTLY THIS:

1) ORIENT — read the inputs. State in one line the single biggest thing blocking a validated
   portfolio that can carry $15K/month (e.g. "too few scored sources," "no proven debate angle,"
   "concept X greenlit but demand unproven," "live brief says angle Y is dying — find replacements").

2) DISCOVER — expand the candidate pool along whichever front is weakest right now:
   • NEW SOURCES — find more big, clip-friendly creators (podcasters, streamers, info/debate
     creators). Use yt-dlp flat-dumps to pull recent uploads + view counts and compute real
     view-velocity (views ÷ hours). Use WebSearch/WebFetch to discover channels you don't know.
   • OWNED MONETIZATION — score each niche's owned RPM path (TikTok Creator Rewards + YPP + affiliate)
     right now ($/1K by niche, realistic ramp). Prioritize the high-RPM owned niches.
   • ANGLES — find content angles with high engagement velocity, especially AGITATION/DEBATE
     content (hot takes, "X vs Y," contrarian expert claims, status/generational friction) that
     reliably spark comment wars — comments & shares are a stronger early algo signal than passive
     views, and the comment war does the distribution. Note who already wins each angle.

3) SCORE — run the Repurpose Opportunity Score (ROS, see this file's rubric) on every new
   candidate: Virality Velocity .30 / Repurpose Earnings .25 / Clip Yield .15 / Agitation .15 /
   Headroom .10 / Compliance Fit .05. Use REAL numbers (actual velocity from yt-dlp, actual
   campaign pool, actual comment-to-view ratio) — never guess a score. Add/re-rank rows in
   SOURCE-INTELLIGENCE.md.

4) ANGLE — maintain 3–5 channel concepts in CHANNEL-CONCEPTS.md. Each concept card needs: a
   one-line editorial thesis (the transformation identity), its feeder source pages (from the
   ranked list), format + hook pattern, lane (owned YPP — the only lane), and a concrete GO/NO-GO TEST
   (e.g. "spin up 1 channel, post 20 clips over 14 days, greenlight if median ≥5K views OR ≥1 clip
   >50K; else kill"). Keep at least one AGITATION/DEBATE concept — and keep
   it inside policy (spicy opinion = fine; hate / harassment / dangerous misinfo = banned, never
   go there).

5) VALIDATE — take the ONE highest-leverage candidate or concept this cycle and PROVE the demand
   is real, not asserted: cite the actual velocity numbers, confirm the campaign is live and
   funded, or point to ≥1 comparable channel already winning with that exact angle (name it, with
   evidence). If you can't prove it, mark it "unvalidated — do not promote" and say why.

6) PROMOTE — write the validated winners into config/niches.yaml in the real schema (niches →
   creators → name/handle/lane/notes). Then run `ycp source` (or at least the
   parser) to confirm it ingests clean and produces a real queue. Only validated rows get promoted.

7) LOG & COMPOUND — append a dated entry to RESEARCH-LOG.md: what you discovered/scored/validated,
   the evidence, what got promoted to niches.yaml, and the NEW top discovery gap. IF data/latest-
   brief.md exists with real results, reconcile against it: which live angles/sources are
   over-indexing (→ find more like them) and which are dying (→ cut, replace). That reconciliation
   IS the closed loop — research bets get re-ranked on real money every week.

8) DECIDE — if ALL exit criteria below hold, output "PORTFOLIO VALIDATED — entering re-rank
   maintenance" and stop. Otherwise end by naming the next cycle's single discovery target.

EXIT CRITERIA (all must hold):
  ✓ SOURCE-INTELLIGENCE.md has ≥30 real, ROS-scored source pages (no placeholders).
  ✓ CHANNEL-CONCEPTS.md has 3–5 concepts, each with thesis + feeders + go/no-go test; ≥1 is a
    debate/agitation angle that stays inside platform policy.
  ✓ Validated winners are in config/niches.yaml and `ycp source` runs clean against it.
  ✓ Every greenlit concept cites real demand evidence (velocity / live campaign / comparable win).
  ✓ Once channels are live, the loop has demonstrably re-ranked sources/angles off data/latest-brief.md.

RULES: real numbers only — a score or a "this will work" with no evidence behind it is a bug, fix
it or mark it unvalidated. Compliance is non-negotiable: every source needs a real monetization
path, agitation angles drive opinion not hate, owned concepts need a true transformation thesis,
channel health > engagement. Be honest in the log — if demand is unproven or a scan failed, say so
plainly. One fully-validated discovery per cycle beats five hand-waved ones. Do not stop early for
any reason other than the exit criteria being met.
```

---

## ▶️ How to run it

**Phase 1 — Discover (now → validated portfolio):** run it self-paced so it keeps discovering,
scoring, and validating until the exit criteria are met:

```
/loop <paste the loop prompt above>
```

(No interval = Claude self-paces, cycle after cycle, until it declares the portfolio validated.)

**Phase 2 — Re-rank (after channels are live):** once the live pipeline is producing a weekly
Double-Down Brief, switch to weekly so research keeps re-ranking on real results — find more of
what's winning, cut what's dying, and surface fresh sources for the proven angles (run it Monday
after the brief drops):

```
/loop 1w <paste the loop prompt above>
```

**Compact one-liner** (terse version):

```
/loop One discovery cycle for the clipping op. Repo root: ~/Documents/Development/youtube-clipping.
Read RESEARCH-LOG.md + SOURCE-INTELLIGENCE.md + CHANNEL-CONCEPTS.md + data/latest-brief.md.
Find the weakest front (sources / live campaigns / angles incl. debate-bait); expand it with REAL
yt-dlp velocity + web research; score candidates on the ROS rubric; keep 3–5 testable channel
concepts (≥1 agitation angle, inside policy); validate the top bet with real evidence; promote
validated winners into config/niches.yaml and prove `ycp source` ingests it; log it + reconcile
against the live brief + name the new gap. Real numbers only; channel health > engagement. One
validated discovery per cycle.
```

---

## 📂 Files the loop maintains

| File | Role |
|---|---|
| `SOURCE-INTELLIGENCE.md` | The ranked, ROS-scored list of candidate source pages — your "good list of options." The loop's primary deliverable; gets longer and sharper every cycle. |
| `CHANNEL-CONCEPTS.md` | The 3–5 channel concepts (content angles) to test, each a thesis + feeders + format/hook + lane + go/no-go test. Includes the debate/agitation bets. |
| `RESEARCH-LOG.md` | Append-only cycle log: what was discovered/scored/validated, the evidence, what got promoted, the current #1 discovery gap, and the reconciliation against the live brief. The loop's memory. |
| `config/niches.yaml` | The hand-off to the live pipeline. Validated winners land here so `ycp source` produces a real queue. (Owned by this loop; consumed by the ops loop.) |

Inputs it reads but doesn't own: the strategy/math (`YOUTUBE-CLIPPING-WORKFLOW.md`), the live
pipeline's output (`data/latest-brief.md`), and the `ycp` system itself.

---

## 🔗 How this closes the loop with the ops pipeline (the week-over-week compounding)

The two loops form one circuit. This is the "feeds its successes back and optimizes week over
week" you asked for:

```
  THIS loop discovers + ranks sources/angles  ─┐
                                                ▼
                     promotes winners → config/niches.yaml
                                                ▼
        ops pipeline runs them:  ycp source → clip → qc → distribute → capture
                                                ▼
              ycp brief scores REAL results → data/latest-brief.md
                                                ▼
   THIS loop reads the brief → doubles down on winning angles, cuts losers, ──┐
                          finds MORE sources like the winners                  │
                                                ▲                              │
                                                └──────────────────────────────┘
```

Research seeds the bets → the pipeline runs them → the Double-Down Brief measures them in real
views and real dollars → research re-ranks on that truth and points the next week's discovery at
what's actually winning. The `$15K/month` target is reached by feeding the proven machine the
highest-opportunity fuel: higher-velocity source pages, the engagement multiplier of the debate
angles, and several validated concepts running in parallel rather than one guess.

> Sister doc: `GOAL-AND-LOOP.md` hardens the machine so any operator can run it. This loop makes
> sure what they're running is the highest-probability portfolio you can find — and keeps it that
> way as the data comes in.
