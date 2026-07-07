# CHANNEL-PLAYBOOK.md — one channel = one path an agent owns + optimizes

**Each concept is a channel.** This is the canonical map. For every owned channel: the exact
sources to clip from, the *one* format that makes it a channel (not a random feed), the metric
that channel optimizes on, and the loop an agent runs to climb its go/no-go gate. **One agent can
own one channel end-to-end** and optimize it independently of the others.

- **Sources** are verbatim from `config/niches.yaml` (the Stage-1 spec `ycp source` reads). Each
  `niches.yaml` group `name:` = one channel here.
- **Format / hook / guardrails / gate** come from `CHANNEL-CONCEPTS.md` (the angle rationale).
- **ROS scores + live velocity** per source are in `SOURCE-INTELLIGENCE.md`.
- **This file is the canonical channel↔source↔format map.** Where the older docs' numbering
  disagrees, the channel IDs here win.

**Launch order (decided):** Hot Seat + Money Fights **now** → Crash Out + Myth-Busting next →
Boardroom on deck.

---

## The optimization loop — every channel runs this

The **format is the path**; these five steps are how an agent *walks it and improves* its channel:

1. **Source** — `ycp source` pulls this channel's feeders (its `niches.yaml` group). Take the
   freshest high-velocity uploads.
2. **Cut to the channel's format** — every clip obeys the one format on the card below. Transform
   always (cut + hook + captions); raw reuploads demonetize the channel.
3. **Post 20 over 14 days, then read the scoreboard** — `ycp brief` ranks this channel's
   `source × hook × length` combos by its **optimize-on metric**.
4. **Double down + prune** — make more of the 🟢 combos, cut the dead ones, and **iterate the hook
   once** if the channel is mid (2–5K). The optimize-on metric is the steering wheel.
5. **Re-rank weekly** — the brief feeds the discovery loop: winning sources get more pulls,
   winning hook-shapes get replicated. Climb the gate → scale → take on the next channel.

**The universal gate** (unless a card raises it): 1 channel · 20 transformed clips · 14 days →
**GREENLIGHT** median ≥5K views OR ≥1 clip >100K · **ITERATE** hooks once at 2–5K median ·
**KILL** <2K median + no breakout. Every result logs to the DB → the weekly Double-Down Brief.

---

## Channel 1 — "Hot Seat" · debate flashpoints  🟢 LAUNCH NOW
*Identity: the moment everyone's arguing about, isolated.*

- **`niches.yaml` group:** `debate-agitation`  ·  **angle:** CHANNEL-CONCEPTS.md §Concept 1
- **Clip from:** `@jubilee` (51.7K/hr — top live velocity) · `@nojumper` · `@PopCultureCrisis` ·
  `@whatever2ND` (the **live** Whatever clip surface — NOT the stale `@whatever` main) ·
  `@moderndaydebate`
- **The format (your path):** cold-open *inside* the clash (never the calm intro) → the polarizing
  claim as bold on-screen text in frame 1 (it has to land on mute) → cut to reaction faces →
  **end on the unresolved question + "Who's right? 👇".**
- **Hook:** the most polarizing line, on screen, first frame.
- **Guardrails:** attack the **position/behavior — never the protected group/person.** Hard-exclude
  the Fresh & Fit grievance lane; treat trans-targeted content as RED. (Structured debate has the
  policy carve-out — staying position-based is what keeps it monetizable.)
- **Optimize on → comment-rate.** Greenlight also if comment-rate >2× the channel baseline even at
  modest views — it's the leading indicator here. Reply to 50+ comments in the first 2 hrs (≈15–20%
  more reach).

## Channel 2 — "Money Fights" · finance conflict  🟢 LAUNCH NOW
*Identity: the money conversations people don't say out loud.*

- **`niches.yaml` group:** `finance-money`  ·  **angle:** CHANNEL-CONCEPTS.md §Concept 2
- **Clip from:** `@ramitsethi` (#1 open lane — Money for Couples) · `@GrahamStephan` ·
  `@codiesanchezCT` · `@MyFirstMillionPod` · `@AlexHormozi` · `@garyvee`
- **The format (your path):** lead with the dollar number or the fight ("They make $400K and can't
  stop fighting about $40") → caption the rising tension → pay off the reveal at the end.
- **Hook:** the dollar number / the conflict line, first.
- **Guardrails:** advertiser-safe finance = highest US RPM; keep it that way (no get-rich-quick
  claims). ⚠ My First Million is HubSpot-owned — watch takedown posture before scaling it.
- **Optimize on → watch-through to the reveal + saves.** The end-payoff drives completion; saves
  mean "I'll act on this." Iterate the opening dollar-hook first.

## Channel 3 — "Crash Out" · comedy & reaction  ⏳ WAVE 2
*Identity: the bit you'll text your group chat.*

- **`niches.yaml` group:** `comedy-crashout`  ·  **angle:** CHANNEL-CONCEPTS.md §Concept 3
- **Clip from:** `@BadFriends` (open lane) · `@killtony` (60s sets = the most clippable format) ·
  `@OfficialFlagrant` · `@thisisimportant` (clearest white space)
- **The format (your path):** punchline-first, fast cuts, **no setup.** Caption the funniest line.
- **Hook:** the punchline in frame 1.
- **Guardrails:** ⚠ Test Content-ID on This Is Important (iHeart) before scaling. Cherry-pick
  Flagrant — political eps = limited ads.
- **Optimize on → share-rate + raw view ceiling.** Comedy has to travel, so **raise the bar:**
  median ≥10K OR ≥1 clip >250K, or it isn't landing. Shares are the comedy tell.

## Channel 4 — "Phoenix Protocol" · transformation  🔥 LIVE
*Identity: proof you can become someone new on the other side of your comfort zone.*

- **`niches.yaml` group:** `health-mythbusting` (key unchanged)  ·  **angle:** `transformation`
  (`autopilot.angle_for` → tunes the hook prompt + scorer to the comfort-zone register)
- **Clip from — SPINE (embodiers, own clean channels):** `@richroll` (addict→ultra = the phoenix
  story) · `@wimhofmethod` · `@ShawnRyanShow` (rock-bottom→rebuilt) · `@TomBilyeu` · `@EdMylettShow`
  (max-effort motivation) · `@garyvee` (hustle/self-belief, *encourages* clips). **Firehose:**
  `@ChrisWillx` (Goggins-types appear as GUESTS).
  **Credibility garnish:** `@RenaissancePeriodization` · `@JeffNippard` · `@biolayne1` · `@bryanjohnson`.
- **The format (your path):** the line that hits your chest → the turn → "you can do hard things."
  Clip for the screenshot-the-quote moment, not the training-notebook moment.
- **Hook:** second-person challenge — "the comfort zone is the enemy", "nobody's coming to save you",
  not "the science says". Embodiers > experts.
- **Guardrails:** **no copyrighted music** — the genre lives on epic orchestral beds = instant
  Content-ID; `guardrails.publish_allowed` HARD-BLOCKS `has_music:true`, so source clean-VO segments
  (avoid music under the source). **Huberman/Attia are gate-blocked** (terms) — surface only as guests
  on the firehose. Agitation attacks **the comfort zone**, never a person/group. No medical prescriptions.
- **Optimize on → shares + saves.** Shareable (motivational = sent to a friend) AND sticky (saved to
  rewatch). Evergreen back-catalog, low time-pressure → test hooks freely.

## Channel 5 — "Boardroom" · business & mindset authority  🪑 ON DECK  *(name = working title)*
*Identity: the contrarian expert take that reframes the room.*

- **`niches.yaml` group:** `business-finance`  ·  **angle:** *not yet in CHANNEL-CONCEPTS — see note*
- **Clip from:** `@TheDiaryOfACEO` (**ROS 86.5 — your #1 source overall**) · `@TheIcedCoffeeHour`
  (net-worth reveals) · `@ImanGadzhi` · `@jayshetty`  (+ Mark Cuban/Trailblazers once the handle's
  confirmed)
- **The format (your path):** the single contrarian/authority claim → the one-sentence reframe →
  the proof. Quote-dense talking-head, captioned.
- **Hook:** the contrarian thesis, first frame.
- **Guardrails:** high-RPM authority is advertiser-safe — keep it position-based, no financial-
  advice claims.
- **Optimize on → watch-through (retention).** Authority monologues live or die on retention +
  quote-density — find the 25–35s windows that hold to the last word.

> **⚠ Decision for Eric (Channel 5).** `niches.yaml` has a 5th source group (`business-finance`)
> holding the **#1-scored source overall — Diary of a CEO (ROS 86.5)** — but `CHANNEL-CONCEPTS.md`
> only names 4 concepts, so it never got its own channel. I made it **Channel 5 "Boardroom"** so
> DOAC has a home an agent can own. Pick one: **(a)** keep it as its own channel (current), **(b)**
> fold DOAC's debate moments into Hot Seat + the reveals into Money Fights, or **(c)** bench it
> until Channels 1–2 clear their gate. Until you decide, it stays 🪑 ON DECK (not launched).

## Channel 6 — "AI Frontier" · the AI arms-race story, clipped  🟢 LAUNCHING (pivot)
*Identity: the single most shocking/contrarian AI moment from the biggest names, isolated.*
*(Replaces the old workflow-heavy "Identic Builder News" with a clip-first format.)*

- **`niches.yaml` group:** `ai-frontier`  ·  **clip queue + ranked moments:** `AI-NEWS-SOURCING.md`
- **Clip from (LOW risk, lead here):** `@TheDiaryOfACEO` (AI eps — Hinton/Bengio/Mo Gawdat/
  Yampolskiy/Galloway; their titles are pre-tested hooks) · `@DwarkeshPatel` (Dario/Ilya/Karpathy) ·
  `@lexfridman` + Lex Clips · `@ycombinator` (Karpathy) · `@ThePrimeTimeagen` + `@t3dotgg` (dev angle).
  **MED (transform hard, <25s):** `@TED`. **HIGH (avoid bulk):** 60 Minutes/CBS, Guardian, ABC, Fireship.
- **The format (your path):** cold-open *on the claim* → the shocking line as bold on-screen text in
  frame 1 (lands on mute) → the speaker delivering it → **end on the stakes + "are they right? 👇".**
  Borrowed authority is the whole game: the doom/job-loss claim comes from the MD/PhD/CEO, never us.
- **Hook:** a number + a year + the threat ("99% of jobs gone by 2030"). The single most shocking line, first frame.
- **The arc / programming:** builders-are-scared (Dario/Hinton/Bengio/Hassabis) → your-job-is-next
  (jobs apocalypse) → the-timeline (AGI dates) → **plant a foil** (LeCun "they're all wrong") to farm
  the comment war → the dev angle (is coding dead?). Mix doom with a contrarian foil deliberately.
- **Guardrails:** transform every clip (cut + hook + captions or defer to theirs — RULE #1, never two
  caption tracks) · no JRE / avoid-list · prefer LOW Content-ID sources, transform MED hard. Claims are
  the *speaker's* (borrowed authority) — present "X said," don't assert it as our fact.
- **Optimize on → comment-rate + shares** (doom/debate is comment-driven, like Hot Seat); watch-through
  as the retention check. Greenlight also if comment-rate >2× baseline at modest views.

---

## How this maps to the rest of the repo
| In this playbook | Lives in / feeds |
|---|---|
| Channel → its sources | `config/niches.yaml` (group `name:` = the channel) |
| Source ROS scores + live velocity | `SOURCE-INTELLIGENCE.md` |
| The angle rationale + format detail | `CHANNEL-CONCEPTS.md` |
| Results → which combos win → re-rank | the DB → `ycp brief` → `ycp scoreboard` |
| Avoid-list (a gate that runs *before* any score) | `SOURCE-INTELLIGENCE.md` §Avoid |
