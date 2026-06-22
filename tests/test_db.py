"""DB lifecycle tests — clips, metrics snapshots, QC decisions."""
from __future__ import annotations

from ycp import db


def test_init_and_clip_lifecycle(tmp_path):
    dbp = tmp_path / "t.db"
    db.init_db(dbp)
    db.insert_clip({
        "clip_id": "c1", "channel": "ch", "platform": "youtube", "lane": "whop",
        "fmt": "list", "hook_type": "question", "length_sec": 30,
    }, dbp)
    # latest-metrics join works even before any metric exists
    df = db.clips_with_latest_metrics(dbp)
    assert len(df) == 1 and df.iloc[0]["views"] == 0


def test_latest_metric_wins(tmp_path):
    dbp = tmp_path / "t.db"
    db.init_db(dbp)
    db.insert_clip({"clip_id": "c1", "channel": "ch", "platform": "youtube",
                    "lane": "whop", "fmt": "list", "hook_type": "q", "length_sec": 30}, dbp)
    db.insert_metric({"clip_id": "c1", "views": 100, "captured_at": "2026-01-01T00:00:00"}, dbp)
    db.insert_metric({"clip_id": "c1", "views": 5000, "captured_at": "2026-01-09T00:00:00"}, dbp)
    df = db.clips_with_latest_metrics(dbp)
    assert df.iloc[0]["views"] == 5000  # most recent snapshot used


def test_qc_decision_updates_status(tmp_path):
    dbp = tmp_path / "t.db"
    db.init_db(dbp)
    db.insert_clip({"clip_id": "c1", "channel": "ch", "platform": "youtube",
                    "lane": "owned", "fmt": "list", "hook_type": "q", "length_sec": 30}, dbp)
    assert len(db.pending_qc_clips(dbp)) == 1
    db.record_qc("c1", "approve", reviewer="U123", db_path=dbp)
    assert len(db.pending_qc_clips(dbp)) == 0


def test_slack_ts_lookup(tmp_path):
    dbp = tmp_path / "t.db"
    db.init_db(dbp)
    db.insert_clip({"clip_id": "c1", "channel": "ch", "platform": "youtube",
                    "lane": "owned", "fmt": "list", "hook_type": "q", "length_sec": 30}, dbp)
    db.set_clip_status("c1", "pending_qc", db_path=dbp, slack_ts="1700000000.0001")
    assert db.clip_by_slack_ts("1700000000.0001", dbp) == "c1"
    assert db.clip_by_slack_ts("nope", dbp) is None
