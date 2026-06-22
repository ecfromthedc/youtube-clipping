"""Capture tests — Whop CSV import robustness (NaN payouts, unknown clips)."""
from __future__ import annotations

import pandas as pd

from ycp import capture, db


def test_import_whop_csv_skips_unknown_and_survives_nan(tmp_path):
    dbp = tmp_path / "t.db"
    db.init_db(dbp)
    db.insert_clip({"clip_id": "c1", "channel": "ch", "platform": "youtube",
                    "lane": "whop", "fmt": "x", "hook_type": "q", "length_sec": 30}, dbp)
    csv = tmp_path / "whop.csv"
    pd.DataFrame([
        {"clip_id": "c1", "payout": 12.5, "views": 5000},
        {"clip_id": "c1", "payout": None, "views": 1000},      # NaN payout -> 0.0, no crash
        {"clip_id": "ghost", "payout": 9.0, "views": 100},     # unknown clip -> skipped (no FK error)
    ]).to_csv(csv, index=False)

    n = capture.import_whop_csv(csv, dbp)
    assert n == 2  # both c1 rows imported; "ghost" skipped


def test_import_whop_csv_url_keyed(tmp_path):
    dbp = tmp_path / "t.db"
    db.init_db(dbp)
    db.insert_clip({"clip_id": "c1", "channel": "ch", "platform": "youtube", "lane": "whop",
                    "fmt": "x", "hook_type": "q", "length_sec": 30,
                    "post_url": "https://x.com/c1"}, dbp)
    csv = tmp_path / "whop.csv"
    pd.DataFrame([{"url": "https://x.com/c1", "earnings": 8.0}]).to_csv(csv, index=False)
    assert capture.import_whop_csv(csv, dbp) == 1
