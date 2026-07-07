# Learnings — mistakes & fixes (so the team doesn't repeat them)

Append-only. Each entry: what broke → why → the fix that's now in the repo.

## 2026-06-25 — first live autonomous cycle

**Over-posting (38 clips/run ≈ 2-week backlog).** The factory posted *every* approved clip and
A/B'd *every* hero moment → one morning flooded ~2 weeks of schedule with redundant variants.
→ **Fix:** quality-selection (`distribution.max_per_run`, cold-start `1`) posts only the
top-scoring clip per cycle and marks the rest `skipped`; A/B only the single best moment per
source. Overproduce → post the best → drop the rest. Unit tests cover the cap.

**Rust cutover = NO-GO.** The ported binary runs the whole pipeline, but the hook-title render
is broken (giant overlapping text) and durations/scores are off (60s clips, 3–5 scores). The
deterministic core is parity-faithful; the native render layer isn't. → Python stays production;
Rust fixes queued. *Lesson: unit-test parity ≠ rendered-output parity — eyeball a real frame.*

**OAuth re-auth silently grabbed the wrong client.** With several `client_secret*.json` in
~/Downloads, `yt_oauth.py` auto-picked the *newest* → consented for a different OAuth client →
re-auth "completed" but the token never gained the new scope. → **Fix:** pin the client_secret
whose `client_id` matches the token already in `.env`.

**Almost deleted another channel's videos.** Postiz's "published" `releaseId` list spans every
channel on the account, not just ours. A cleanup pass nearly issued deletes against a different
channel's videos (YouTube 403'd, but still). → **Fix:** `youtube_ops.delete_video` verifies a
video is on *our* channel before deleting. *Lesson: never trust an external id list for a
destructive op — confirm ownership against our own API first.*

**Deleted a teammate's scheduled posts on a shared Postiz account.** During a QA loop I saw 16
new QUEUE posts, *assumed* our loop had leaked them, and bulk-deleted all QUEUE — before checking
content. They were "Zohran Mamdani" clips scheduled on a *different* integration (Carry the
Fire / Marc Robinson), not Phoenix Protocol. Nothing was *published*, but 16 of a teammate's
scheduled posts were removed (no Postiz trash/undo). I violated the rule the entry right above
this one states. → **Fix / hard rule: every Postiz operation — post AND cleanup — filters to OUR
integration id only (`cmqsakb8z…` = Phoenix Protocol). NEVER act on QUEUE/posts by state alone on
a shared account; match the integration first.** Our `distribute` already scopes posting to mapped
channels; the gap was a manual cleanup script. Any cleanup must take an integration id and refuse
to touch others. The team's other accounts run independently — we never touch them.

## 2026-06-26 — AI Frontier channel pivot + clip-sourcing rules

**Source selection makes or breaks `ycp clip` when Gemini is OFF.** Cutting 5+ AI-news samples
(Channel 6 "AI Frontier") with **no `GEMINI_API_KEY`** in `.env`, so the moment-picker falls back
to the transcript heuristic. The heuristic + face-tracker reliably nail **single-speaker** sources
(solo talking-head, keynote/stage talk) — any window lands on the right person saying on-topic
words (Karpathy YC, Theo solo, LeCun Lex clip, Tenev TED all came out clean). They **fail** on:
(1) **two-person interviews** — grabs the *interviewer's setup question* and the face-pan locks
onto the host, not the guest (Altman/Lex, Primeagen/Lex both missed); (2) **screen-share / B-roll**
windows — clips a chart or cutaway, no speaker (Primeagen survey graphic); (3) **DOAC-style
produced cold-open teaser montages** — fast B-roll + host narration in the first ~3 min, so a
from-zero `--window` grabs the montage, not the interview (Hinton/DOAC missed). → **Rules until
Gemini is wired:** prefer single-speaker sources; for interviews, either add `GEMINI_API_KEY` (the
real fix — vision picks the guest's actual payoff window) or pass an explicit known timestamp; never
`--window N` a podcast whose intro is a montage. Also: a hand-written `--title` rides on top of
*whatever* window the heuristic picked, so verify hook-matches-payoff on a rendered frame.

**Two caption pitfalls (RULE #1).** (a) Re-uploads/clip-channels that **burn their own captions**
(the Axios Dario clip, DOAC re-cuts) → stacking ours = double track; pass `--no-captions` (set
`has_captions: true` in `niches.yaml`) to defer — the hook still renders. (b) Already-**vertical /
picture-in-picture** clip-channel shorts (a Lex *Clips* PiP cam) reframe terribly — the face-pan
locks onto the tiny inset and the rest is empty wall. Source the original 16:9 long-form instead.
*Lesson (again): eyeball a real frame — wrong speaker, double captions, and B-roll all pass the
"clip produced ✓" check silently.*

**Gemini picks WHEN, not WHO — 2-cam interviews still framed on the host.** Wired
`GEMINI_API_KEY` and added `--start MIN` (download a slice from an offset, not just the first N —
so we can reach the gold deep in a 90–140 min interview past the cold-open montage; `_section()`
is unit-tested). With both, Gemini reliably picks a good *moment* and it improved selection (the
Hinton/DOAC clip now opens on Hinton, not the host). BUT on **two-camera podcasts (Dwarkesh)** the
editor cuts between host and guest, and our center/face reframe crops to whoever dominates the
window — frequently the interviewer asking the setup question (Ilya/Dario/Dwarkesh clips came out
framed on *Dwarkesh*). Gemini chooses the timestamp; it does not control which face gets cropped.
→ **Until a speaker-aware reframe exists** (crop to the *named guest* specifically — the real next
feature for interview sources), clip interviews only from **guest-dominant** uploads / official
clip channels that stay on the guest, or accept host-framed misses. Single-speaker sources (solo
creators, keynote/stage talks, guest-dominant clips) remain the reliable lane — the 5 shipped AI
Frontier samples are all that shape.

**Team rule:** every fix, optimization, and learning ships to GitHub so all agents/teammates
inherit it. Update `STATUS.md` (current state) + this file (the why) on anything non-obvious.
