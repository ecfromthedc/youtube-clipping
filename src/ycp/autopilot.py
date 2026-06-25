"""Stage 6 — AUTOPILOT. One command that chains the daily closed loop.

    source → clip (top-N queued, idempotent) → qc → capture → brief → scoreboard

Distribution (→ Postiz, preferred; Repurpose.io alternative) slots in as a stage once
the token is set + channels connected once (see DISTRIBUTION.md / HANDOFF.md §8 #3 / §9).

Design goals:
- **Idempotent / safe to re-run.** Already-clipped sources are skipped (no
  re-download); every underlying writer is `ON CONFLICT`-safe.
- **Fault-isolated.** Each stage is wrapped: one stage failing is logged and the
  chain continues, so a transient yt-dlp hiccup never strands the brief/scoreboard.
- **Honest about gates.** Distribution and auto-QC are reported as PENDING until
  their build cycles land (filters before auto-posting — HANDOFF §9 QC=auto).

The stage-selection logic (`select_unclipped`) is pure and unit-tested; the
`run` chain is verified by a real invocation (`ycp autopilot --skip-source`).
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Callable

from . import brief as brief_mod
from . import capture as capture_mod
from . import clip as clip_mod
from . import db
from . import scoreboard as scoreboard_mod
from . import sourcing as sourcing_mod
from .config import ROOT, settings

# Owned channels are the only lane the factory clips for (Whop cut 2026-06; pure owned-first).
DEFAULT_LANES: tuple[str, ...] = ("owned",)


def angle_for(niche: str | None) -> str:
    """Map a niche label → a hook-agent angle (tunes the viral-hook prompt). Pure."""
    n = (niche or "").lower()
    if "debate" in n or "agitation" in n or "hot seat" in n:
        return "agitation"
    if "finance" in n or "money" in n:
        return "finance"
    return ""


# niche name (niches.yaml `name:`) → owned-channel slug (the Postiz routing key in
# settings.yaml distribution.postiz.channels). ponytail: explicit 5-entry map; if the
# channel set ever goes dynamic, add a `channel:` key per niche group in niches.yaml.
CHANNEL_SLUGS: dict[str, str] = {
    "debate-agitation": "hot-seat",
    "finance-money": "money-fights",
    "comedy-crashout": "crash-out",
    "health-mythbusting": "phoenix-protocol",
    "business-finance": "boardroom",
}


def channel_for(niche: str | None) -> str:
    """Map a niche label → its owned-channel slug (the Postiz routing key). Pure.

    Unknown niches fall back to 'clips', which won't match a configured Postiz
    integration → distribution raises loudly rather than posting to the wrong channel.
    """
    return CHANNEL_SLUGS.get((niche or "").lower(), "clips")


@dataclass(frozen=True)
class StageResult:
    name: str
    ok: bool
    detail: str

    def line(self) -> str:
        mark = "✓" if self.ok else "✗"
        return f"  {mark} {self.name:<11} {self.detail}"


def select_unclipped(
    queue: list[dict[str, Any]],
    clipped_ids: set[str],
    max_videos: int,
    lanes: tuple[str, ...] = DEFAULT_LANES,
) -> list[dict[str, Any]]:
    """Pick the top `max_videos` queued rows not yet clipped, in allowed lanes. Pure.

    `queue` is assumed already ranked (hottest first). A row needs a `url`; rows
    whose `video_id` is in `clipped_ids` (or whose lane isn't allowed) are skipped.
    """
    picked: list[dict[str, Any]] = []
    for row in queue:
        if max_videos and len(picked) >= max_videos:
            break
        if row.get("lane") not in lanes:
            continue
        if not row.get("url"):
            continue
        if row.get("video_id") in clipped_ids:
            continue
        picked.append(row)
    return picked


def _stage(name: str, fn: Callable[[], str], results: list[StageResult],
           log: Callable[[str], None]) -> None:
    """Run one stage, capture ok/detail, never raise out of the chain."""
    try:
        detail = fn()
        res = StageResult(name, True, detail)
    except Exception as exc:  # noqa: BLE001 — fault isolation is the whole point
        res = StageResult(name, False, f"{type(exc).__name__}: {str(exc)[:160]}")
    results.append(res)
    log(res.line())


def run(
    max_videos: int = 5,
    *,
    skip_source: bool = False,
    do_clip: bool = True,
    hook_cta: bool = True,
    lanes: tuple[str, ...] = DEFAULT_LANES,
    db_path: Any = None,
    log: Callable[[str], None] = print,
) -> list[StageResult]:
    """Chain the daily loop end-to-end. Returns a per-stage result list."""
    results: list[StageResult] = []
    db.init_db(db_path)
    log("▶ autopilot: source → clip → qc → capture → brief → scoreboard")

    # 1 ─ SOURCE ───────────────────────────────────────────────────────────────
    queue: list[dict[str, Any]] = []

    def _source() -> str:
        nonlocal queue
        if skip_source:
            queue = db.source_queue(db_path)
            return f"reused {len(queue)} queued (skip-source)"
        queue = sourcing_mod.run(db_path=db_path)
        (ROOT / "data" / "source-queue.md").write_text(sourcing_mod.render_queue_md(queue))
        return f"{len(queue)} videos queued → data/source-queue.md"

    _stage("source", _source, results, log)

    # 2 ─ CLIP (idempotent) ──────────────────────────────────────────────────────
    def _clip() -> str:
        if not do_clip:
            return "skipped (--no-clip)"
        clipped = db.clipped_source_ids(db_path)
        todo = select_unclipped(queue, clipped, max_videos, lanes)
        if not todo:
            return "0 new sources to clip (all caught up)"
        made = 0
        for row in todo:
            created = clip_mod.run(
                row["url"],
                lane=row.get("lane", "owned"),
                source_creator=row.get("creator", "unknown"),
                source_video_id=row.get("video_id"),
                channel=channel_for(row.get("niche")),
                hook_cta=hook_cta,
                angle=angle_for(row.get("niche")),
                captions_on=not row.get("has_captions", False),  # defer if source has captions
                db_path=db_path,
            )
            made += len(created)
        return f"{made} clips from {len(todo)} sources (pending_qc)"

    _stage("clip", _clip, results, log)

    # 3 ─ QC ─────────────────────────────────────────────────────────────────────
    # Eric's call (§9): QC is MANUAL — clips post to the Slack QC channel and nothing
    # advances until he ✅'s them. `qc.auto: true` (later, once trust is earned) flips
    # to auto-approve via the in-code guardrail filters.
    def _qc() -> str:
        if not db.pending_qc_clips(db_path):
            return "no clips pending QC"
        from . import qc as qc_mod
        r = qc_mod.dispatch_pending(db_path)
        if "dispatched" in r:
            return f"{r['dispatched']} dispatched for review via {r['channel']}"
        return f"auto-QC: {r.get('approved', 0)} approved, {r.get('rejected', 0)} rejected (guardrails)"

    _stage("qc", _qc, results, log)

    # 4 ─ CAPTURE ────────────────────────────────────────────────────────────────
    # Resolve Postiz post_ids → YouTube URLs, snapshot public views, then pull owned
    # retention/revenue (no-op until creds + published videos exist).
    def _capture() -> str:
        pub = capture_mod.capture_public(db_path=db_path)
        full = capture_mod.capture_full_analytics(db_path=db_path)
        return f"{pub} public-view + {full} owned-analytics snapshots"

    _stage("capture", _capture, results, log)

    # 5 ─ BRIEF ──────────────────────────────────────────────────────────────────
    def _brief() -> str:
        import datetime
        dframe = db.clips_with_latest_metrics(db_path)
        md = brief_mod.build(dframe)
        db.save_brief(week_start=datetime.date.today().isoformat(), content=md, db_path=db_path)
        (ROOT / "data" / "latest-brief.md").write_text(md)
        return "Double-Down Brief → data/latest-brief.md"

    _stage("brief", _brief, results, log)

    # 6 ─ SCOREBOARD ─────────────────────────────────────────────────────────────
    def _scoreboard() -> str:
        md = scoreboard_mod.build(db.clips_with_latest_metrics(db_path))
        (ROOT / "SCOREBOARD.md").write_text(md)
        return "Race to $15K → SCOREBOARD.md"

    _stage("scoreboard", _scoreboard, results, log)

    # 6.5 ─ OPTIMIZE ─────────────────────────────────────────────────────────────
    # The actuator: turn the scoreboard's scale/kill verdicts into source weights for
    # the NEXT cycle (double down on winners, starve losers) + journal it to IMPROVEMENT-LOG.md.
    def _optimize() -> str:
        from . import optimize
        r = optimize.run(db_path)
        return (f"learned from {r['clips']} clips → +{len(r['boosted'])} boosted / "
                f"-{len(r['suppressed'])} starved (→ IMPROVEMENT-LOG.md)")

    _stage("optimize", _optimize, results, log)

    # 7 ─ DISTRIBUTE ─────────────────────────────────────────────────────────────
    # Postiz (preferred) / Repurpose.io (alternative) per distribution.provider. Stays
    # OFF until the token + channels are connected + distribution.enabled — so it reports
    # the gate instead of posting. See DISTRIBUTION.md.
    def _distribute() -> str:
        from . import distribute
        r = distribute.run(db_path)
        if not r["enabled"]:
            return f"OFF — {r['waiting']} approved clips waiting; {r['note']}"
        prov = settings().get("distribution", {}).get("provider", "postiz")
        return (f"delivered {r['delivered']} via {prov} ({r.get('parked', 0)} parked "
                f"[channel not connected], {r.get('blocked', 0)} blocked, {r.get('failed', 0)} failed)")

    _stage("distribute", _distribute, results, log)

    ok = sum(1 for r in results if r.ok)
    log(f"▶ autopilot done: {ok}/{len(results)} stages ok")
    return results
