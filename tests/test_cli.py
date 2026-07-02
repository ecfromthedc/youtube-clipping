"""CLI arg-parsing + routing tests — pure, no network."""
from __future__ import annotations

import pytest

from ycp import cli


def test_parse_ts_mm_ss():
    assert cli._parse_ts("1:30") == 90.0


def test_parse_ts_hh_mm_ss():
    assert cli._parse_ts("1:02:03") == 3723.0


def test_parse_ts_bare_seconds():
    assert cli._parse_ts("45") == 45.0
    assert cli._parse_ts("45.5") == 45.5


def test_parse_ts_garbage_raises():
    with pytest.raises(ValueError):
        cli._parse_ts("not-a-timestamp")
    with pytest.raises(ValueError):
        cli._parse_ts("1:2:3:4")
    with pytest.raises(ValueError):
        cli._parse_ts("")


def test_clip_from_to_routes_into_exact(monkeypatch):
    """--from/--to must call clip.run(..., exact=(from_sec, to_sec)) — no moment-picking."""
    captured = {}

    def fake_run(url, **kwargs):
        captured["url"] = url
        captured["kwargs"] = kwargs
        return [{"clip_id": "abc", "len": 30, "score": 1.0, "preview": "hi"}]

    from ycp import clip as clip_mod
    monkeypatch.setattr(clip_mod, "run", fake_run)

    parser = cli.build_parser()
    args = parser.parse_args(["clip", "https://example.com/v", "--from", "1:30", "--to", "2:00"])
    rc = args.fn(args)

    assert rc == 0
    assert captured["url"] == "https://example.com/v"
    assert captured["kwargs"]["exact"] == (90.0, 120.0)
    # exact mode must not also pass a moment-picking download window
    assert "window_sec" not in captured["kwargs"]
    assert "start_sec" not in captured["kwargs"]


def test_clip_without_from_to_keeps_existing_behavior(monkeypatch):
    """No --from/--to: unchanged start/window minute-based behavior, no exact= kwarg."""
    captured = {}

    def fake_run(url, **kwargs):
        captured["kwargs"] = kwargs
        return [{"clip_id": "abc", "len": 30, "score": 1.0, "preview": "hi"}]

    from ycp import clip as clip_mod
    monkeypatch.setattr(clip_mod, "run", fake_run)

    parser = cli.build_parser()
    args = parser.parse_args(["clip", "https://example.com/v", "--start", "42", "--window", "8"])
    rc = args.fn(args)

    assert rc == 0
    assert "exact" not in captured["kwargs"]
    assert captured["kwargs"]["start_sec"] == 42 * 60
    assert captured["kwargs"]["window_sec"] == 8 * 60


def test_clip_to_must_be_after_from(monkeypatch, capsys):
    from ycp import clip as clip_mod
    monkeypatch.setattr(clip_mod, "run", lambda *a, **k: pytest.fail("should not be called"))

    parser = cli.build_parser()
    args = parser.parse_args(["clip", "https://example.com/v", "--from", "2:00", "--to", "1:00"])
    rc = args.fn(args)

    assert rc == 1
    assert "must be after" in capsys.readouterr().out


def test_clip_from_without_to_errors(monkeypatch, capsys):
    from ycp import clip as clip_mod
    monkeypatch.setattr(clip_mod, "run", lambda *a, **k: pytest.fail("should not be called"))

    parser = cli.build_parser()
    args = parser.parse_args(["clip", "https://example.com/v", "--from", "1:00"])
    rc = args.fn(args)

    assert rc == 1
    assert "together" in capsys.readouterr().out
