# Monetization — readiness & path to $15K/month (AdSense)

North star: **AdSense revenue → $15,000/month**, owned. The milestone watcher (`ycp milestones`,
daily 9 AM cron) tracks progress and Slack-alerts `#youtube-clipping` on every threshold crossed —
so nobody has to watch the dashboard. This doc is what we get ready for.

## The gates (2026 YouTube Partner Program)
- **ENTRY tier:** 500 subs + (**3M Shorts views/90d** OR 3k watch hrs/12mo) + ≥3 public posts/90d.
- **FULL tier:** 1,000 subs + (**10M Shorts views/90d** OR 4k watch hrs/12mo).
- Prereqs: live in a YPP country · **no active Community-Guidelines strikes** · **2-step verification ON** · follow monetization policies.
- Shorts revenue split: creator keeps **45%** of allocated Shorts ad revenue.

## ⚠️ The clip-channel eligibility rule (most important thing here)
**Non-original / unedited reuploads / compilations with no original content = INELIGIBLE Shorts
views.** To earn, every clip MUST be **transformed** — our word-by-word captions, hook overlay,
vertical reframe, the cut itself. Our pipeline already enforces this: `guardrails.publish_allowed`
requires `fmt=auto-clip` (transformed, not a raw reupload), and we only clip **credentialed experts**
(advertiser-safety for YMYL health). This is the moat — a lazy raw-reupload clip channel accrues
*zero eligible views* and never monetizes. We're built to pass.

## Steps to turn monetization ON (once entry-eligible — the watcher will alert with these)
1. **YouTube Studio → Earn → Apply** (the option appears once eligible).
2. **Turn ON 2-step verification** on the channel's Google account (required *before* applying).
3. **Link AdSense** — create the AdSense-for-YouTube account **inside Studio's Earn flow**, not separately.
4. **Accept the YPP Terms AND the Shorts Monetization Module** (Shorts ad-rev only counts from the date you accept — accept promptly).
5. **Submit** → automated + human review (~1 month).
6. **Add tax info** in AdSense (required to get paid).
7. Maintain **zero Community-Guidelines strikes** throughout review.

## Prep NOW (before 500 subs, so there's no scramble at the threshold)
- 2-step verification on the Phoenix Protocol Google account.
- AdSense ready (create via Studio the moment the Earn tab unlocks).
- Transformed-only clips (enforced) · credentialed sources only · zero strikes.

## The watcher
`ycp milestones` pulls subs / trailing-90-day views / trailing-30-day revenue run-rate via the
YouTube Data + Analytics OAuth, tracks fired thresholds idempotently (`data/milestones.json`), and
Slack-alerts on each NEW crossing: subs (100 → 500 → 1k → …), views/90d (1M → 3M → 10M → …),
revenue ($1 → $100 → $1k → $5k → $10k → **$15k/mo**). When entry-eligible (500 subs + 3M views/90d)
the alert includes the apply-now steps above.

## Sources
support.google.com/youtube (YPP overview + Shorts monetization policies), youtube.com/creators/partner-program,
vidiq.com, unkoa.com, milx.app, studiobinder.com, nexlev.io, tubebuddy.com
