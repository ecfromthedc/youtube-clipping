"""Distribution — auto-QC verdict + outbox/Postiz adapters (network mocked)."""
from __future__ import annotations

from pathlib import Path

import pytest

from ycp import distribute


def test_auto_qc_approves_transformed_clean_clip():
    decision, _ = distribute.qc_decision(
        {"fmt": "auto-clip", "source_creator": "Ramit Sethi"})
    assert decision == "approve"


def test_auto_qc_rejects_untransformed_clip():
    # fmt != auto-clip → treated as raw reupload → rejected
    decision, reason = distribute.qc_decision({"fmt": "raw", "source_creator": "x"})
    assert decision == "reject" and "transform" in reason.lower()


def test_caption_falls_back_to_creator():
    cap = distribute.caption_for({"source_creator": "Codie Sanchez"})
    assert cap.startswith("Codie Sanchez — clip")
    assert "#shorts" in cap                       # default hashtags when no channel


def test_caption_includes_channel_hashtags():
    cap = distribute.caption_for(
        {"post_title": "muscle is the organ of longevity:", "channel": "phoenix-protocol"})
    assert cap.startswith("muscle is the organ of longevity:")
    assert "#longevity" in cap and "#health" in cap


def test_assign_slots_rolls_across_days_in_order():
    from datetime import datetime
    from zoneinfo import ZoneInfo
    start = datetime(2026, 6, 24, 7, 0, tzinfo=ZoneInfo("America/New_York"))  # past 06:00
    slots = distribute.assign_slots(
        4, ["06:00", "12:30", "19:00"], "America/New_York", start)
    times = [datetime.fromisoformat(s) for s in slots]
    assert len(times) == 4
    assert times == sorted(times)                 # strictly chronological
    assert (times[0].hour, times[0].minute) == (12, 30)   # 06:00 today already passed
    assert times[2].date() > times[0].date()      # 4th slot rolled to the next day


def test_assign_slots_empty_when_zero():
    from datetime import datetime
    from zoneinfo import ZoneInfo
    assert distribute.assign_slots(
        0, ["06:00"], "UTC", datetime(2026, 6, 24, tzinfo=ZoneInfo("UTC"))) == []


def test_outbox_adapter_writes_clip_and_sidecar(tmp_path):
    src = tmp_path / "clip.mp4"
    src.write_bytes(b"fake mp4")
    adapter = distribute.OutboxAdapter(tmp_path / "outbox")
    dest = adapter.deliver(src, {"clip_id": "abc", "caption": "Hook here"})
    assert Path(dest).exists()
    sidecar = (tmp_path / "outbox" / "clip.json")
    assert sidecar.exists() and "Hook here" in sidecar.read_text()


def test_run_disabled_reports_gate(monkeypatch):
    # Force the disabled path (live config now ships enabled: true for production).
    monkeypatch.setattr(distribute, "settings",
                        lambda: {"distribution": {"enabled": False, "provider": "postiz"}})
    result = distribute.run(db_path=None)
    assert result["enabled"] is False and "Postiz" in result["note"]


class _FakeResp:
    def __init__(self, data):
        self._data = data

    def raise_for_status(self):
        pass

    def json(self):
        return self._data


def test_postiz_adapter_uploads_then_posts(monkeypatch, tmp_path):
    calls = []

    def fake_post(url, **kw):
        calls.append(url)
        if url.endswith("/upload"):
            return _FakeResp({"id": "img-1", "path": "https://uploads/clip.mp4"})
        return _FakeResp({"id": "post-9"})

    monkeypatch.setattr(distribute.requests, "post", fake_post)
    src = tmp_path / "clip.mp4"
    src.write_bytes(b"fake mp4")
    adapter = distribute.PostizAdapter(
        token="t", api_url="https://api.postiz.com/public/v1",
        channels={"hot-seat": "intg-1"})
    out = adapter.deliver(src, {"channel": "hot-seat", "caption": "Hook", "platform": "youtube"})
    assert out == "post-9"
    assert any(u.endswith("/upload") for u in calls)
    assert any(u.endswith("/posts") for u in calls)


def test_postiz_adapter_unknown_channel_raises(tmp_path):
    src = tmp_path / "c.mp4"
    src.write_bytes(b"x")
    adapter = distribute.PostizAdapter(token="t", api_url="x", channels={})
    with pytest.raises(RuntimeError, match="integration id"):
        adapter.deliver(src, {"channel": "nope", "caption": "h"})


def test_postiz_from_config_requires_token(monkeypatch):
    monkeypatch.delenv("POSTIZ_API_TOKEN", raising=False)
    with pytest.raises(RuntimeError, match="POSTIZ_API_TOKEN"):
        distribute.PostizAdapter.from_config({})


def test_build_adapter_selects_provider(monkeypatch, tmp_path):
    monkeypatch.setenv("POSTIZ_API_TOKEN", "tok")
    assert isinstance(distribute.build_adapter({"provider": "postiz"}), distribute.PostizAdapter)
    assert isinstance(
        distribute.build_adapter({"provider": "repurpose", "outbox": str(tmp_path)}),
        distribute.OutboxAdapter)
