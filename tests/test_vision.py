"""Vision moment-selector — pure parsing + the disabled/fail-open fallbacks (no live API)."""
from __future__ import annotations

from ycp import vision


def test_parse_moments_filters_bad_and_ranks_by_score():
    raw = [
        {"start_sec": 10, "end_sec": 40, "score": 0.9, "reason": "hook"},
        {"start_sec": 5, "end_sec": 5, "score": 0.5},          # zero-length -> dropped
        {"start_sec": "x", "end_sec": 20},                       # unparseable -> dropped
        {"start_sec": 60, "end_sec": 90, "score": 0.7, "reason": "payoff"},
    ]
    ms = vision._parse_moments(raw)
    assert [(m.start, m.end) for m in ms] == [(10.0, 40.0), (60.0, 90.0)]  # score desc
    assert ms[0].score == 0.9 and ms[0].duration == 30.0


def test_rank_moments_returns_empty_when_disabled(monkeypatch):
    monkeypatch.setattr(vision, "enabled", lambda: False)
    assert vision.rank_moments("/nonexistent.mp4") == []


def test_qc_screen_fails_open_when_disabled(monkeypatch):
    monkeypatch.setattr(vision, "enabled", lambda: False)
    assert vision.qc_screen("/nonexistent.mp4") == {"ok": True, "flags": []}
