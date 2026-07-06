# ycp — Rust port (parity reached ✅)

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
./target/release/ycp serve         # Tides Tiller editor → http://localhost:8787
```

## Tides Tiller editor (`ycp serve`)

A browser UI over the clip pipeline — internal team tool, no auth/billing. Drop in
raw footage → the pipeline transcribes it + ranks your best moments → pick one on
the timeline → render a captioned 9:16 MP4. Runs entirely against the existing
Rust modules; nothing in the pipeline is duplicated.

**One-time setup on a team machine:**
```
brew install ffmpeg yt-dlp whisper-cpp   # or: ./scripts/setup-whisper.sh
cd rust && cargo build --release
./target/release/ycp serve --port 8787
```

**What each step actually runs:**
| Step | Module | What happens |
|------|--------|--------------|
| Upload | `server.rs` | multipart → `data/editor/<id>/source.mp4` + ffprobe duration |
| Transcribe | `transcribe.rs` | whisper.cpp (or openai-whisper) → word-level SRT |
| Plan clips | `clip::plan_clips` | groups segments into 15–38s windows, scores by hook strength |
| Render | `clip::cut_vertical` + `captions::burn_captions` | ffmpeg trim → 9:16 reframe → ab_glyph opus-style captions |
| Persist | `warm_cache` | on restart, rebuilds project state from disk |

**API surface** (everything else is static frontend):
- `GET  /api/health`
- `GET  /api/projects` — list
- `POST /api/projects` `{filename?}` → `{id}`
- `GET  /api/projects/:id` — full project (transcript + candidates + renders)
- `DELETE /api/projects/:id`
- `POST /api/projects/:id/upload` (multipart `file`) → `{id, duration}`
- `POST /api/projects/:id/transcribe` `{min_len?, max_len?, top?}` → updated project
- `POST /api/projects/:id/render` `{start, end, title?}` → `{path, duration}`
- `GET  /api/projects/:id/files/<path>` — stream source.mp4 or renders

**Project state** lives in `data/editor/<id>/`: `source.mp4`, `transcript.json`,
`duration.txt`, `filename.txt`, `renders/*.mp4`. Delete a project by deleting its
folder, or via the dashboard trash button.

## Port status (parity-checked against Python, module by module)

| Module | Python | Rust | State |
|--------|--------|------|-------|
| config (root, settings, .env) | config.py | config.rs | ✅ done |
| db (schema, models, queries) | db.py | db.rs | ✅ spine done |
| CLI skeleton | cli.py | main.rs | ✅ `init`, `status`, `scoreboard` |
| scoring (scores, rollups, scale/kill) | scoring.py | scoring.rs | ✅ done — parity-exact (rollup/score rounding switched to `util::round_to` = pandas round-half-to-even during the autopilot row; bare `.round()` diverged on .5 ties once metrics had variance) |
| scoreboard / brief (markdown) | scoreboard.py, brief.py | scoreboard.rs, brief.rs | ✅ done — parity-exact (byte-identical vs Python on clips.db + demo.db; tabulate "pipe" tables, decimal alignment, `%g`, comma-money all reproduced) |
| optimize (learning weights, creative prefs) | optimize.py | optimize.rs | ✅ done — parity-exact (added `optimize::run` orchestrator + re-synced `format_entry`/log-header to current `optimize.py` during the autopilot row; both verified byte-identical via the autopilot IMPROVEMENT-LOG.md) |
| experiment (A/B winners) | experiment.py | experiment.rs | ✅ done — parity-exact (byte-identical winners vs experiment.py on a synthetic A/B db: sorted groups, runner-up ties, min-views + single-variant skips) |
| guardrails / srt / captions-chunking | guardrails.py, srt.py, captions.py | guardrails.rs, srt.rs, captions.rs | ✅ done — parity-exact (srt slice/shift/format + caption chunks byte-identical vs Python across 4 windows on a sample SRT; guardrails gates match Python on full battery. `ycp captions <srt> <start> <end>` is the cross-check harness. Note: only chunking ported here — Pillow render is the separate "captions render" row. Python `round()` reproduced via `{:.N}` format-parse, verified byte-identical on a timing battery.) |
| hooks (score + select; DeepSeek via reqwest) | hooks.py | hooks.rs, enhance.rs (`pick_title`) | ✅ done — parity-exact (`score_hook` + `looks_safe` byte-identical vs hooks.py across a 15-case battery via `ycp hook <moment> [angle]`: curiosity/stakes/personal/digit signals, length taper, finance+debate bonuses, slur safety, blank/short edges. Selection/coerce/variants logic locked by the ported `test_hooks.py` assertions. Generation hits DeepSeek live — `best`/`variants` aren't byte-diffable, so the deterministic core is the cross-check; `best` falls back to the heuristic when no key.) |
| capture / distribute / archive (APIs) | capture.py, distribute.py, archive.py | capture.rs, distribute.rs, archive.rs | ✅ done — parity-exact (pure cores byte-identical vs Python: `ycp slots`/`ycp retention` harnesses match `assign_slots`/`analyze_retention` across a battery — DST spring-forward gap (PEP-495 fold=0), fall-back ambiguity, non-US tz, prod config, retention ties/clamps. qc_decision/caption_for/hashtags_for/title_for/is_rclone/video_id/post_id/prune_local/auto_qc locked by the ported test batteries. Live-API paths — Postiz upload+post, YT-Analytics OAuth, yt-dlp/rclone shell-outs — mirror Python structurally and no-op without creds; not byte-diffable, same as hooks. db.rs gained insert_metric/record_qc/pending_qc_clips + set_clip_status extra-fields; approved_clips fixed to status='approved' for Python parity.) |
| sourcing / transcribe / clip / reframe (native) | sourcing.py, transcribe.py, clip.py, reframe.py | sourcing.rs, transcribe.rs, clip.rs, reframe.rs | ✅ done — parity-exact (pure cores byte-identical vs Python on shared inputs: `ycp src-rank` matches `parse_entries`+`rank` (velocity round, isoformat published_at, min_views filter, tie order, NA/tab-title parse); `ycp plan` matches `plan_clips` (windowing + heuristic score); `ycp score-cand` matches `score_candidate`; `ycp crop-x` matches `reframe.crop_x_expr` across center/clamp/sustained-pan. `render_queue_md`, `load_creators`, `whisper_cpp_cmd`/`model_path`/`find_cpp_binary`, `window_text`, `smooth` ported + unit-locked. Native shell-outs (yt-dlp enumerate/meta/download, ffmpeg trim/reframe, whisper.cpp/openai) + `sourcing::run` mirror Python structurally, no-op without tools, not byte-diffable. db.rs gained `upsert_source_video`. **Deferred to autopilot row:** `clip::run` orchestrator (needs unported captions-render + vision.py + enhance.stack_gameplay). **Parity gap:** `reframe::face_track` returns empty (no pure-Rust OpenCV) → center-crop fallback, byte-identical to Python's cv2-absent path; real face-pan needs an OpenCV binding (later call).) |
| captions render (Pillow → image+cosmic-text) | captions.py | captions.rs | ✅ done — parity-exact on the deterministic layer (pure Rust: `ab_glyph` rasterizes the TTF glyphs, `image` writes the RGBA PNG sequence ffmpeg `overlay` composites — no Python/Pillow on target machines, the single-binary goal holds). `ycp caprender <srt> <dur> [title]` is the cross-check harness: its frame **schedule** (cfg knobs, frame count = `ceil(dur·fps)`, and which chunk/title shows per frame) is byte-identical vs `captions.py` across chunk boundaries + the 7s title-hold flip + the no-title path. The **pixels** are visually-equivalent, NOT byte-identical (ab_glyph vs Pillow are different rasterizers/strokers) — verified structurally instead: 1080×1920 frames, all frames inked, and the active(yellow)/idle(white)/fat-outline(black) color logic confirmed present per frame. `render_overlay`/`burn_captions`/`probe_duration` mirror Python structurally; the ffmpeg overlay cmd is identical. Chose `ab_glyph` over `cosmic-text` (no shaping/bidi needed — manual L→R glyph placement, same as Pillow). **Ceiling:** outline = stroke-radius disk stamp per glyph pixel (O(ink·stroke²)); fine for a background pipeline, upgrade to separable max-dilation if a frame renders slow. No embedded fallback font (missing TTF → blank caption frames → `clip.py` ships a plain clip, as it already does on caption failure). |
| autopilot (orchestrator) | autopilot.py | autopilot.rs | ✅ done — parity-exact. `ycp autopilot --skip-source --no-clip` is **byte-identical** to Python across all 9 stages (source→clip→qc→capture→brief→scoreboard→optimize→distribute→cleanup), and SCOREBOARD.md + IMPROVEMENT-LOG.md + latest-brief.md write byte-identical (verified on isolated temp homes, distribution disabled to avoid live Postiz, fresh DB per run). Pure cores unit-locked: `select_unclipped`, `angle_for`, `channel_for`, `connected_channels`. This row also **wired the deferred `clip::run`** (sha1[:8] clip-ids, vision moment-pick → cut → A/B hook sets → caption burn → gameplay stack → archive → pending_qc) + new modules `vision.rs`/`qc.rs`/`diagnose.rs`/`enhance::stack_gameplay` + db helpers (`insert_clip`/`save_brief`/`source_queue`/`clipped_source_ids`) + `optimize::run`. Native/live paths (yt-dlp/ffmpeg/whisper, DeepSeek, Gemini Files-API, Postiz/YT-Analytics) mirror Python structurally, no-op without creds, not byte-diffable — same doctrine as prior rows. **Two upstream parity gaps the integration surfaced + fixed:** (1) `optimize::format_entry`/log-header had drifted from current `optimize.py` (new 3-line header, "Top formats/lengths" line, comma-formatted views, reworded "Why") → re-synced; (2) `scoring::rollup`/`compute_scores` used Rust's bare `.round()` (half-away) vs pandas `.round(n)` (half-to-even) → switched to `util::round_to`, fixing an avg_views/score tie (0.5 → 0 not 1) that only appears once metrics have variance. **Ceilings:** `vision::rank_moments` returns [] (live Gemini upload not ported — heuristic fallback, like `reframe::face_track`); qc `slack`/`telegram` human channels delegate to the unported Python listeners (shipped config is `qc.auto: true` → the auto-guardrails path, which IS exact). |

## Order (dependency-first, compiling at every step)
1. **Foundation** — config, db, CLI (✅).
2. **Deterministic core** — scoring → scoreboard/brief → optimize/experiment. Port + cross-check
   numbers against Python on the same DB. (Pure logic; no native deps.)
3. **API clients** — reqwest+serde for DeepSeek, Postiz, YouTube Analytics, Gemini.
4. **Native pipeline** — sourcing (yt-dlp), transcribe (whisper.cpp), clip/reframe (ffmpeg/opencv),
   captions render (image crate). Shell out where the tool is already native.
5. **Autopilot** — chain the stages; flip the crons from the Python `ycp` to the Rust `ycp`.

Cut over only when `cargo build --release` is green AND each stage matches Python output.
**Status: both hold.** Every row is ✅, `cargo build --release` is green, `cargo test` is 80/80,
and `ycp autopilot --skip-source --no-clip` is byte-identical to Python (stdout + SCOREBOARD.md +
IMPROVEMENT-LOG.md + latest-brief.md). Remaining non-byte-diffable surfaces are the documented
live/native ceilings (yt-dlp/ffmpeg/whisper shell-outs, DeepSeek/Gemini/Postiz APIs, OpenCV
face-track) — they mirror Python structurally and no-op without creds. Next step is the live
cutover: run the full clip path (`--no-clip` off) on a real machine with creds + ffmpeg, then
flip the crons from the Python `ycp` to the Rust `ycp`.
