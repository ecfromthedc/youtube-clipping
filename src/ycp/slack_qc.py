"""Stage 3 — APPROVE, wired to Slack.

Uploads each pending clip's actual mp4 to the QC channel with a context caption, then
adds ✅/❌ reactions. A reviewer reacts to approve/reject; the decision writes straight
back to the DB via Socket Mode (no public URL / webhook server needed).

Slack SDK is an optional dependency — import it lazily so importing this module never
breaks the rest of the package. Install with:  uv pip install -e '.[slack]'
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


def _card_text(clip: dict) -> str:
    """The QC caption (mrkdwn) that rides with the uploaded clip."""
    return (
        f"🎬 *QC · {clip['clip_id']}*  ·  source: *{clip.get('source_creator', '?')}*\n"
        f"*Channel:* {clip.get('channel', '?')} · {clip.get('platform', '?')}   "
        f"*Length:* {clip.get('length_sec', '?')}s   *Format:* {clip.get('fmt', '?')}\n"
        "React ✅ to *approve & schedule* · ❌ to *reject*.  "
        "Check: hook lands in 1–2s · transformed (not a raw reupload) · no copyrighted music."
    )


def _file_id(resp) -> str | None:
    files = resp.get("files") or ([resp["file"]] if resp.get("file") else [])
    return files[0].get("id") if files else None


def _uploaded_ts(client, channel: str, file_id: str | None) -> str | None:
    """Find the ts of the message that actually carries our uploaded file (for reaction
    routing). files_upload_v2's own share map lags, so match by file id in recent history."""
    try:
        msgs = client.conversations_history(channel=channel, limit=8).get("messages") or []
    except Exception:  # noqa: BLE001
        return None
    for m in msgs:
        if file_id and file_id in {f.get("id") for f in m.get("files", [])}:
            return m.get("ts")
    return msgs[0].get("ts") if msgs else None


def post_pending(db_path: Path | None = None) -> int:
    """Upload every pending_qc clip (the real mp4) to the QC channel; store the message ts."""
    client = _client()
    channel = env()["slack_qc_channel"]
    if not channel:
        raise RuntimeError("SLACK_QC_CHANNEL missing in .env")
    clips = db.pending_qc_clips(db_path)
    for clip in clips:
        path = clip.get("post_url") or ""
        caption = _card_text(clip)
        if path and Path(path).exists():
            resp = client.files_upload_v2(
                channel=channel, file=path,
                title=f"{clip['clip_id']} · {clip.get('source_creator', '?')}",
                initial_comment=caption)
            ts = _uploaded_ts(client, channel, _file_id(resp))
        else:
            # No local file (e.g. already posted/distributed) — fall back to a text card.
            ts = client.chat_postMessage(channel=channel, text=caption, mrkdwn=True).get("ts")
        if ts:
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
    post_message(markdown)


def post_message(text: str) -> None:
    """Post a plain mrkdwn message to the QC channel (briefs, milestone alerts, etc.)."""
    client = _client()
    client.chat_postMessage(channel=env()["slack_qc_channel"], text=text, mrkdwn=True)
