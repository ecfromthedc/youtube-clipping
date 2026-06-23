# Autopilot Goal + Build Loop — "Make the factory run itself"

Two loops already exist: the **discovery loop** (`RESEARCH-GOAL-AND-LOOP.md`) finds *what to clip*,
and the **ops loop** (`GOAL-AND-LOOP.md`) hardens *how a human runs it*. This is the third and now
the **primary** one: it **builds the autonomous execution** — it takes the verified research + the
`ycp` skeleton and turns them into a content factory that sources, clips, QCs, posts, measures, and
re-ranks itself on a schedule, so Eric is **mainly hands-off**. It executes the autopilot roadmap in
`HANDOFF.md §8`.

Same shape as the other two: a **Goal** (north star + exit criteria) and a **Loop Prompt** you paste
into `/loop`. It runs build cycle after build cycle until the machine runs itself — then drops to
weekly maintenance that keeps tightening autonomy.

> Read `HANDOFF.md` first — it is the cold-start source of truth this loop builds on.

---

## 🎯 THE GOAL

> **Turn the verified research + the `ycp` skeleton into a fully autonomous YouTube clipping factory:
> a closed loop that sources → clips → enhances → QCs → distributes → measures → re-ranks itself
> on a schedule, so the only recurring human touch is a quick Slack ✅/❌ review of each batch (plus a
> one-time channel auth), while the system runs the channels and climbs the scoreboard to $15,000/month.**
> *(QC is MANUAL per Eric — §9: he reviews content in Slack before it posts, until the output earns
> autonomy. The in-code guardrail filters run as defense-in-depth behind that human gate.)*

**Done when ALL of these hold (the loop's exit criteria):**
1. **The queue is real** — `ycp source` writes a non-empty `data/source-queue.md` from `config/niches.yaml` (HANDOFF §8 #1 fixed).
2. **The loop is one command on a timer** — an orchestrator (`ycp autopilot` / `scripts/autopilot.sh`) chains `source → clip → qc-post → capture → brief → scoreboard` (clip does the enhancement stages — captions/hook/CTA/gameplay — via its flags; brief does the scoring; there's no `ycp enhance`/`ycp score` subcommand; `distribute` slots in once build #3 is wired), runs end-to-end without errors on a real invocation, and is scheduled (cron/launchd). *(Note: `source`/`clip` hit the live network + real machine, so there's no fully-offline demo of the whole chain.)*
3. **Posting is automated after a one-time auth** — distribution is wired to **Repurpose.io** (§9 resolved); a human connects accounts once in its dashboard, then approved clips post themselves via its watch-folder / cloud trigger. Account *creation* stays human (deliberate).
4. **Channels are live** — ≥1 owned channel is launched (Concept 1 "Hot Seat" + Concept 2 "Money Fights", §9 resolved) and running its go/no-go gate; first real clips are posted; the scoreboard moves off Day 0 (First Blood → Signal).
5. **It runs unattended between approvals** — end-to-end with no babysitting; the one recurring human action is the **Slack ✅/❌ review** (§9), until Eric flips to auto once trust is earned.
6. **Guardrails get enforced in code, not just docs** — an avoid-list gate inside sourcing (net-new; none exists today) and a no-music / transformation check before publish. These run as **defense-in-depth** behind the human QC gate (and become the primary gate if/when QC flips to auto).

**Hard constraints (never optimize past these):** all of `HANDOFF.md §10` (transform every owned clip
· no copyrighted music · avoid-list is a *gate* before the score · agitation drives opinion never hate
· channel health > raw output · real numbers only · verify before done) **plus**: never automate
account creation (automating it is the bannable path — keep it a one-time human step) · the §9 open
questions are RESOLVED (posting → Repurpose.io · launch Hot Seat + Money Fights · Whop cut · QC MANUAL
via Slack · hooks → DeepSeek) — build to them, don't re-ask · **QC is manual: clips post only after
Eric ✅'s them in Slack; the in-code guardrail filters run as defense-in-depth, not the sole gate.**

---

## 🔁 THE LOOP PROMPT  (paste this after `/loop`)

```
You are the autopilot engineer for the Rising Tides YouTube clipping operation. Your job: turn the
verified research + the ycp skeleton into a content factory that RUNS ITSELF on a schedule, so the
owner is mainly hands-off. Run ONE build-and-harden cycle, fully and verified.

NORTH STAR: a closed loop that sources → clips → enhances → QCs → distributes → measures →
re-ranks on a cron, the recurring human touch being a quick Slack ✅/❌ review (+ one-time channel auth),
climbing the scoreboard to $15,000/month.

§9 OPEN QUESTIONS — RESOLVED by Eric 2026-06-22 (build to these, do NOT re-ask):
  • Posting → Repurpose.io (NOT Ssemble). Thin, swappable watch-folder/cloud-trigger adapter; Eric is trialing it.
  • Launch order → Hot Seat + Money Fights first.
  • Whop → CUT ENTIRELY (pure owned-first; Whop-payout path in capture.py + Whop refs in docs stripped 2026-06).
  • QC → MANUAL Slack review. Clips post to the Slack QC channel; nothing reaches distribution until
    Eric ✅'s it ("once I see you're doing the right thing then I'll let it run autonomously"). The
    in-code guardrail filters (build #6) run as defense-in-depth behind the human gate. Flip qc.auto
    to true only once trust is earned.
  • Hooks → DeepSeek (strong model, key in 1Password → DEEPSEEK_API_KEY via `op read`; heuristic fallback).

READ FIRST, EVERY CYCLE — repo root:
  ~/Documents/Development/youtube-clipping/
- HANDOFF.md            (the cold-start source of truth: state, strategic frame §3, build order §8, guardrails §10, open questions §9)
- AUTOPILOT-LOG.md      (what you've built + the current #1 autonomy bottleneck — your memory)
- SCOREBOARD.md / run `.venv/bin/python -m ycp scoreboard`  (the game state — Day/Level toward $15K)
- CHANNEL-CONCEPTS.md + SOURCE-INTELLIGENCE.md + config/niches.yaml  (what to launch / what to clip)
- data/latest-brief.md  (real results once channels run — created by `ycp brief`; git-ignored and
                         absent until channels run, so its absence on a fresh checkout is normal)
If AUTOPILOT-LOG.md doesn't exist, creating it (with the real Day-0 baseline) is part of cycle 1.

BUILD ORDER (HANDOFF §8 — follow it, but always attack the current autonomy bottleneck):
  1. FIX `ycp source` (unblocks the live queue). Bug: sourcing._ytdlp_json uses yt-dlp
     --flat-playlist, which now returns view_count=None for channel /videos tabs, so rank() drops
     everything at min_views (50000 in config/settings.yaml). PROVEN FIX: keep flat mode for IDs, then non-flat `--print`
     (%(view_count)s;%(timestamp)s;...) on the top-N recent per creator. Do NOT rip out flat mode
     (it's fast / no-key). Keep parse_entries + rank as-is (they already read view_count/timestamp).
     Spawned task id: task_f81d84f1. Verify: `ycp source` writes a NON-EMPTY data/source-queue.md.
  2. ORCHESTRATOR — one command/cron (`ycp autopilot` or scripts/autopilot.sh) that chains
     source → clip → qc-post → capture → brief → scoreboard (clip already does the enhancement stages
     — captions/hook/CTA/gameplay — via its flags; brief does the scoring; there is NO `ycp enhance`
     or `ycp score` subcommand; `distribute` slots in once build #3 is wired). scripts/daily.sh +
     weekly.sh are starting points. Make it idempotent, safe to re-run, and log each stage.
  3. DISTRIBUTION → Repurpose.io (§9 RESOLVED — build it; the one-time human action is connecting
     accounts in the Repurpose dashboard, not a decision to escalate). Wire the approved-clip handoff
     as a thin, swappable adapter to its watch-folder/cloud-trigger model (drop approved clip +
     metadata → it auto-posts to connected channels). GATE: QC is manual — only clips Eric ✅'d in
     Slack reach distribution; the build #6 filters run as defense-in-depth behind that. A human
     connects accounts ONCE; after that, approved clips post automatically. NEVER automate account
     creation. Keep it loosely coupled — Eric is trialing the tool.
  4. LAUNCH FIRST CHANNELS — recommended Concept 1 "Hot Seat" (debate) + Concept 2 "Money Fights"
     (finance) per CHANNEL-CONCEPTS.md; run each concept's go/no-go gate. Moves the scoreboard off Day 0.
  5. COMPOUND — once live, the daily loop + weekly brief feed the discovery loop; the scoreboard tracks the climb.

EACH CYCLE, DO EXACTLY THIS:

1) ORIENT — read the inputs. State in one line the single biggest thing blocking "the factory runs
   itself" right now (e.g. "source queue empty," "no orchestrator," "distribution un-wired — waiting
   on Eric's posting call," "channels not launched").

2) PICK — choose the build-order step that removes that bottleneck. If it's a human-gated step
   (distribution) or an open question (§9), DON'T guess — escalate (step 7) and instead pick the most
   valuable thing you CAN finish autonomously this cycle.

3) BUILD — implement that ONE step fully. Smallest change that advances autonomy. Respect the strategic
   frame (owned-first; the avoid-list is a GATE that runs before the score; mega-creators per HANDOFF §3
   are turbo-only or AVOID) and the guardrails (§10). Enforce guardrails IN CODE where feasible (avoid-
   list gate inside sourcing; a no-music / transformation check before publish) — not just in docs.
   If you edit src/ycp, remember the NON-EDITABLE install: re-run ./scripts/setup.sh (or
   `uv pip install . --reinstall-package youtube-clipping`) before testing the `ycp` command.
   (pytest reflects src edits WITHOUT reinstall via pyproject pythonpath — so if tests pass but
   `ycp source` still returns empty, you forgot to reinstall. Cycle 1 edits sourcing.py, so this bites first.)

4) VERIFY — prove it. Run the ACTUAL command (e.g. `ycp source` → non-empty queue; `ycp autopilot`
   → chains clean end-to-end on a real run — source/clip hit the network, so there's no fully-offline
   demo of the whole chain) AND `ruff check src tests` + the NAMED test subset:
   `.venv/bin/python -m pytest tests/test_scoreboard.py tests/test_scoring.py tests/test_db.py tests/test_sourcing.py -q`
   (plain `pytest` HANGS on an ffmpeg smoke test in the sandbox — run the named subset or
   `-k "not cut_vertical"`). ffmpeg/whisper paths validate on the real machine. Never mark done what
   you have not actually run.

5) REDUCE HUMAN TOUCH — state explicitly which human steps remain after this change, and confirm the
   trend is toward {one-time channel auth in Repurpose.io + a quick Slack ✅/❌ per batch} (QC is
   manual per §9 until trust is earned).
   Anything else automatable-safely → queue it as a future cycle. (Account creation stays human —
   that's deliberate, not a gap.)

6) LOG & COMPOUND — append a dated entry to AUTOPILOT-LOG.md: what shipped, the evidence it works, the
   remaining human-touch count, and the NEW #1 autonomy bottleneck. Re-run `ycp scoreboard` and note
   the game state (Day / Level / $).
   Then COMMIT CLEANLY: stage ONLY the paths you changed this cycle, by explicit name
   (`git add path/a path/b`) — NEVER `git add -A` / `git add .` / `git commit -am`, since Eric may be
   editing this repo live and a blanket add bundles his work into your commit under the wrong message
   (this has happened). If `git status --porcelain` shows files you didn't touch, leave them unstaged.
   One conventional commit scoped to your change, then push.

7) ESCALATE — the §9 questions are RESOLVED (don't re-ask). Escalate ONLY a one-time human action
   (connect accounts in Repurpose.io, create/auth a channel) or a genuinely new decision not covered
   by §9 — surface it in ONE crisp line at the TOP of your output and stop on that item. Don't guess
   past a decision that's his.

8) DECIDE — if ALL exit criteria below hold, output "AUTOPILOT LIVE — entering compounding maintenance"
   and stop. Otherwise end by naming the next cycle's single target.

EXIT CRITERIA (all must hold):
  ✓ `ycp source` writes a real non-empty data/source-queue.md from niches.yaml.
  ✓ An orchestrator chains source→clip→qc-post→capture→brief→scoreboard (no `ycp enhance`/`score` — clip + brief cover those), runs clean end-to-end on a real invocation, and is scheduled (cron/launchd).
  ✓ Distribution wired to Repurpose.io (§9); after one-time account auth in its dashboard, approved clips post automatically.
  ✓ ≥1 owned channel launched + running its go/no-go gate; first real clips live; scoreboard off Day 0.
  ✓ The loop runs unattended end-to-end between approvals; recurring human touch = the Slack ✅/❌ review (§9).
  ✓ Guardrails in code — an avoid-list gate inside sourcing (net-new; none today) + a no-music / transformation check before publish. These run as defense-in-depth behind the manual QC gate.

RULES: real numbers only · channel health > raw output · transform every owned clip · no copyrighted
music · respect the avoid-list as a gate · agitation drives opinion never hate · NEVER automate account
creation · §9 is resolved (Repurpose.io · Hot Seat + Money Fights · Whop cut · QC MANUAL via Slack ·
hooks → DeepSeek) — build to it · QC is manual: clips post only after Eric ✅'s them in Slack · verify
before "done"; don't break what works. One finished, verified step per cycle beats five half-built ones.
Stop only when the exit criteria are met or you're genuinely blocked on a one-time human action.
```

---

## ▶️ How to run it

**Phase 1 — Build the autopilot (now → it runs itself):** self-paced, cycle after cycle, until the
exit criteria are met:

```
/loop <paste the loop prompt above>
```

(No interval = Claude self-paces until it declares "AUTOPILOT LIVE.")

**Phase 2 — Maintain + tighten (after it's live):** weekly, to keep the orchestrator healthy and keep
*reducing* residual touch / risk as filters earn trust (e.g. tighten the auto-QC filters, add channels):

```
/loop 1w <paste the loop prompt above>
```

**Compact one-liner** (terse version):

```
/loop One autopilot build cycle for the clipping op. Repo root: ~/Documents/Development/youtube-clipping.
Read HANDOFF.md (state/frame/§8 build order/§9 RESOLVED decisions/§10 guardrails) +
AUTOPILOT-LOG.md + ycp scoreboard. Find the #1 thing blocking "the factory runs itself"; build the next
§8 step that removes it (1: fix ycp source [flat→non-flat top-N: flat for IDs, then non-flat --print
%(view_count)s;%(timestamp)s on top-N recent per creator], 2: orchestrator ycp autopilot, 3: distribution
→ Repurpose.io watch-folder adapter [behind the manual Slack QC gate], 4: launch Hot
Seat + Money Fights); §9 resolved (Repurpose.io · Hot Seat+Money Fights · Whop CUT · QC MANUAL via Slack
· hooks DeepSeek) so don't re-ask; enforce guardrails in code (avoid-list gate + no-music/transformation);
VERIFY by running the command + ruff + the named pytest subset (plain pytest hangs); reinstall after src
edits; reduce human touch toward {one-time auth in Repurpose}; log it + the new bottleneck + scoreboard;
escalate only a one-time human action. Never automate account creation. One verified step per cycle.
```

---

## 📂 Files / artifacts the loop maintains

| Artifact | Role |
|---|---|
| `AUTOPILOT-LOG.md` | Append-only build log: what shipped, evidence, human-touch count, the current #1 autonomy bottleneck, scoreboard state. The loop's memory. |
| `scripts/autopilot.sh` / `ycp autopilot` | The orchestrator — chains the daily loop end-to-end on a schedule. The loop's primary build. |
| distribution wiring | The approved-clip → Repurpose.io handoff (thin watch-folder/cloud-trigger adapter), behind a one-time human auth. |
| channel-launch tracker | Which concepts are live + where each sits in its go/no-go gate (in AUTOPILOT-LOG.md). |

Inputs it reads but doesn't own: `HANDOFF.md`, `CHANNEL-CONCEPTS.md`, `SOURCE-INTELLIGENCE.md`,
`config/niches.yaml`, `data/latest-brief.md`, and the `ycp` system.

---

## 🔗 How the three loops close the circuit

```
  DISCOVERY loop  →  what to clip / which angles  →  promotes winners to config/niches.yaml
                                                          │
                                                          ▼
  THIS (AUTOPILOT) loop  →  builds + schedules the self-running machine, wires posting, launches channels
                                                          │
                                                          ▼
        the machine runs unattended:  source → clip → enhance → QC → distribute → capture
                                                          │
                                                          ▼
                       ycp brief + ycp scoreboard  →  real $ + the climb to $15K
                                                          │
                                                          ▼
            DISCOVERY re-ranks on the real brief  ───────┘   (week-over-week compounding)
```

Discovery finds the fuel · **this loop builds the engine that burns it without a driver** · the ops
loop (`GOAL-AND-LOOP.md`) keeps it human-runnable as a fallback. Eric's job: a quick Slack ✅/❌ review
of each batch + read the scoreboard (and one-time account auth in Repurpose when a channel launches) —
until the output earns autonomy.

> Sister docs: `RESEARCH-GOAL-AND-LOOP.md` (what to clip) · `GOAL-AND-LOOP.md` (human operability).
> This loop is the one that removes the human from the daily run.
