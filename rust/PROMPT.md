# Ralph prompt — port ycp to Rust (one module per run)

You are one iteration of a stateless loop. The repo is the memory. Do ONE unit of work, verify
it, commit it, and exit. A fresh you runs next.

## Goal
Port the Python `src/ycp/*.py` to a single Rust binary in `rust/` at behavior parity, until
every row in `rust/README.md` is ✅ and `cd rust && cargo build --release` is green and
`ycp autopilot` runs the full pipeline at parity. Python in `src/ycp/` stays LIVE — never touch it.

## This iteration — do exactly this
1. **Pick the next unit.** Open `rust/README.md`, take the FIRST row still marked ⏳ (top-down =
   dependency order). That Python module is your target.
2. **Port it** to `rust/src/<module>.rs`, mirroring the Python. Reuse existing Rust types
   (`db::ClipRow`, `scoring::Analysis`, etc.) — read the already-ported modules first to match
   patterns. Add deps to `rust/Cargo.toml` as needed. Native tools (ffmpeg/whisper/yt-dlp) are
   shelled out via `std::process::Command`, same as the Python does.
3. **Compile.** `cd rust && cargo build` until it is GREEN. Fix every error before continuing.
4. **Cross-check** against Python on the SAME `data/clips.db` where feasible: add a CLI subcommand
   that exercises the module and compare its output to the Python equivalent
   (`.venv/bin/python -c "..."`). Numbers/structure must match.
5. **Mark it ✅** in `rust/README.md` (and note "parity-exact" if cross-checked).
6. **Commit + push.** Stage ONLY the rust/ files you touched by explicit path (NEVER `git add -A`).
   Message: `feat(rust): port <module> — parity-exact with <module>.py`. `git push origin main`.

## Hard rules
- One module per run. Don't start a second once the first is committed — exit.
- If `cargo build` won't go green after honest effort, STILL commit the WIP with a `wip(rust):`
  message + a clear note in `rust/README.md` of what's blocking, so the next iteration resumes.
- The caption renderer (Pillow → Rust `image`/`cosmic-text`) is the hardest. If you reach it and
  it's not converging, write the blocker to `rust/README.md` and STOP rather than thrash — it may
  need a human call (pure-Rust render vs. shell out to a tiny Python renderer).
- Never edit `src/ycp/` (the live Python). Never use `--dangerously-skip-permissions`.

## Done check (if true, do nothing and exit)
`rust/README.md` has zero ⏳ rows AND `cd rust && cargo build --release` exits 0.
