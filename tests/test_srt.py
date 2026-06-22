"""SRT parse / slice / render tests — pure timing logic."""
from __future__ import annotations

from ycp.srt import Segment, parse_srt, slice_and_shift, to_srt

SAMPLE = """1
00:00:00,000 --> 00:00:02,500
Hello there

2
00:00:02,500 --> 00:00:05,000
this is a test

3
00:00:30,000 --> 00:00:33,000
much later
"""


def test_parse_srt():
    segs = parse_srt(SAMPLE)
    assert len(segs) == 3
    assert segs[0] == Segment(0.0, 2.5, "Hello there")
    assert segs[2].start == 30.0


def test_slice_and_shift_retimes_to_zero():
    segs = parse_srt(SAMPLE)
    sliced = slice_and_shift(segs, 2.5, 5.0)
    assert len(sliced) == 1
    assert sliced[0].start == 0.0          # shifted so window begins at 0
    assert sliced[0].end == 2.5
    assert sliced[0].text == "this is a test"


def test_roundtrip():
    segs = parse_srt(SAMPLE)
    reparsed = parse_srt(to_srt(segs))
    assert [s.text for s in reparsed] == [s.text for s in segs]
