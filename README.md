# YouTube Clipping Operation

One self-contained repo. Everything for the play lives here.

### 📋 Strategy & math
- **[YOUTUBE-CLIPPING-WORKFLOW.md](YOUTUBE-CLIPPING-WORKFLOW.md)** — full strategy, the
  two-lane model, the $10K math, A-Z pipeline, 90-day roadmap, risk register.
- **[LAUNCH-CHECKLIST.md](LAUNCH-CHECKLIST.md)** — the 30-day setup do-it order for Jay.

### 🏃 Run the play
- **[OPERATOR-PLAYBOOK.md](OPERATOR-PLAYBOOK.md)** — the turnkey runbook. Anyone on the team
  follows this to run the play and clear $10K/month. Start here if you're operating.
- **[GOAL-AND-LOOP.md](GOAL-AND-LOOP.md)** — the goal + the `/loop` prompt that keeps hardening
  the operation until it's robust and idiot-proof. Paste it into `/loop` to run the engine.
- **[LOOP-LOG.md](LOOP-LOG.md)** — what the loop has improved + the live $10K/operator scorecard.

### ⚙️ The `ycp` system
`source` (ranked clip queue, yt-dlp, no key) · `qc-post`/`qc-listen` (Slack approval) ·
`capture` (views + Whop payouts) · `brief` (weekly Double-Down Brief) · `demo` (see it work).
Code in `src/ycp/`, tests in `tests/`, config in `config/`, cron in `scripts/`.

### See it work in 10 seconds
```bash
cd ~/Desktop/Development/"Youtube Clipping Workflow"
uv venv && uv pip install -e '.[dev]'   # first time only
uv run ycp demo
```
Prints a real Double-Down Brief from demo data — the closed loop that tells you which
creator × format × hook to double down on.

### The model in one line
Clip big creators → post across many channels → **Whop bounties pay you now** (~30–75× the
YouTube ad rate) while **owned channels grow toward YouTube monetization**. Whop = paycheck,
owned channels = savings. Transformation + account health keep it all alive.

---
*Standalone git repo (its own root — not part of the home/viral-course repo). Code lives here
per the project's home in `Desktop/Development/`.*
