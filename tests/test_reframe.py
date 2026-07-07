"""Face-pan reframe — pure crop-x expression logic (no OpenCV / ffmpeg)."""
from __future__ import annotations

from ycp import reframe


def test_empty_track_returns_none():
    assert reframe.crop_x_expr([], 3413) is None


def test_static_track_centers_and_clamps():
    track = [(i * 0.3, 0.5) for i in range(40)]
    assert reframe.crop_x_expr(track, 3413, crop_w=1080) == str(int(0.5 * 3413 - 540))


def test_track_clamps_to_frame_edges():
    left = [(i * 0.3, 0.0) for i in range(40)]
    right = [(i * 0.3, 1.0) for i in range(40)]
    assert reframe.crop_x_expr(left, 3413) == "0"
    assert reframe.crop_x_expr(right, 3413) == str(3413 - 1080)


def test_sustained_pan_builds_conditional_expression():
    track = [(i * 0.3, 0.2) for i in range(20)] + [(6 + i * 0.3, 0.85) for i in range(20)]
    expr = reframe.crop_x_expr(track, 3413)
    assert expr.startswith("if(lt(t")  # hard-cut pan between two positions


def test_yunet_detector_bundled():
    """The YuNet ONNX ships with the repo and loads (no extra pip dep)."""
    from ycp import reframe
    assert reframe.YUNET_MODEL.exists(), "YuNet model missing from assets/models/"
    # _yunet() returns a detector when cv2+model are present (None only if cv2 lacks FaceDetectorYN)
    import cv2
    if hasattr(cv2, "FaceDetectorYN"):
        assert reframe._yunet() is not None


def test_dominant_track_picks_most_present_identity():
    """_dominant_track clusters faces by embedding and returns the most-present person's track."""
    from ycp.reframe import _dominant_track
    guest = [1.0, 0.0, 0.0]   # guest embedding (appears 4x, left side)
    host = [0.0, 1.0, 0.0]    # host embedding (appears 2x, right side)
    records = [
        (0.0, 0.30, guest), (1.0, 0.31, host), (2.0, 0.29, guest),
        (3.0, 0.70, host), (4.0, 0.30, guest), (5.0, 0.32, guest),
    ]
    track = _dominant_track(records)
    assert len(track) == 4                       # the 4 guest appearances, not the 2 host
    assert all(x < 0.5 for _, x in track)        # guest is the left-side cluster
    assert track == sorted(track)                # returned time-ordered


def test_sface_model_bundled():
    from ycp import reframe
    assert reframe.SFACE_MODEL.exists(), "SFace model missing from assets/models/"


# -- active-speaker detection: speech intervals + mouth-motion -----------------

def test_is_speech_covers_and_excludes_boundaries():
    """_is_speech is true inside/on a segment's [start,end], false outside or with no segments."""
    from ycp.reframe import _is_speech
    from ycp.srt import Segment
    segs = [Segment(1.0, 2.0, "a"), Segment(5.0, 6.0, "b")]
    assert _is_speech(1.5, segs)          # inside first
    assert _is_speech(1.0, segs)          # inclusive start boundary
    assert _is_speech(2.0, segs)          # inclusive end boundary
    assert not _is_speech(0.5, segs)      # before any segment
    assert not _is_speech(3.0, segs)      # in the silent gap
    assert not _is_speech(6.5, segs)      # after the last segment
    assert not _is_speech(1.5, [])        # no segments → never speech
    assert not _is_speech(1.5, None)      # None → never speech


def test_mouth_motion_zero_on_identical_positive_on_diff():
    """_mouth_motion ~0 for identical crops, >0 for different, 0.0 on missing/shape-mismatch."""
    import numpy as np
    from ycp.reframe import _mouth_motion
    a = np.zeros((10, 8), dtype=np.uint8)
    b = a.copy()
    assert _mouth_motion(a, b) == 0.0                      # identical → no motion
    c = np.full((10, 8), 40, dtype=np.uint8)
    assert _mouth_motion(a, c) == 40.0                     # uniform 40-level diff
    assert _mouth_motion(None, b) == 0.0                   # no previous crop → 0
    assert _mouth_motion(a, None) == 0.0                   # no current crop → 0
    assert _mouth_motion(a, np.zeros((5, 8), np.uint8)) == 0.0  # shape mismatch → 0


def test_nearest_mouth_matches_position_within_tolerance():
    """_nearest_mouth returns the closest previous crop by center-x, or None past tolerance."""
    from ycp.reframe import _nearest_mouth
    prev = [(0.30, "left"), (0.72, "right")]
    assert _nearest_mouth(prev, 0.31) == "left"      # closest to the left face
    assert _nearest_mouth(prev, 0.70) == "right"     # closest to the right face
    assert _nearest_mouth(prev, 0.50) is None        # both too far (tol 0.08)
    assert _nearest_mouth([], 0.5) is None            # nothing to match


def test_speaking_track_beats_raw_frame_count():
    """The SPEAKING face wins over a quieter face that appears MORE often. Face A (right side)
    is on screen 4x but its frames never overlap speech; face B (left side) appears only 2x but
    both frames coincide with a speech interval AND show mouth motion → B is selected."""
    from ycp.reframe import _speaking_track
    from ycp.srt import Segment
    a = [1.0, 0.0, 0.0]   # frequent-but-silent face (right side, x≈0.8)
    b = [0.0, 1.0, 0.0]   # infrequent-but-speaking face (left side, x≈0.2)
    # (t, cx, emb, mouth_motion)
    records = [
        (0.0, 0.80, a, 0.0), (1.0, 0.81, a, 0.0),
        (2.0, 0.20, b, 9.0), (3.0, 0.79, a, 0.0),
        (4.0, 0.21, b, 8.0), (5.0, 0.80, a, 0.0),
    ]
    segments = [Segment(1.8, 4.2, "B is talking here")]  # covers only B's frames (t=2,4)
    track = _speaking_track(records, segments)
    assert len(track) == 2                       # B appears twice — the speaking cluster
    assert all(x < 0.5 for _, x in track)        # B is the left-side face
    assert track == sorted(track)                # time-ordered


def test_speaking_track_falls_back_to_frame_count_without_motion():
    """With no speech-overlapping motion anywhere, _speaking_track degrades to most-frames-wins."""
    from ycp.reframe import _speaking_track
    a = [1.0, 0.0, 0.0]   # appears 4x, right side
    b = [0.0, 1.0, 0.0]   # appears 2x, left side
    records = [
        (0.0, 0.80, a, 0.0), (1.0, 0.81, a, 0.0),
        (2.0, 0.20, b, 0.0), (3.0, 0.79, a, 0.0),
        (4.0, 0.21, b, 0.0), (5.0, 0.80, a, 0.0),
    ]
    track = _speaking_track(records, segments=[])
    assert len(track) == 4                       # falls back to the most-present face (A)
    assert all(x > 0.5 for _, x in track)        # A is the right-side cluster
