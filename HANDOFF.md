# 🤝 HANDOFF — start here if you're picking this up

**You're inheriting a YouTube clipping operation.** The research foundation is built and
verified; your job is to **build the execution strategy + the automation (the "autopilot") on
top of it**, and keep the closed loop optimizing. This doc gets you fully oriented cold — read
it top to bottom, then run the quick-start checklist at the bottom.

**Owner / operator:** Eric Cromartie (Rising Tides). He wants to be *mainly hands-off* — you
lead, the system runs the channels, he taps approve and reads the scoreboard.
**North star:** **$15,000/month**, durable, on a closed-loop automated content pipeline that
feeds its own wins back in and optimizes week over week.
**Last session:** 2026-06-22 (research Cycles 1–2 + the gamified scoreboard engine shipped).

---

## 🎯 Your mission (what "build a strategy into the research" means)

The research answers **what to clip and which angles to run**. You take it from there:

1. **Operationalize the 5 channel concepts** into a concrete launch + test plan (which channels
   to stand up first, in what order, with what cadence) — respect each concept's go/no-go gate.
2. **Build the autopilot** so the daily loop runs itself (see §8 roadmap) — Eric's core ask.
3. **Keep the closed loop compounding** — once channels are live, the weekly Double-Down Brief
   produces real data; feed it back to re-rank sources/angles (the research loop already does
   this — run it weekly).
4. **Don't re-derive what's done.** The ranked sources, the scoring rubric, the concepts, and
   the scoreboard exist and are verified. Build *on* them.

---

## 📍 Current state (what's true right now)

### ✅ Built + verified
- **Research/discovery engine** — a self-hardening loop (`RESEARCH-GOAL-AND-LOOP.md`) that finds
  and ranks clip sources + content angles. **2 cycles run.**
- **33 source pages scored** on the Repurpose Opportunity Score (ROS) rubric → `SOURCE-INTELLIGENCE.md`.
  Velocity numbers are **real** (live yt-dlp, 2026-06-22).
- **5 channel concepts** with go/no-go gates → `CHANNEL-CONCEPTS.md` (one is the debate/agitation bet).
- **24 creators promoted** to `config/niches.yaml` (parses clean via `sourcing.load_creators()`).
- **Gamified scoreboard** — `ycp scoreboard` turns the closed-loop DB into a "Race to $15K"
  game (`src/ycp/scoreboard.py`, `SCOREBOARD.md`, tests pass). Run `ycp scoreboard --demo` to see
  mid-game; plain `ycp scoreboard` shows the real Day-0 state.
- **`ycp source` works** — the empty-queue bug is **fixed** (`sourcing.py`: flat mode for IDs →
  non-flat `--print` for real `view_count`/`timestamp` on the top-N per creator). Writes a
  non-empty `data/source-queue.md`.
- **Autopilot orchestrator built** — `ycp autopilot` chains `source → clip → qc → capture →
  brief → scoreboard` (flags `--skip-source` / `--no-clip`); `scripts/autopilot.sh` + the
  launchd plist wrap it for cron.
- The full `ycp` pipeline skeleton: `source · clip · qc · capture · brief · demo · scoreboard · autopilot`.

### ❌ Not built yet (your work)
- **Distribution** — posting to YouTube/TikTok/IG is NOT wired. Target = **Repurpose.io** (§9
  resolved; Eric is trialing it). **A human must connect accounts once** in its dashboard.
- **Live channels** — zero channels exist yet. Game state = **Day 0, Level 1 "Boot Up", $0.**

---

## 🧭 Strategic frame — decisions already made (don't relitigate without Eric)

1. **Pure owned-first. Whop is CUT** (§9 resolved, 2026-06-22). The core (and only) asset =
   Eric's own faceless channels (he owns them, they appreciate, they run ~hands-off), monetized by
   **YouTube Partner Program + TikTok Creator Rewards + affiliate + (later) direct brand deals.**
   Whop clipping campaigns are **removed entirely** — gig work where you own nothing, pools dry up,
   and payouts legally require Eric's identity (un-automatable). *Cleanup done (2026-06):* the
   Whop-payout path in `src/ycp/capture.py` and Whop references across the system were stripped.
   *(Original docs were Whop-first; reconciled owned-first 2026-06-22, then Whop cut entirely.)*
2. **Single-lane now** — only Lane 2 (owned / YPP asset) remains. Lane 1 (permissioned/cash via
   Whop) is gone, not just demoted.
3. **Mega-creators are a trap — velocity ≠ ownable.** Session ruling, with real numbers:
   - **MrBeast** (270K views/hr 🤯, ~5× our #1 pick) → **turbo-only**, and *only* via his official
     Vyro clip campaign (permissioned). Zero headroom as an owned lane (everyone clips him);
     spectacle doesn't pull standalone 30s hooks.
   - **IShowSpeed** (128K/hr), **Kai Cenat** (33K/hr) → **AVOID.** Content-ID minefield
     (music / reactions / licensed sports & game IP). A strike pattern kills a faceless network.
   - **Taylor Swift** (24K/hr) → **AVOID.** It's music = the most aggressive Content-ID on Earth +
     no monetization path. The "music is the silent killer" rule, personified.
   - **The lesson, baked into the rubric:** the avoid-list is a *gate that runs before the score.*
     Ask "can I keep this channel alive and own it?" before "how viral is it?" That's why the
     ranked list is "boring" podcasts/debate (quotable, clean audio, open lanes), not the biggest
     names alive.
4. **Compliance is non-negotiable** (these protect the whole operation — see §10).

---

## 🗂️ File map

| File | What it is | Maintained by |
|---|---|---|
| **HANDOFF.md** | This doc — the cold-start entry point | whoever drives |
| `README.md` | Repo overview | — |
| `RESEARCH-GOAL-AND-LOOP.md` | The discovery loop: goal + `/loop` prompt + the ROS rubric | discovery loop |
| `SOURCE-INTELLIGENCE.md` | **The 33 ranked clip sources** (the "good list") + campaign board + avoid-list | discovery loop |
| `CHANNEL-CONCEPTS.md` | **The 5 content angles to test**, each with a go/no-go gate | discovery loop |
| `RESEARCH-LOG.md` | Discovery cycle log + scorecard (Cycles 1–2) | discovery loop |
| `SCOREBOARD.md` | The gamified "Race to $15K" game state (auto-generated) | `ycp scoreboard` |
| `config/niches.yaml` | **Live sourcing spec** — 24 validated creators (feeds `ycp source`) | discovery loop → ops |
| `AUTOPILOT-GOAL-AND-LOOP.md` | The **autopilot** loop: builds the self-running machine — executes the §8 roadmap | autopilot loop |
| `AUTOPILOT-LOG.md` | Autopilot build log + human-touch count + autonomy bottleneck (created on first cycle) | autopilot loop |
| `GOAL-AND-LOOP.md` | The **ops** loop: hardens *how the operation runs* (human-runnable fallback) | ops loop |
| `LOOP-LOG.md` | Ops cycle log + the $/operator scorecard | ops loop |
| `OPERATOR-PLAYBOOK.md` | Turnkey runbook a human operator follows | ops loop |
| `YOUTUBE-CLIPPING-WORKFLOW.md` | Full strategy + money math (reconciled owned-first, 2026-06-22) | — |
| `LAUNCH-CHECKLIST.md` | 30-day setup do-it order | — |
| `SSEMBLE-PARITY.md` | Maps Ssemble features → owned local tools (yt-dlp/ffmpeg/whisper) | ops loop |
| `src/ycp/` | The system (see §6) · `tests/` · `scripts/` (cron) | ops loop |

---

## ⚙️ The `ycp` system (how to run + verify)

Python package, installed **non-editable** (the spaced folder name breaks editable installs).
**If you edit `src/ycp/`, re-run `./scripts/setup.sh` (or `uv pip install . --reinstall-package
youtube-clipping`) or the `ycp` command won't see your change.** Tests run against live src via
pyproject `pythonpath`, so `pytest` reflects edits without reinstall.

```bash
cd ~/Documents/Development/youtube-clipping
.venv/bin/python -m ycp demo          # seed demo data + print a Double-Down Brief (no creds)
.venv/bin/python -m ycp scoreboard --demo   # see the game mid-run (Level 5, $7.3K/mo)
.venv/bin/python -m ycp scoreboard    # the REAL Day-0 state ($0, Level 1)
.venv/bin/python -m ycp source        # Stage 1 — writes a non-empty ranked queue (~2–3 min, live yt-dlp)
.venv/bin/python -m ycp clip <url>    # Stage 2 — hybrid yt-dlp+whisper+ffmpeg vertical clips
.venv/bin/python -m ycp autopilot --skip-source --no-clip   # chain all stages end-to-end (7/7)
```

**Verify before claiming anything done:** `ruff check src tests` (clean) · `pytest -q` (**72 pass,
~1s** on a real machine with ffmpeg). The `cut_vertical` ffmpeg smoke is auto-skipped when ffmpeg
is absent; it only *hung* under the agent sandbox — on a real Mac plain `pytest` runs it fine
(use `-k "not cut_vertical"` only if you're in a sandbox).

**The closed loop (the DB is the single source of truth):**
`source → clip → qc (Slack ✅/❌) → distribute → capture (views/$) → brief (scale/kill) → scoreboard → re-source to the brief.`

---

## 🔁 The two loops (this is the engine)

- **Discovery loop** (`RESEARCH-GOAL-AND-LOOP.md`) — finds & ranks **what to clip / which angles**.
  Promotes winners to `niches.yaml`. Paste its prompt into `/loop` to run a cycle. Once channels
  are live, it reads `data/latest-brief.md` and re-ranks on real results (weekly).
- **Ops loop** (`GOAL-AND-LOOP.md`) — hardens **how the operation runs** so any operator clears
  the number from `OPERATOR-PLAYBOOK.md`.
- **Together they close the circuit:** discovery seeds bets → ops runs them → the Double-Down
  Brief measures real $ → discovery re-ranks → repeat. That's the week-over-week compounding.

---

## 🔨 What to build next (the autopilot roadmap)

> **This roadmap is driven by `AUTOPILOT-GOAL-AND-LOOP.md`.** Paste its loop prompt into `/loop` to
> execute these steps autonomously — one verified build per cycle — until the factory runs itself.

Do these roughly in order. **#1 + #2 are done (✅ below) — your live work starts at #3**, which
needs Eric's accounts connected once.

1. ~~**Fix `ycp source`**~~ ✅ **DONE.** The empty-queue bug is fixed — `sourcing.py` uses flat
   mode for IDs → non-flat `--print` (`%(view_count)s`/`%(timestamp)s`) on the top-N recent per
   creator. `ycp source` now writes a non-empty `data/source-queue.md`; `pytest` + `ruff` green.
2. ~~**The orchestrator**~~ ✅ **DONE.** `ycp autopilot` (and `scripts/autopilot.sh` + the
   launchd plist) chains `source → clip → qc → capture → brief → scoreboard` end-to-end.
3. **Wire distribution → Repurpose.io** (§9 resolved). Build a thin, swappable adapter to its
   watch-folder / cloud-trigger model: the `distribute` stage drops each approved clip + metadata
   into the watched source and Repurpose auto-posts to the connected channels. **Eric connects
   accounts once** in the Repurpose dashboard; after that, posting is automated. Keep it loosely
   coupled (he's trialing the tool, not married to it). **Gate:** QC is manual (§9) — only clips
   Eric ✅'d in Slack reach distribution. The in-code guardrail filters (build #6) run as
   defense-in-depth behind that human gate.
4. **Launch the first channels.** **Concept 1 "Hot Seat" (debate)** + **Concept 2 "Money Fights"
   (finance)** (§9 resolved) — top velocity + highest open-lane EV. Run each concept's go/no-go
   gate (in `CHANNEL-CONCEPTS.md`). Clearing First Blood → Signal on the scoreboard.
5. **Then let the loops compound** weekly; the scoreboard tracks the climb to $15K.

---

## ✅ Open questions — RESOLVED by Eric (2026-06-22)

All four §9 decisions are made. Build to these; do **not** re-ask.

- **Posting → Repurpose.io** (NOT Ssemble). Eric is trialing [repurpose.io](https://repurpose.io)
  for a few weeks alongside content production. Build distribution as a **thin adapter** to its
  watch-folder / cloud-trigger model (drop approved clip + metadata into the watched source →
  Repurpose auto-posts to connected channels). Keep it **loosely coupled** so it's swappable —
  Eric is "not that dependent on it long-term." One-time human step = connect accounts in the
  Repurpose dashboard (matches the one-time-auth constraint exactly). *(Affects build 3.)*
- **Launch order → Hot Seat + Money Fights** first (Concept 1 + Concept 2), per the recommendation.
- **Whop → CUT ENTIRELY.** Pure owned-first. Whop was stripped from the system (2026-06): the
  Whop-payout path in `src/ycp/capture.py` and Whop references across the docs are removed. No turbo lane.
- **QC → MANUAL Slack review** (revised 2026-06-22; was briefly "auto"). Every clip is posted to
  the Slack QC channel and **nothing advances to distribution until Eric ✅ it** — he oversees the
  content to make sure it's done right. Flip `qc.auto: true` ONLY once the output has earned trust
  over time ("once I see you're doing the right thing then I'll let it run autonomously"). The
  in-code guardrail filters still run as **defense-in-depth**, not as the sole gate.
- **Hook agent → DeepSeek.** Viral hook titles are written by a strong model via DeepSeek
  (`src/ycp/hooks.py`), NOT a local model. Key lives in 1Password → injected as `DEEPSEEK_API_KEY`
  via `op read` (never on disk; set `DEEPSEEK_OP_REF`). Falls back to a transcript heuristic if
  absent. *(History: Ollama was wrong for this — too weak for viral hooks; removed.)*

---

## 🚧 Guardrails (non-negotiable — these protect the whole operation)

- **Transform every owned-channel clip** (your cut/hook/commentary) — raw reuploads get a channel
  demonetized *channel-wide* under YouTube's inauthentic-content policy.
- **No copyrighted music** — Content-ID claims it instantly. The silent killer.
- **Respect the avoid-list** (`SOURCE-INTELLIGENCE.md`) — it's a *gate*, not a deduction. JRE,
  Tate, Huberman (terms), music/casino/react-meta creators, and the mega-creators in §Strategic
  frame #3 are disqualifying.
- **Agitation angles drive *opinion*, never hate** — attack the position/behavior, never a
  protected group/person. Spicy = monetizable; hateful/harassing = struck. (See `CHANNEL-CONCEPTS.md`
  Concept 1 + `SOURCE-INTELLIGENCE.md` for the exact policy line.)
- **Channel health > raw output** — a ban zeroes the operator. Protect accounts above volume.
- **Real numbers only** — every score/claim cites real data (live velocity, a funded pool, a
  named comparable win) or is marked unvalidated. Be honest in the logs.
- **Verify before "done"** — run the test / the command / the math. Don't break what works.

---

## ✅ Quick-start checklist (your first 30 minutes)

1. Read this doc, then `RESEARCH-GOAL-AND-LOOP.md` (the rubric), `SOURCE-INTELLIGENCE.md` (the
   sources), `CHANNEL-CONCEPTS.md` (the angles).
2. Run `.venv/bin/python -m ycp scoreboard --demo` — see the game working.
3. Run `.venv/bin/python -m ycp demo` — see the closed loop produce a Double-Down Brief.
4. Skim `src/ycp/sourcing.py` + `scoreboard.py` + `brief.py` to learn the system's shape.
5. Confirm the §9 open questions with Eric.
6. Start at **autopilot roadmap #3** (wire distribution → Repurpose.io) — #1 (source fix) and #2
   (orchestrator) are already done. Then launch Hot Seat + Money Fights (#4).

> Welcome aboard. The research is solid and the game is set — now make the engine run itself. 🏁
