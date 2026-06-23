# Operator Playbook — run the clipping play, clear $15K/month

**Who this is for:** anyone on the team running clipping. No special skill required.
Follow it top to bottom. If a step makes you ask a question, that's a bug — flag it
and it gets fixed in the next loop cycle.

**The one-sentence model:** clip viral moments from big creators → post across many **owned**
channels → monetize the owned stack (TikTok Creator Rewards + YouTube Partner Program +
affiliate + brand deals) as the channels grow. **Your owned channels are the appreciating,
automatable, sellable asset.** _(Whop cut 2026-06; pure owned-first.)_

> Maintained by the optimization loop (`GOAL-AND-LOOP.md`). Gets sharper every cycle.

---

## 0. Your scorecard — the ramp to $15K (know your numbers)

Income = **views × rate**. The table below is your first-90-days ramp to ~$10K/mo (the **"Cruise"**
milestone on the scoreboard); **$15K is the goal** as the owned stack matures. On the owned
monetization stack (TikTok Creator Rewards + YPP + affiliate + brand deals), it's reachable at a
fraction of the 300M+ views/month that pure YouTube Shorts ad revenue alone would need.

| Lever | Month 1 target | Month 2 | Month 3 (≈$10K) |
|---|---|---|---|
| Channels live (warmed) | 8–12 | 12–15 | 12–15 |
| Posts/day (across YT+TikTok+Reels) | 15–30 | 45–75 | 75–100 |
| Monthly views (all channels) | 1–2M | 3–6M | **6–10M** |
| Realized $/1K (owned stack, blended) | ~$1.00 | ~$1.25 | ~$1.50 |
| **Est. monthly income** | **$1–2K** | **$3–5K** | **$8–12K** |

**Check yourself weekly:** open `data/latest-brief.md`. If your views aren't climbing,
the bottleneck is one of: volume (post more), hit-rate (make the 🟢 combos), or
monetization (more funded campaigns, submitted in-window). Fix that one thing.

---

## 1. Your daily routine (≈90 min, then the system runs)

1. **Pull the queue** — `cd ~/Documents/Development/youtube-clipping && ./scripts/daily.sh`
   (or `uv run ycp source`). Open `data/source-queue.md` — these are today's highest-
   velocity videos to clip, ranked. Work top-down.
2. **Clip** — run each source video through Ssemble AI clipping (or the hybrid pipeline,
   §4). Target 5–10 clips per source video.
3. **Transform owned-channel clips** (§3 guardrails) — add your hook, cut, and a take.
   Every owned-channel clip MUST be transformed.
4. **Approve** — clips post to the Slack `#clip-qc` channel automatically. React ✅ to
   approve, ❌ to reject. Run the §2 checklist on each.
5. **Distribute** — approved clips schedule through Ssemble to all channels at best times.
   Hands-off after approval.
6. Done. Capture + scoring run on cron; the weekly brief tells you what to double down on.

---

## 2. The QC approval checklist (seconds per clip)

Before you react ✅ in Slack, every clip must pass:
- [ ] **Hook lands in the first 1–2 seconds** (or it's dead on Shorts).
- [ ] **Transformation present** (owned-channel clips only — your cut/hook/take).
- [ ] **No copyrighted music bed** — strip/replace it. Music = instant Content-ID claim.
- [ ] **Captions accurate and readable.**
- [ ] **Right channel + niche fit.**
- [ ] **Platform-ready**: correct platform, length, required tags, 9:16.

If it fails any box → ❌ reject (or send back for a fix). One bad owned-channel clip can
demonetize the whole channel.

---

## 3. Guardrails — these protect your income (never skip)

| Rule | Why |
|---|---|
| **Transform every owned-channel clip** (your hook/cut/commentary) | Raw reuploads get the channel demonetized *channel-wide* under YouTube's inauthentic-content policy. |
| **Clip-friendly / permissioned sources first** | Lowest copyright-strike risk; many creators publicly encourage clipping. |
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

---

## 5. Owned monetization — how you actually get paid

_(Whop cut 2026-06; pure owned-first.)_ Revenue comes from the **owned monetization stack** —
you own every channel, so the income compounds instead of evaporating when a campaign pool dries up.
- **TikTok Creator Rewards** — the fastest owned cash: ~$0.40–$1.00/1K on 1-min+ videos
  (~10–20× YouTube Shorts ad rev). Turns on well before YPP. Verify current rates.
- **YouTube Partner Program (YPP)** — eligibility takes months (1,000 subs + 10M Shorts
  views/90d, or 4,000 watch hours). Don't wait on it; it matures in the background.
- **Affiliate** — earns on even a small, engaged channel. Wire relevant offers per niche.
- **Brand deals** — Rising Tides' agency edge; stack these on top once a channel has an audience.

**Channel health is the meta-rule:** one channel-wide demonetization costs months. Transform every
clip, keep music clean, warm accounts on residential IPs — protect the asset above raw output.

**Realistic ramp:** ~$50–200 (mo 1, TikTok Rewards + early affiliate) → $300–1,000+ (mo 3) per
operator, with YPP + brand deals stacking on top toward $15K as the channels mature ($10K is the "Cruise" milestone en route).

---

## 6. Troubleshooting

| Symptom | Fix |
|---|---|
| `ycp source` returns empty | Check creator handles in `config/niches.yaml` are valid `@handles`/URLs. |
| A channel got flagged/str?iked | Stop posting to it; review §3. Never share AdSense across the network. |
| Views flat week over week | Open `data/latest-brief.md` → make more of the 🟢 combos, kill the 🔴. |
| Owned revenue not landing | Re-check §5: TikTok Creator Rewards eligibility, affiliate links live, channel monetization status. |
| Ssemble out of credits | Switch to the hybrid pipeline (§4) for volume; save credits for posting. |
| Slack QC not posting | Confirm `.env` has `SLACK_BOT_TOKEN`/`SLACK_APP_TOKEN`/`SLACK_QC_CHANNEL` and `ycp qc-listen` is running. |

---

## 7. First-week setup (one time)

Follow `LAUNCH-CHECKLIST.md` (in this same folder): Cloudflare
emails → warm 8–12 owned channels → connect to Ssemble → confirm owned monetization
(TikTok Creator Rewards / affiliate) → fill `config/niches.yaml` → run `ycp init` → first
clips live by end of week 2.
