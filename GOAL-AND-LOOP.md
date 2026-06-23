# Goal + Optimization Loop — "Any operator clears $15K/month"

This is the self-hardening engine for the clipping operation. The **Goal** is the
north star. The **Loop Prompt** is what you paste into `/loop` so Claude keeps
making the system more robust, more optimized, and more idiot-proof every cycle —
until any team member can run the play from the playbook alone and clear $15K/mo.

---

## 🎯 THE GOAL

> **Make the YouTube clipping operation robust and turnkey enough that ANY Rising
> Tides team member — with no special skill and no hand-holding — can follow
> `OPERATOR-PLAYBOOK.md` cold and clear $15,000+/month net within 90 days, and
> sustain it, without ever tripping a ban or copyright strike.**

**Done when ALL of these hold (the loop's exit criteria):**
1. **Runnable cold** — a brand-new operator runs sourcing → repurpose → QC → distribute → measure end-to-end from the playbook with zero outside questions.
2. **Every failure mode guarded** — account flag, music Content-ID, Ssemble credit cap, dead clip: each has a documented guardrail or recovery step.
3. **The loop actually compounds** — the weekly Double-Down Brief demonstrably steers sourcing, and output + hit-rate rise week over week.
4. **Economics pencil out** — a worked scorecard shows a credible path to $15K/operator/month (volume × hit-rate × rate), with at least one real example.
5. **System is green** — all `ycp` commands run clean and tests pass.

**Hard constraints (never optimize past these):** transformation on every owned-channel clip · clip-friendly / permissioned sources preferred · residential IPs + warmed accounts · no copyrighted music · **channel health > raw output**.

---

## 🔁 THE LOOP PROMPT  (paste this after `/loop`)

```
You are hardening and optimizing the Rising Tides YouTube clipping operation so ANY
team member can run it from the playbook alone and clear $15,000+/month net. Run ONE
improvement cycle, fully and verified.

NORTH STAR: a brand-new operator, no special skill, follows OPERATOR-PLAYBOOK.md cold
and reaches $15K+/month within 90 days — sustainably, inside the compliance guardrails.

INPUTS — everything is in this one repo root:
  ~/Documents/Development/youtube-clipping/
Read these first, every cycle:
- LOOP-LOG.md          (what's done + current top bottleneck)
- OPERATOR-PLAYBOOK.md (the turnkey runbook you maintain)
- YOUTUBE-CLIPPING-WORKFLOW.md, LAUNCH-CHECKLIST.md (strategy + the $15K math)
- src/ + tests/ + config/ + data/latest-brief.md (the `ycp` system and its output)
If LOOP-LOG.md or OPERATOR-PLAYBOOK.md don't exist yet, creating them is your first cycle.

EACH CYCLE, DO EXACTLY THIS:

1) ORIENT — read the inputs. State in one line what the current single biggest thing
   blocking "any operator clears $15K/month" is.

2) DIAGNOSE — scan these six lenses and pick the ONE most-binding gap right now:
   • THROUGHPUT  — is the pipeline hitting target posts/day (ramp 15→45→75→100+)?
                   Where's the choke: sourcing, editing, QC, Ssemble credit cap, posting?
   • HIT RATE    — are operators making the creator×format×hook combos the latest
                   Double-Down Brief says win? Is the brief actually steering sourcing?
   • MONETIZATION— is every owned channel wired to the monetization stack (TikTok Creator
                   Rewards + YPP + affiliate + brand deals), and is each clip transformed
                   and posted to a healthy owned channel? (These gate whether you get paid.)
   • ACCOUNT HEALTH — residential IPs, warmed accounts, no copyrighted-music Content-ID,
                   transformation on every owned clip. A ban zeroes an operator — protect
                   this ABOVE output.
   • OPERABILITY — could a first-day hire do this step from the playbook with zero
                   questions? Turn every judgment call (which campaign, which clip,
                   which format) into a checklist or a number pulled from the data.
   • ECONOMICS   — is each operator's actual $/month measured and on the $15K curve?
                   Name the exact gap (volume? hit-rate? rate? pool timing?) and the lever
                   that closes it.

3) ACT — make the ONE highest-leverage improvement, fully. Examples: write/upgrade an SOP
   in OPERATOR-PLAYBOOK.md, harden or add a guardrail to a `ycp` script, automate a manual
   step, tune a parameter from REAL data, add a test, add a recovery step for a failure
   mode, or close a compliance gap. Smallest change that moves the needle. Do NOT break
   what works. Do NOT touch credentials. Do NOT optimize in any way that risks a ban or
   copyright strike — those constraints are hard.

4) VERIFY — prove it. Run the test / `ycp` command / `ycp demo` / the math. For SOP changes,
   re-read the step as if you were a first-day hire and confirm it's unambiguous and
   self-sufficient. NEVER mark anything done you haven't verified. If you can't verify it,
   say so and treat it as not done.

5) LOG & COMPOUND — append a dated entry to LOOP-LOG.md: what changed, why, the evidence it
   works, and the NEW top bottleneck. Update the operator scorecard (target $15K/operator/mo:
   current estimate, the gap, the next lever).

6) DECIDE — if ALL exit criteria below hold, output "OPERATION IS TURNKEY — entering
   maintenance" and stop. Otherwise end by naming the next cycle's single target.

EXIT CRITERIA (all must hold):
  ✓ New operator runs the full pipeline from OPERATOR-PLAYBOOK.md with zero outside questions.
  ✓ Every known failure mode has a documented guardrail or recovery step.
  ✓ The Double-Down Brief is measurably driving sourcing; output + hit-rate rise weekly.
  ✓ A worked scorecard shows a credible path to $15K/operator/month with ≥1 real example.
  ✓ All `ycp` commands run clean; tests pass.

RULES: compliance guardrails are non-negotiable (transformation on owned clips, clip-friendly
sources preferred, residential IPs + warming, no copyrighted music, channel health > output).
Be honest in the log — if a step failed or is unverified, say so plainly. One finished,
verified improvement per cycle beats five half-done ones. Do not stop early for any reason
other than the exit criteria being met.
```

---

## ▶️ How to run it

**Phase 1 — Harden (now → turnkey):** run it self-paced so it keeps improving until the
exit criteria are met:

```
/loop <paste the loop prompt above>
```

(No interval = Claude self-paces, cycle after cycle, until it declares the operation turnkey.)

**Phase 2 — Sustain (after turnkey):** once it's idiot-proof, switch to weekly so it keeps
optimizing on each fresh Double-Down Brief (run it Monday after the brief drops):

```
/loop 1w <paste the loop prompt above>
```

**Compact one-liner** (if you want a terse version):

```
/loop One robustness+optimization cycle on the clipping op so any operator clears $15K/mo.
Repo root: ~/Documents/Development/youtube-clipping. Read LOOP-LOG.md +
OPERATOR-PLAYBOOK.md + the ycp system there; find the #1
gap (throughput / hit-rate / monetization / account-health / operability / economics); fix
ONE thing fully; verify it (run tests/ycp/the math); log it + the new bottleneck; respect
compliance guardrails; stop only when a first-day hire can run it end-to-end and the
economics show a real path to $15K/operator. One verified improvement per cycle.
```

---

## 📂 Files the loop maintains

| File | Role |
|---|---|
| `OPERATOR-PLAYBOOK.md` | The turnkey runbook a new hire follows cold. The loop's main deliverable — it gets sharper every cycle. |
| `LOOP-LOG.md` | Append-only cycle log: what changed, evidence, current #1 bottleneck, and the $15K/operator scorecard. The loop's memory. |

Inputs it reads but doesn't own: the strategy docs (`YOUTUBE-CLIPPING-WORKFLOW.md`, `LAUNCH-CHECKLIST.md`) and the live `ycp` system + weekly brief — all in this same repo root.
