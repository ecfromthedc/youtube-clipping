from pathlib import Path
from ycp import notes


def test_clip_id_strips_note_suffix():
    assert notes.clip_id_for(Path("ab12cd34-00 -- shows the host.mp4")) == "ab12cd34-00"
    assert notes.clip_id_for(Path("ab12cd34-00.mp4")) == "ab12cd34-00"


def test_note_from_filename():
    assert notes.note_for(Path("ab12cd34-00 -- hook is wrong.mp4")) == "hook is wrong"
    assert notes.note_for(Path("plain-00.mp4")) in ("", notes._finder_comment(Path("plain-00.mp4")))


def test_collect(tmp_path):
    (tmp_path / "a-00 -- bad framing.mp4").write_bytes(b"x")
    (tmp_path / "b-00.mp4").write_bytes(b"x")
    got = notes.collect(tmp_path)
    assert ("a-00", "bad framing", tmp_path / "a-00 -- bad framing.mp4") in got
    assert all(cid != "b-00" for cid, _, _ in got)   # un-noted clip excluded
