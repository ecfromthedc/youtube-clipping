"""Milestone watcher — pure threshold-crossing logic (no network)."""
from __future__ import annotations

from ycp import milestones


def test_crossed_fires_each_passed_threshold_once():
    fired: set[str] = set()
    new = milestones.crossed(600, milestones.SUBS, fired, "subs")
    joined = " ".join(new)
    assert "100 subscribers" in joined and "ENTRY tier" in joined   # 100 + 500 crossed
    assert "subs:100" in fired and "subs:500" in fired
    assert "subs:1000" not in fired                                  # 600 < 1000


def test_crossed_is_idempotent():
    fired: set[str] = set()
    milestones.crossed(600, milestones.SUBS, fired, "subs")
    assert milestones.crossed(600, milestones.SUBS, fired, "subs") == []  # nothing new second pass


def test_label_specials():
    assert "ENTRY tier" in milestones._label("subs", 500)
    assert "THE GOAL" in milestones._label("rev", 15_000)
    assert milestones._label("subs", 5000) == "5,000 subscribers"
