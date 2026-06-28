"""Caption chunking (pure) + a real Pillow overlay render."""
from __future__ import annotations

from ycp import captions
from ycp.srt import Segment


def test_split_words_even_distribution():
    words = captions.split_words(Segment(0.0, 3.0, "one two three"))
    assert [w.text for w in words] == ["one", "two", "three"]
    assert words[0].start == 0.0
    assert abs(words[-1].end - 3.0) < 0.01
    assert all(words[i].end <= words[i + 1].start + 1e-6 for i in range(len(words) - 1))


def test_build_chunks_caps_words_and_is_non_overlapping():
    chunks = captions.build_chunks([Segment(0.0, 7.0, "a b c d e f g")], max_words=3)
    assert [len(c.words) for c in chunks] == [3, 3, 1]
    assert all(c.end > c.start for c in chunks)
    assert all(chunks[i].end <= chunks[i + 1].start + 1e-6 for i in range(len(chunks) - 1))


def test_build_chunks_enforces_min_dwell():
    chunks = captions.build_chunks([Segment(0.0, 0.1, "hi there")], max_words=3, min_dwell=0.5)
    assert chunks and chunks[0].end - chunks[0].start >= 0.5


def test_render_overlay_hook_only_when_no_captions(tmp_path):
    # RULE #1 defer path: empty chunks → the hook still renders, no second subtitle track.
    n = captions.render_overlay([], duration=1.0, out_dir=tmp_path / "f", title="big hook", fps=10)
    assert n == 10 and (tmp_path / "f" / "00000.png").exists()


def test_case_helper_lowercases():
    assert captions._case("Dr Mike SAYS", "lower") == "dr mike says"
    assert captions._case("loud", "upper") == "LOUD"


def test_caption_cfg_reflects_settings():
    # reads creative knobs from settings.yaml; values are tunable, so just sanity-check ranges.
    cfg = captions._caption_cfg()
    assert cfg["case"] in ("lower", "upper")
    assert 1.0 <= cfg["hook_hold_sec"] <= 12.0
    assert 0.0 < cfg["size_pct"] < 0.20


def test_render_overlay_writes_transparent_frames(tmp_path):
    chunks = captions.build_chunks([Segment(0.0, 1.0, "hello world now")])
    n = captions.render_overlay(chunks, duration=1.0, out_dir=tmp_path / "f",
                                title="BIG HOOK", fps=10)
    assert n == 10
    first = tmp_path / "f" / "00000.png"
    assert first.exists()
    from PIL import Image
    im = Image.open(first)
    assert im.mode == "RGBA" and im.size == captions.SIZE
