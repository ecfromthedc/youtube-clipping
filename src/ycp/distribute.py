"""Stage 7 — DISTRIBUTE. Approved clips → the connected owned channels.

PREFERRED: Postiz (public API). We hold POSTIZ_API_TOKEN and connect each YouTube
channel directly in Postiz; per approved clip the PostizAdapter uploads the mp4
(POST /upload) then creates a post (POST /posts) on that channel's integration id.

ALTERNATIVE: Repurpose.io (OutboxAdapter) — drop clip + a metadata sidecar into a
watch-folder Repurpose syncs and auto-posts. Kept as a swappable fallback.

Pick the path with `distribution.provider` (postiz | repurpose). Both sit behind the
`Adapter` protocol, so switching is config, not a rewrite. Full plan, one-time setup,
and the adapter contract: DISTRIBUTION.md.

Safety properties:
- Every clip clears `guardrails.publish_allowed` again right before delivery
  (transformed, no music, clean title) — defense in depth behind the manual Slack QC gate.
- DISABLED by default (`distribution.enabled: false`) until the token + channels are
  connected, so building/testing this never risks an accidental public post.
"""
from __future__ import annotations

import json
import os
import shutil
from datetime import datetime, time as dtime, timedelta
from pathlib import Path
from typing import Any, Protocol
from zoneinfo import ZoneInfo

import requests

from . import db, guardrails
from .config import ROOT, settings


# ── auto-QC (Eric's call §9) ──────────────────────────────────────────────────

def qc_decision(clip: dict[str, Any]) -> tuple[str, str]:
    """Auto-QC verdict for one clip. Pure.

    'approve' only if it clears the publish gate. fmt=='auto-clip' means the clip
    went through our cut + caption (+ hook) pipeline → transformed (not a raw
    reupload). The in-code filters are the only gate now, so this stays strict.
    """
    meta = {
        "transformed": clip.get("fmt") == "auto-clip",
        "has_music": bool(clip.get("has_music", False)),
        "title": clip.get("post_title") or clip.get("source_creator") or "",
    }
    ok, reason = guardrails.publish_allowed(meta)
    return ("approve", "") if ok else ("reject", reason)


def auto_qc(db_path: Any = None) -> dict[str, int]:
    """Apply the auto-QC verdict to every pending_qc clip. Returns counts."""
    counts = {"approved": 0, "rejected": 0}
    for clip in db.pending_qc_clips(db_path):
        decision, reason = qc_decision(clip)
        db.record_qc(clip["clip_id"], decision, reviewer="auto-qc", note=reason, db_path=db_path)
        counts["approved" if decision == "approve" else "rejected"] += 1
    return counts


# ── distribution adapter ──────────────────────────────────────────────────────

def hashtags_for(channel: str | None) -> list[str]:
    """Curated, niche-appropriate hashtags for a channel slug (from settings, tunable).
    Falls back to the `default` set, then to #shorts. Pure read."""
    tags = settings().get("distribution", {}).get("hashtags", {})
    return tags.get(channel or "") or tags.get("default") or ["#shorts"]


def caption_for(clip: dict[str, Any]) -> str:
    """Post caption for a clip: the hook title + the channel's hashtags. The burned hook
    is the on-screen title; the tags ride in the post description for discovery."""
    title = clip.get("post_title") or f"{clip.get('source_creator', '')} — clip".strip(" —")
    tags = " ".join(hashtags_for(clip.get("channel")))
    return f"{title}\n\n{tags}".strip()


def assign_slots(n: int, times: list[str], tz: str, start: datetime) -> list[str]:
    """The next `n` posting slots (ISO strings) drawn from `times` (HH:MM, channel-local),
    at/after `start`, rolling to following days. Pure — pass `start` in so it's testable."""
    if n <= 0:
        return []
    zone = ZoneInfo(tz)
    start = start.astimezone(zone)
    ordered = sorted(times)
    out: list[str] = []
    day = start.date()
    while len(out) < n:
        for hhmm in ordered:
            hh, mm = (int(x) for x in hhmm.split(":"))
            cand = datetime.combine(day, dtime(hh, mm, tzinfo=zone))
            if cand >= start:
                out.append(cand.isoformat())
                if len(out) == n:
                    break
        day += timedelta(days=1)
    return out


class Adapter(Protocol):
    def deliver(self, clip_path: Path, meta: dict) -> str: ...


class OutboxAdapter:
    """ALTERNATIVE (Repurpose.io): drops clip + a JSON metadata sidecar into the
    folder Repurpose watches and auto-posts from."""

    def __init__(self, outbox: Path):
        self.outbox = outbox

    def deliver(self, clip_path: Path, meta: dict) -> str:
        self.outbox.mkdir(parents=True, exist_ok=True)
        dest = self.outbox / clip_path.name
        if clip_path.exists():
            shutil.copy2(clip_path, dest)
        (self.outbox / f"{clip_path.stem}.json").write_text(json.dumps(meta, indent=2))
        return str(dest)


class PostizAdapter:
    """PREFERRED: posts approved clips to channels connected in Postiz via its public API.

    Per clip: upload the mp4 (POST /upload) → create a post (POST /posts) on the target
    channel's Postiz integration id. We hold POSTIZ_API_TOKEN (HANDOFF §9 / DISTRIBUTION.md).
    One-time human step: connect each channel in Postiz, then map our channel id → its
    integration id in `distribution.postiz.channels`.
    """

    def __init__(self, token: str, api_url: str, channels: dict[str, str],
                 schedule: str = "now"):
        self.token = token
        self.api_url = api_url.rstrip("/")
        self.channels = channels
        self.schedule = schedule

    @classmethod
    def from_config(cls, pz: dict) -> "PostizAdapter":
        token = os.environ.get(pz.get("token_env", "POSTIZ_API_TOKEN"), "")
        if not token:
            raise RuntimeError(
                "POSTIZ_API_TOKEN not set — add it to .env (see DISTRIBUTION.md / SETUP §3).")
        return cls(
            token=token,
            api_url=pz.get("api_url", "https://api.postiz.com/public/v1"),
            channels=pz.get("channels", {}),
            schedule=pz.get("schedule", "now"),
        )

    def deliver(self, clip_path: Path, meta: dict) -> str:
        integration_id = self.channels.get(meta.get("channel") or "")
        if not integration_id:
            raise RuntimeError(
                f"no Postiz integration id for channel {meta.get('channel')!r} — map it in "
                "distribution.postiz.channels (ids from GET /public/v1/integrations).")
        headers = {"Authorization": self.token}
        with clip_path.open("rb") as fh:
            up = requests.post(f"{self.api_url}/upload", headers=headers,
                               files={"file": (clip_path.name, fh, "video/mp4")}, timeout=180)
        up.raise_for_status()
        media = up.json()
        caption = meta.get("caption", "")
        body = {
            "type": self.schedule,
            "date": meta.get("date") or db.now(),
            "posts": [{
                "integration": {"id": integration_id},
                "value": [{"content": caption,
                           "image": [{"id": media.get("id"), "path": media.get("path")}]}],
                "settings": {"__type": meta.get("platform") or "youtube", "title": caption[:100]},
            }],
        }
        resp = requests.post(f"{self.api_url}/posts", headers=headers, json=body, timeout=60)
        resp.raise_for_status()
        out = resp.json()
        return str(out.get("id") or out.get("postId") or "posted")


def _resolve_outbox(cfg: dict) -> Path:
    path = Path(cfg.get("outbox", "data/outbox"))
    return path if path.is_absolute() else ROOT / path


def build_adapter(cfg: dict) -> Adapter:
    """Select the distribution adapter. Postiz (API) is preferred; Repurpose.io
    (outbox watch-folder) is the swappable alternative. See DISTRIBUTION.md."""
    provider = (cfg.get("provider") or cfg.get("adapter") or "postiz").lower()
    if provider.startswith("postiz"):
        return PostizAdapter.from_config(cfg.get("postiz", {}))
    if provider.startswith("repurpose") or "outbox" in provider:
        return OutboxAdapter(_resolve_outbox(cfg))
    raise RuntimeError(f"unknown distribution provider {provider!r} (use 'postiz' or 'repurpose').")


def run(db_path: Any = None) -> dict[str, Any]:
    """Hand approved clips to the distribution adapter, marking them posted.

    Gated by `distribution.enabled` (default off) until the provider is configured
    (Postiz token + channels, or Repurpose accounts). Re-checks the publish gate per
    clip — defense in depth behind the manual Slack QC gate.
    """
    cfg = settings().get("distribution", {})
    if not cfg.get("enabled", False):
        n = len(db.approved_clips(db_path))
        return {"enabled": False, "delivered": 0, "waiting": n,
                "note": "distribution OFF — set POSTIZ_API_TOKEN, connect channels in Postiz + "
                        "map them in distribution.postiz.channels, then set "
                        "distribution.enabled: true (see DISTRIBUTION.md)"}
    adapter = build_adapter(cfg)
    provider = (cfg.get("provider") or "postiz")
    pz = cfg.get("postiz", {})
    # Postiz: only channels with a mapped integration id can post. Clips for channels not
    # connected yet are PARKED (left approved so they flush once mapped) — a not-yet-set-up
    # channel must never crash the whole distribute batch and block the connected ones.
    mapped = ({k for k, v in (pz.get("channels") or {}).items() if v}
              if provider.startswith("postiz") else None)
    postable, parked = [], 0
    for clip in db.approved_clips(db_path):
        if mapped is not None and clip.get("channel") not in mapped:
            parked += 1
        else:
            postable.append(clip)

    # Schedule mode → assign each postable clip to the next free posting slot so the
    # channel posts on a steady cadence instead of dumping the whole batch at once.
    slots: list[str] = []
    if provider.startswith("postiz") and pz.get("schedule") == "schedule":
        tz = pz.get("timezone", "UTC")
        slots = assign_slots(len(postable), pz.get("posting_times", []), tz,
                             datetime.now(ZoneInfo(tz)))

    delivered = blocked = failed = 0
    for i, clip in enumerate(postable):
        meta = {
            "transformed": clip.get("fmt") == "auto-clip",
            "has_music": bool(clip.get("has_music", False)),
            "title": caption_for(clip),
        }
        ok, reason = guardrails.publish_allowed(meta)
        if not ok:
            db.set_clip_status(clip["clip_id"], "rejected", db_path=db_path)
            blocked += 1
            continue
        try:
            dest = adapter.deliver(Path(clip.get("post_url") or ""), {
                "clip_id": clip["clip_id"], "caption": caption_for(clip),
                "channel": clip.get("channel"), "platform": clip.get("platform"),
                "date": slots[i] if i < len(slots) else None,
            })
        except Exception as exc:  # noqa: BLE001 — one clip's failure must not kill the batch
            print(f"  ! post failed for {clip['clip_id']} ({clip.get('channel')}): {str(exc)[:140]}")
            failed += 1
            continue
        db.set_clip_status(clip["clip_id"], "posted", db_path=db_path,
                           post_url=dest, posted_at=db.now())
        delivered += 1
    return {"enabled": True, "delivered": delivered, "blocked": blocked,
            "parked": parked, "failed": failed}
