# Operator Playbook — run the clipping play, clear $10K/month

**Who this is for:** anyone on the team running clipping. No special skill required.
Follow it top to bottom. If a step makes you ask a question, that's a bug — flag it
and it gets fixed in the next loop cycle.

**The one-sentence model:** clip viral moments from big creators → post across many
channels → get paid by Whop bounties NOW (cash) while your owned channels grow toward
YouTube monetization (asset). Whop pays ~30–75× more per view than YouTube ad revenue,
so **Whop is your paycheck and owned channels are your savings account.**

> Maintained by the optimization loop (`GOAL-AND-LOOP.md`). Gets sharper every cycle.

---

## 0. Your $10K/month scorecard (know your numbers)

You hit $10K when **views × rate** clears it. Whop-first, the target is reachable on
~5–10M views/month — not the 200M+ that pure YouTube ad revenue would need.

| Lever | Month 1 target | Month 2 | Month 3 (≈$10K) |
|---|---|---|---|
| Channels live (warmed) | 8–12 | 12–15 | 12–15 |
| Posts/day (across YT+TikTok+Reels) | 15–30 | 45–75 | 75–100 |
| Whop campaigns active | 5–10 | 8–12 | 10–15 |
| Monthly views (all channels) | 1–2M | 3–6M | **6–10M** |
| Realized $/1K (Whop blended) | ~$1.00 | ~$1.25 | ~$1.50 |
| **Est. monthly income** | **$1–2K** | **$3–5K** | **$8–12K** |

**Check yourself weekly:** open `data/latest-brief.md`. If your views aren't climbing,
the bottleneck is one of: volume (post more), hit-rate (make the 🟢 combos), or
monetization (more funded campaigns, submitted in-window). Fix that one thing.

---

## 1. Your daily routine (≈90 min, then the system runs)

1. **Pull the queue** — `cd ~/Desktop/Development/"Youtube Clipping Workflow" && ./scripts/daily.sh`
   (or `uv run ycp source`). Open `data/source-queue.md` — these are today's highest-
   velocity videos to clip, ranked. Work top-down.
2. **Clip** — run each source video through Ssemble AI clipping (or the hybrid pipeline,
   §4). Target 5–10 clips per source video.
3. **Transform owned-channel clips** (§3 guardrails) — add your hook, cut, and a take.
   Whop clips can stay raw; owned-channel clips MUST be transformed.
4. **Approve** — clips post to the Slack `#clip-qc` channel automatically. React ✅ to
   approve, ❌ to reject. Run the §2 checklist on each.
5. **Distribute** — approved clips schedule through Ssemble to all channels at best times.
   Hands-off after approval.
6. **Submit to Whop** — for every clip from a Whop-campaign creator, submit the posted
   link to that campaign **within the submission window** (§5). This is how you get paid.
7. Done. Capture + scoring run on cron; the weekly brief tells you what to double down on.

---

## 2. The QC approval checklist (seconds per clip)

Before you react ✅ in Slack, every clip must pass:
- [ ] **Hook lands in the first 1–2 seconds** (or it's dead on Shorts).
- [ ] **Transformation present** (owned-channel clips only — your cut/hook/take).
- [ ] **No copyrighted music bed** — strip/replace it. Music = instant Content-ID claim.
- [ ] **Captions accurate and readable.**
- [ ] **Right channel + niche fit.**
- [ ] **Whop rules met** (if applicable): correct platform, length, required tags, 9:16.

If it fails any box → ❌ reject (or send back for a fix). One bad owned-channel clip can
demonetize the whole channel.

---

## 3. Guardrails — these protect your income (never skip)

| Rule | Why |
|---|---|
| **Transform every owned-channel clip** (your hook/cut/commentary) | Raw reuploads get the channel demonetized *channel-wide* under YouTube's inauthentic-content policy. |
| **Permissioned / Whop sources first** | Zero copyright-strike risk; you're paid to post their content. |
| **Residential IPs, warm accounts 7–14 days, create gradually** | Datacenter-VPN bursts get whole account networks banned together. |
| **No copyrighted music** | Content-ID finds it instantly → claims/strikes. |
| **Channel health > raw output** | A ban zeroes an operator. Protect the accounts above all. |

---

## 4. Tools

- **Ssemble (Business plan)** — AI clipping (finds viral moments, captions, face-tracking,
  hook titles) + calendar auto-post to unlimited connected accounts. Your repurpose +
  distribution engine. ⚠️ Credits are ~2,600/year (~7/day) — confirm what 1 credit buys;
  if it caps your volume, use the hybrid pipeline below.
- **Hybrid pipeline (free, uncapped — for volume)** — `yt-dlp` (download) + `ffmpeg`
  (cut) + `whisper` (captions). Reserve Ssemble credits for hero clips + auto-posting.
- **The `ycp` system** (this repo) — sourcing queue, Slack QC, metric capture, weekly brief.
- **Whop / Vyro / Content Rewards** — where the bounties are (§5).

---

## 5. Whop — how you actually get paid (don't skip a single rule)

These are the top reasons clippers DON'T get paid. Treat as law:
- **Submit inside the window** — many campaigns require submission **within 1 hour** of
  posting. Miss it = $0 even if the clip goes viral.
- **Set your verification code** — add the 6-char code to your social profile bio so Whop
  can track your views. No code = no approval.
- **Organic views only** — bought/bot views = permanent ban + forfeiture across all campaigns.
- **Hit the minimum view threshold** before a clip is eligible.
- **Check the pool before joining** — only join campaigns with **60%+ budget remaining** and
  **$2+/1K**. "Paid in order of approval" means if the pool empties before your approval, you
  get nothing. Speed matters.
- **Stay on-platform** — anyone DMing you a "campaign" on Telegram/Discord is a scam.

**Campaign vetting (green vs red):** ✅ $2+/1K · $5K+ pool · 60%+ remaining · visible payout
history · ≤14-day validation. 🚩 pool <20% · no payout history · <$1/1K · 30+ day holds ·
unverified creator · off-platform contact.

**Stack:** Whop/Content Rewards (variety) + Vyro ($3/1K floor, no follower minimum). Realistic
solo ramp: ~$50–200 (mo 1) → $300–1,000+ (mo 3) per operator *on Whop alone* — owned-channel
YPP + affiliates stack on top toward $10K.

---

## 6. Troubleshooting

| Symptom | Fix |
|---|---|
| `ycp source` returns empty | Check creator handles in `config/niches.yaml` are valid `@handles`/URLs. |
| A channel got flagged/str?iked | Stop posting to it; review §3. Never share AdSense across the network. |
| Views flat week over week | Open `data/latest-brief.md` → make more of the 🟢 combos, kill the 🔴. |
| Not getting paid on Whop | Re-check §5: window, verification code, min views, pool remaining. |
| Ssemble out of credits | Switch to the hybrid pipeline (§4) for volume; save credits for posting. |
| Slack QC not posting | Confirm `.env` has `SLACK_BOT_TOKEN`/`SLACK_APP_TOKEN`/`SLACK_QC_CHANNEL` and `ycp qc-listen` is running. |

---

## 7. First-week setup (one time)

Follow `LAUNCH-CHECKLIST.md` (in this same folder): Cloudflare
emails → warm 8–12 channels → connect to Ssemble → join 5–10 Whop campaigns → fill
`config/niches.yaml` → run `ycp init` → first clips live by end of week 2.
