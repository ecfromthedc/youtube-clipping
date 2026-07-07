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


def test_sidecar_note_wins_and_strips_template(tmp_path):
    clip = tmp_path / "c5-00.mp4"
    clip.write_bytes(b"x")
    (tmp_path / "c5-00.note.txt").write_text(
        "# Why is this clip wrong? (template line, ignored)\n\nshows the interviewer not the guest\n")
    assert notes.note_for(clip) == "shows the interviewer not the guest"
    assert notes.sidecar_for(clip).name == "c5-00.note.txt"


def test_sidecar_template_only_is_empty(tmp_path):
    clip = tmp_path / "c6-00.mp4"
    clip.write_bytes(b"x")
    (tmp_path / "c6-00.note.txt").write_text("# just the template\n\n")
    assert notes.note_for(clip) == ""   # not yet noted → won't trigger refinement
