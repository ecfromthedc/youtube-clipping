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
