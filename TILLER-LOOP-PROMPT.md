# Goal + Loop Prompt: Tides Tiller — Rust UI port + Page Agent long-haul hardening

**What exists:** the Tiller is a vanilla-JS SPA (`rust/web/app.js` 1,249 lines + `styles.css`,
hash router) embedded via rust-embed into the `ycp` binary, served by axum (`rust/src/server.rs`)
on `:8788` → cloudflared tunnel → **tidestiller.risingtidesviral.com**. The Page Agent copilot is
Alibaba's `page-agent@1.11.0` loaded from **esm.sh at runtime** (a CDN outage kills it), talking
to the server-side DeepSeek proxy (`/api/llm/proxy/chat/completions`, key never in client JS).

**Precedent (the proven setup):** the Campaign Hub React→Leptos port —
`~/Documents/Development/campaign hub/LOOP_PROMPT.md` + `progress.md` (2026-07-03 entry).
21 screens converted by waves of **page agents** (one agent per page, pages never touch shared
files) + a **gate agent per phase** (clippy wasm32 `-D warnings`, fmt, host tests, trunk build),
class-for-class markup port so the CSS is the parity contract. Same setup here.

## Goal

The entire Tiller UI is a **Leptos WASM crate** (`rust/ui/`), pixel-identical to today's design
(Poppins, magenta→purple brand, claymorphic bead — the design is LOCKED, port ≠ redesign),
embedded in the same single `ycp` binary, gate-green, live through the tunnel. The Page Agent
copilot survives the long haul: **vendored** (no runtime CDN), driven by an **action map that
cannot drift** from the real UI, and failing loud instead of silent. Done = a teammate at
tidestiller.risingtidesviral.com sees zero difference, `rust/web/app.js` is deleted, and every
gate below is green.

## Method (per page — the Campaign Hub recipe, adapted)

1. `rust/ui/` is a **separate crate, NOT a workspace member** (mirrors campaign hub — keeps the
   root gate fast). Trunk builds it; `server.rs` embeds `ui/dist` via rust-embed (build order:
   `trunk build --release` then `cargo build` — script it in `rust/scripts/build-ui.sh`).
2. **`styles.css` is the contract.** Port markup class-for-class from `app.js`; the stylesheet
   itself is copied verbatim and never edited during the port. If a page needs a class that
   doesn't exist, hand-roll locally and report the gap — never edit shared files
   (router/shell/api module/styles) from a page agent.
3. One agent per page; signals + `spawn_local` for fetches against the existing `/api/*` routes
   (no server route changes — the API is already the shape the UI needs).
4. **Gate per phase** (the phase is not done until all pass):
   - `cd rust/ui && cargo clippy --target wasm32-unknown-unknown -- -D warnings && trunk build`
   - `cd rust && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
     (80 tests green today — that number only goes up)
   - chrome-devtools screenshot of each converted page at 1440px vs the live `rust/web` version —
     visually identical or the diff is listed and justified.
5. Resume-safe: run phases via Workflow so a session limit resumes with cached completed phases
   (campaign hub did exactly this).

## Backlog (priority order)

- **P0 — Foundation:** `rust/ui/` scaffold (Leptos + Trunk), topbar/shell/router with the four
  routes (`/` Projects · `/studio` · `/analytics` · `/pipeline`), `styles.css` + `logo.svg`
  copied in, served at `/next` from `server.rs` so old and new run side-by-side until parity.
- **P1 — Simple pages wave:** dashboard (Projects list + delete), new-project (upload +
  transcribe), pipeline.
- **P2 — Studio + Analytics wave:** studio index, studio format pages (ranking / storytelling /
  commentary forms → `/api/studio/render`), analytics (rollup / top / daily / retention /
  recommendations).
- **P3 — The editor:** project page (`/p/:id`) — moments sidebar, ranking-compilation section,
  render form, renders list with Postiz publish (⚠ shared Postiz account: only ever the mapped
  integration id). Biggest page; split into sub-agents if needed, same shared-file rule.
- **P4 — Page Agent long haul (the enhancement):**
  1. **Vendor** `page-agent@1.11.0`: download the esm.sh bundle once, commit under
     `rust/ui/vendor/`, serve via rust-embed. Pin the version; kill the runtime CDN dependency.
  2. **Anti-drift action map:** move the copilot's capabilities out of the prose
     `PAGE_AGENT_SYSTEM` string into a single `actions.rs` table (route + action + selector).
     The system prompt is *generated* from it, and a host test asserts every selector in the
     table exists in the built `ui/dist` HTML/components — UI change that breaks the copilot now
     breaks the gate, not the teammate.
  3. **Fail loud:** proxy gets a 30s timeout; missing `DEEPSEEK_API_KEY` → clear 503 surfaced as
     a toast in the panel, never a silent console warn. Replace the `window.prompt()` trigger
     with a real Leptos panel component (keep Ctrl+/ and the clay bead trigger).
- **P5 — Closeout:** flip `/` to the new UI, delete `rust/web/` (git keeps history), update
  STATUS.md + HANDOFF.md + this file's status line, restart `ycp serve`, verify live through
  the tunnel end-to-end (load each page, run one copilot task, publish nothing).

## Out of scope

The Python engine (`src/ycp` stays live — the Rust port cutover is a separate decision), any
pipeline/clip logic, mobile form factor (desktop-first internal tool), new features, new server
routes, redesign of any kind.

## Guardrails (LOCKED)

- ⛔ Shared Postiz account — every Postiz call filters to the mapped integration id; never act
  by state.
- Commit discipline: stage by explicit path only; never `git add -A` / `-am` (pre-commit hook
  enforces; other loop sessions run in this repo concurrently).
- `rust/web/` stays untouched and serving until its replacement page passes the gate — the team
  uses this tool daily; no dark windows.
- Secrets stay in `.env`/1Password; the DeepSeek key never reaches client JS (proxy only).
- Real verification only: gates + screenshots, no "should work."

## Run it

Paste into `/loop` (self-paced):

```
/loop Work TILLER-LOOP-PROMPT.md top-down: take the highest unfinished backlog item, execute it
with the Method (page agents never touch shared files), run the full phase gate, screenshot-verify
parity at 1440px, commit by explicit path, update the backlog checkboxes in this file, then
continue. Stop only when P5 closeout is verified live through the tunnel.
```
