"""Scoring engine tests — the deterministic core must be provably correct."""
from __future__ import annotations

import pandas as pd

from ycp import db, mock, scoring


def _scored_demo(tmp_path):
    dbp = tmp_path / "t.db"
    mock.seed(dbp)
    return db.clips_with_latest_metrics(dbp)


def test_length_bucket():
    assert scoring.length_bucket(18) == "15-25s"
    assert scoring.length_bucket(33) == "25-35s"
    assert scoring.length_bucket(None) == "unknown"
    assert scoring.length_bucket(120).endswith("s+")


def test_scores_bounded_and_ordered(tmp_path):
    scored = scoring.compute_scores(_scored_demo(tmp_path))
    assert scored["virality_score"].between(0, 100).all()
    # Flagrant debate-moment (winner) must outscore RandomVlogger reaction (loser).
    win = scored[scored["source_creator"] == "Flagrant"]["virality_score"].mean()
    lose = scored[scored["source_creator"] == "RandomVlogger"]["virality_score"].mean()
    assert win > lose


def test_revenue_per_1k_no_div_by_zero():
    df = pd.DataFrame([{"views": 0, "ad_revenue": 0, "whop_payout": 0,
                        "length_sec": 30, "retention_pct": 0}])
    out = scoring.add_derived(df)
    assert out["revenue_per_1k"].iloc[0] == 0.0


def test_rollup_respects_min_sample(tmp_path):
    scored = scoring.compute_scores(_scored_demo(tmp_path))
    combo = scoring.rollup(scored, scoring.ACTION_DIMS, min_sample=4)
    assert (combo["n"] >= 4).all()
    assert not combo.empty


def test_scale_and_kill_picks_right_combos(tmp_path):
    res = scoring.analyze(_scored_demo(tmp_path))
    scale_creators = set(res["scale"]["source_creator"])
    kill_creators = set(res["kill"]["source_creator"])
    assert "Flagrant" in scale_creators           # engineered winner scales
    assert "RandomVlogger" in kill_creators        # engineered loser is cut


def test_empty_input_safe():
    empty = pd.DataFrame()
    assert scoring.compute_scores(empty).empty
    assert scoring.rollup(empty, scoring.ACTION_DIMS).empty
    scale, kill = scoring.scale_and_kill(scoring.rollup(empty, ["fmt"]))
    assert scale.empty and kill.empty
