# Ralph loop — dial the WHOLE build to flawless

You are a fresh agent. The repo is your memory. Goal: make the YouTube clipping build provably
correct end-to-end. The work list is **QA-CHECKLIST.md** — read it first; it has the rules.

## Each iteration
1. Read `QA-CHECKLIST.md`. Pick the **first unchecked `- [ ]`** item.
2. **Exercise it on real inputs** per the item (run the actual command / render a real clip /
   Read a real frame / run the gate). Don't assume — produce evidence.
3. **If it passes:** change `- [ ]` → `- [x]` and append the evidence inline (the command + its
   key output / the ffprobe number / what you saw in the frame).
4. **If it fails:** fix the code minimally to make it correct, re-verify, THEN check it off with
   the evidence. Keep the fix small and targeted.
5. After ANY code change: `ruff check src tests` clean + `pytest -q` green (re-run
   `./scripts/setup.sh` after editing `src/ycp/` so the installed `ycp` reflects it).
6. Commit just what you touched by explicit path (`git add <paths>`; NEVER `git add -A`) →
   `git commit` → `git push origin main`. One item (or one fix) per commit.

## Hard safety rails — do not violate
- **NEVER post to the live channel.** Verify `distribute`/`autopilot` via the unit tests, a mock
  adapter, or a sandbox: `mkdir -p /tmp/ycp-qa/data && cp -r config /tmp/ycp-qa/ && ln -sf "$PWD/.env" /tmp/ycp-qa/.env && ln -sf "$PWD/models" /tmp/ycp-qa/models`, set `distribution.enabled: false` in the sandbox config, run with `YCP_HOME=/tmp/ycp-qa`. The real channel is never a test target.
- **`src/ycp/` is LIVE production.** Fix forward; never leave tests red or the pipeline broken.
- Read-only API calls (capture, milestones, channel stats) are fine against the real channel.
- If a check needs a real clip, bound the source short for speed. Reuse an already-downloaded
  source if one exists rather than re-fetching.
- The Rust items: work in `rust/` only (Python stays untouched). Build with `cargo build --release`,
  render via `rust/target/release/ycp clip <url> --max 1` (no posting), ffprobe + Read a frame.

## Reference for the 3 Rust render bugs (if you reach them)
- 38s clamp: Python does `min(end, start + MAX_CLIP_SEC=38)` in `src/ycp/clip.py`. Mirror in `rust/src/clip.rs`/`vision.rs`.
- score 0–1: Python normalizes (Gemini path); Rust emits 3–5 → A/B gate (0.9) over-fires. Normalize in `rust/src/vision.rs`/scoring.
- title render: word captions render fine; the TITLE path in `rust/src/captions.rs` mis-wraps/over-sizes. Match `src/ycp/captions.py` (whole-word wrap, `_fit_font` stepping, top position).

## DONE
When EVERY box in QA-CHECKLIST.md is `- [x]` with evidence, do one final holistic read, then
write `DIALED-DONE` containing a one-line summary (e.g. "all 22 checks ✅, pytest N green, Rust
frame matches Python"). The loop stops when that file exists. Do NOT write it early — a skeptic
reading the checklist must see real evidence on every line first.
