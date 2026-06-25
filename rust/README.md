# ycp — Rust port (in progress)

A single static binary replacing the Python `src/ycp` for lightweight, robust distribution
to the team. **The Python version stays the live production system until this reaches parity**
— both read the same `data/clips.db` and `config/settings.yaml`, so they interoperate during
the port and can be cross-checked module-for-module.

## Why Rust here
It's an orchestration + data tool (shell out to ffmpeg/whisper/yt-dlp, call REST APIs,
analyze a dataframe, drive a CLI) — not Python ML. One binary `scp`'d to N machines beats a
Python env per machine; compile-time safety kills cross-machine dependency drift.

## Build & run
```
cd rust && cargo build --release
./target/release/ycp status        # cross-checks against the Python output
```

## Port status (parity-checked against Python, module by module)

| Module | Python | Rust | State |
|--------|--------|------|-------|
| config (root, settings, .env) | config.py | config.rs | ✅ done |
| db (schema, models, queries) | db.py | db.rs | ✅ spine done |
| CLI skeleton | cli.py | main.rs | ✅ `init`, `status`, `scoreboard` |
| scoring (scores, rollups, scale/kill) | scoring.py | scoring.rs | ✅ done — parity-exact |
| scoreboard / brief (markdown) | scoreboard.py, brief.py | scoreboard.rs, brief.rs | ✅ done — parity-exact (byte-identical vs Python on clips.db + demo.db; tabulate "pipe" tables, decimal alignment, `%g`, comma-money all reproduced) |
| optimize (learning weights, creative prefs) | optimize.py | optimize.rs | ✅ done — parity-exact |
| experiment (A/B winners) | experiment.py | experiment.rs | ✅ done — parity-exact (byte-identical winners vs experiment.py on a synthetic A/B db: sorted groups, runner-up ties, min-views + single-variant skips) |
| guardrails / srt / captions-chunking | guardrails.py, srt.py, captions.py | guardrails.rs, srt.rs, captions.rs | ✅ done — parity-exact (srt slice/shift/format + caption chunks byte-identical vs Python across 4 windows on a sample SRT; guardrails gates match Python on full battery. `ycp captions <srt> <start> <end>` is the cross-check harness. Note: only chunking ported here — Pillow render is the separate "captions render" row. Python `round()` reproduced via `{:.N}` format-parse, verified byte-identical on a timing battery.) |
| hooks (score + select; DeepSeek via reqwest) | hooks.py | hooks.rs, enhance.rs (`pick_title`) | ✅ done — parity-exact (`score_hook` + `looks_safe` byte-identical vs hooks.py across a 15-case battery via `ycp hook <moment> [angle]`: curiosity/stakes/personal/digit signals, length taper, finance+debate bonuses, slur safety, blank/short edges. Selection/coerce/variants logic locked by the ported `test_hooks.py` assertions. Generation hits DeepSeek live — `best`/`variants` aren't byte-diffable, so the deterministic core is the cross-check; `best` falls back to the heuristic when no key.) |
| capture / distribute / archive (APIs) | capture.py, distribute.py, archive.py | — | ⏳ |
| sourcing / transcribe / clip / reframe (native) | *.py | — | ⏳ (shell out to yt-dlp/whisper/ffmpeg) |
| captions render (Pillow → image+cosmic-text) | captions.py | — | ⏳ (hardest port) |
| autopilot (orchestrator) | autopilot.py | — | ⏳ (last — wires it all) |

## Order (dependency-first, compiling at every step)
1. **Foundation** — config, db, CLI (✅).
2. **Deterministic core** — scoring → scoreboard/brief → optimize/experiment. Port + cross-check
   numbers against Python on the same DB. (Pure logic; no native deps.)
3. **API clients** — reqwest+serde for DeepSeek, Postiz, YouTube Analytics, Gemini.
4. **Native pipeline** — sourcing (yt-dlp), transcribe (whisper.cpp), clip/reframe (ffmpeg/opencv),
   captions render (image crate). Shell out where the tool is already native.
5. **Autopilot** — chain the stages; flip the crons from the Python `ycp` to the Rust `ycp`.

Cut over only when `cargo build --release` is green AND each stage matches Python output.
