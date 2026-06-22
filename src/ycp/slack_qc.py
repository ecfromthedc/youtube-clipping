"""Stage 3 — APPROVE, wired to Slack.

Posts each pending clip as a card to your #clip-qc channel. A reviewer reacts
✅ (approve) or ❌ (reject); the decision writes straight back to the DB via
Socket Mode (no public URL / webhook server needed).

Slack SDK is an optional dependency — import it lazily so importing this module
never breaks the rest of the package. Install with:  uv pip install -e '.[slack]'
"""
from __future__ import annotations

from pathlib import Path

from . import db
from .config import env

APPROVE = {"white_check_mark", "heavy_check_mark", "+1", "thumbsup"}
REJECT = {"x", "negative_squared_cross_mark", "-1", "thumbsdown", "no_entry"}


def _require_slack():
    try:
        from slack_sdk import WebClient  # noqa: F401
    except ImportError as exc:  # pragma: no cover
        raise RuntimeError(
            "Slack support not installed. Run:  uv pip install -e '.[slack]'"
        ) from exc


def _client():
    _require_slack()
    from slack_sdk import WebClient

    token = env()["slack_bot_token"]
    if not token:
        raise RuntimeError("SLACK_BOT_TOKEN missing in .env")
    return WebClient(token=token)


def _card_blocks(clip: dict) -> list[dict]:
    lane = clip.get("lane", "?")
    badge = "💰 Whop" if lane == "whop" else "📺 Owned/YPP"
    link = f"\n<{clip['post_url']}|▶︎ Preview clip>" if clip.get("post_url") else ""
    return [
        {"type": "header", "text": {"type": "plain_text", "text": f"🎬 QC · {clip['clip_id']}"}},
        {"type": "section", "fields": [
            {"type": "mrkdwn", "text": f"*Lane:*\n{badge}"},
            {"type": "mrkdwn", "text": f"*Channel:*\n{clip.get('channel','?')} · {clip.get('platform','?')}"},
            {"type": "mrkdwn", "text": f"*Source:*\n{clip.get('source_creator','?')}"},
            {"type": "mrkdwn", "text": f"*Format / Hook:*\n{clip.get('fmt','?')} · {clip.get('hook_type','?')}"},
            {"type": "mrkdwn", "text": f"*Length:*\n{clip.get('length_sec','?')}s"},
        ]},
        {"type": "context", "elements": [{"type": "mrkdwn",
            "text": ("React ✅ to *approve & schedule* · ❌ to *reject*. "
                     "Check: hook in 1–2s · transformation present (owned) · "
                     "no copyrighted music · Whop rules met." + link)}]},
    ]


def post_pending(db_path: Path | None = None) -> int:
    """Post every pending_qc clip to the QC channel; store the message ts."""
    client = _client()
    channel = env()["slack_qc_channel"]
    if not channel:
        raise RuntimeError("SLACK_QC_CHANNEL missing in .env")
    clips = db.pending_qc_clips(db_path)
    for clip in clips:
        resp = client.chat_postMessage(
            channel=channel, blocks=_card_blocks(clip),
            text=f"QC clip {clip['clip_id']}",
        )
        ts = resp["ts"]
        db.set_clip_status(clip["clip_id"], "pending_qc", db_path=db_path, slack_ts=ts)
        for emoji in ("white_check_mark", "x"):
            try:
                client.reactions_add(channel=channel, name=emoji, timestamp=ts)
            except Exception:  # noqa: BLE001  (affordance only; non-fatal)
                pass
    return len(clips)


def run_listener(db_path: Path | None = None) -> None:
    """Block on Socket Mode, routing ✅/❌ reactions to QC decisions."""
    _require_slack()
    from slack_bolt import App
    from slack_bolt.adapter.socket_mode import SocketModeHandler

    e = env()
    if not (e["slack_bot_token"] and e["slack_app_token"]):
        raise RuntimeError("SLACK_BOT_TOKEN and SLACK_APP_TOKEN required for the listener")
    app = App(token=e["slack_bot_token"])

    @app.event("reaction_added")
    def _on_reaction(event, client):  # noqa: ANN001
        reaction, ts, user = event.get("reaction"), event["item"].get("ts"), event.get("user")
        clip_id = db.clip_by_slack_ts(ts, db_path) if ts else None
        if not clip_id:
            return
        if reaction in APPROVE:
            decision, mark = "approve", "✅ Approved → scheduling"
        elif reaction in REJECT:
            decision, mark = "reject", "❌ Rejected"
        else:
            return
        db.record_qc(clip_id, decision, reviewer=user, db_path=db_path)
        if decision == "approve":
            db.set_clip_status(clip_id, "scheduled", db_path=db_path)
        try:
            client.chat_update(channel=event["item"]["channel"], ts=ts,
                               text=f"{mark} · {clip_id}")
        except Exception:  # noqa: BLE001
            pass

    print(f"QC listener live on channel {e['slack_qc_channel']}. Ctrl-C to stop.")
    SocketModeHandler(app, e["slack_app_token"]).start()


def post_brief(markdown: str, db_path: Path | None = None) -> None:
    """Drop the weekly Double-Down Brief into the QC channel."""
    client = _client()
    channel = env()["slack_qc_channel"]
    client.chat_postMessage(channel=channel, text=markdown, mrkdwn=True)
