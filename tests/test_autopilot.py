"""Autopilot orchestrator — pure stage-selection logic (no network/ffmpeg)."""
from __future__ import annotations

from ycp import autopilot

QUEUE = [
    {"video_id": "a", "url": "https://y/a", "lane": "owned", "view_velocity": 9000},
    {"video_id": "b", "url": "https://y/b", "lane": "owned", "view_velocity": 8000},
    {"video_id": "c", "url": "https://y/c", "lane": "other", "view_velocity": 7000},  # not an allowed lane
    {"video_id": "d", "url": "", "lane": "owned", "view_velocity": 6000},  # no url
    {"video_id": "e", "url": "https://y/e", "lane": "owned", "view_velocity": 5000},
]


def test_selects_top_unclipped_owned_only():
    picked = autopilot.select_unclipped(QUEUE, clipped_ids=set(), max_videos=5)
    ids = [r["video_id"] for r in picked]
    assert ids == ["a", "b", "e"]      # c=non-owned lane dropped, d=no-url dropped


def test_skips_already_clipped():
    picked = autopilot.select_unclipped(QUEUE, clipped_ids={"a"}, max_videos=5)
    assert [r["video_id"] for r in picked] == ["b", "e"]


def test_respects_max_videos():
    picked = autopilot.select_unclipped(QUEUE, clipped_ids=set(), max_videos=1)
    assert [r["video_id"] for r in picked] == ["a"]


def test_channel_for_routes_health_niche_to_phoenix_protocol():
    assert autopilot.channel_for("health-mythbusting") == "phoenix-protocol"
    assert autopilot.channel_for(None) == "clips"          # unknown → loud-fail fallback


def test_channel_slugs_all_have_a_postiz_mapping():
    # The routing key a clip carries MUST be a configured Postiz channel, or
    # distribution can't find an integration id for it.
    from ycp.config import settings
    configured = set(settings()["distribution"]["postiz"]["channels"])
    assert set(autopilot.CHANNEL_SLUGS.values()) == configured


def test_default_lane_is_owned_only():
    # owned is the only lane; the default filter admits exactly the owned rows.
    assert autopilot.DEFAULT_LANES == ("owned",)
    picked = autopilot.select_unclipped(QUEUE, set(), max_videos=5)
    assert [r["video_id"] for r in picked] == ["a", "b", "e"]


def test_stage_result_line_renders_mark():
    assert autopilot.StageResult("source", True, "5 queued").line().startswith("  ✓ source")
    assert autopilot.StageResult("clip", False, "boom").line().startswith("  ✗ clip")
