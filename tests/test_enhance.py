"""Enhancement builder tests — pure filter/command construction, no ffmpeg run."""
from __future__ import annotations

from pathlib import Path

from ycp import enhance


def test_title_filter_uses_textfile():
    f = enhance.title_filter("title.txt")
    assert f.startswith("drawtext=")
    assert "textfile='title.txt'" in f       # escaping-proof: text read from a file
    assert "fontsize=" in f


def test_cta_filter_is_timed_and_filebacked():
    f = enhance.cta_filter("cta.txt", 2.0, 7.0)
    assert "textfile='cta.txt'" in f
    assert "enable='between(t,2.0,7.0)'" in f


def test_hook_cta_vf_combines_and_empties():
    both = enhance.hook_cta_vf("title.txt", "cta.txt", (2, 7))
    assert both.count("drawtext=") == 2
    assert enhance.hook_cta_vf(None, None, (2, 7)) == ""


def test_vstack_cmd_well_formed():
    cmd = enhance.vstack_cmd(Path("clip.mp4"), Path("game.mp4"), Path("out.mp4"))
    assert cmd[0] == "ffmpeg"
    j = " ".join(cmd)
    assert "vstack=inputs=2" in j
    assert "-stream_loop" in cmd and "-shortest" in cmd
    assert "0:a?" in j  # keep clip audio, tolerate missing


def test_pick_title_prefers_question_and_truncates():
    assert enhance.pick_title("We did stuff. Why is this the biggest mistake? Then more.").endswith("?")
    long = enhance.pick_title("word " * 20)
    assert long.endswith("…")
    assert enhance.pick_title("") == ""
