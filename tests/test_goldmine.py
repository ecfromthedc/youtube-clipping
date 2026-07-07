from ycp.goldmine import peak_windows, quote_at, _dedupe
from ycp.srt import Segment


def test_peak_windows_ranks_and_spaces():
    hm = [{"start_time": float(t), "end_time": t + 5, "value": v}
          for t, v in [(0, 0.2), (30, 1.0), (33, 0.9), (120, 0.6), (600, 0.8)]]
    w = peak_windows(hm, top=3)
    assert [round(x["peak_t"]) for x in w] == [30, 600, 120]   # by intensity; 33 dropped (≤25s from 30)
    assert w[0]["start"] == 22.0 and w[0]["end"] == 62.0       # peak−lead .. +window


def test_peak_windows_clamps_start_at_zero():
    hm = [{"start_time": 3.0, "end_time": 8.0, "value": 1.0}]
    assert peak_windows(hm)[0]["start"] == 0.0


def test_dedupe_collapses_scroll_repeats():
    assert _dedupe("software will go to go to zero") == "software will go to zero"
    assert _dedupe("the age the age of scaling") == "the age of scaling"


def test_quote_at_window():
    segs = [Segment(20, 25, "all application software"), Segment(25, 30, "will go to zero")]
    assert quote_at(segs, 22, 62) == "all application software will go to zero"
