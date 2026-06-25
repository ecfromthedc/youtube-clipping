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

**Team rule:** every fix, optimization, and learning ships to GitHub so all agents/teammates
inherit it. Update `STATUS.md` (current state) + this file (the why) on anything non-obvious.
