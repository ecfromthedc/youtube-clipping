"""Stage 2.2 — FACE-PAN REFRAME. Crop a 9:16 window that follows the speaker.

A blind center-crop takes the middle slice — it cuts off off-center speakers and keeps junk
(sidebars, b-roll). This detects the speaker's face with OpenCV across the clip and crops a
1080-wide window centered on them, hard-cutting to follow when they move. Falls back to a
static center crop when OpenCV is missing, the source is narrow, or no face is found — so the
pipeline never breaks.

`crop_x_expr` is pure + unit-tested; `face_track` and `reframe` are thin wrappers verified on
a real clip.
"""
from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

TARGET_W, TARGET_H = 1080, 1920


def _probe_dims(video: Path) -> tuple[int, int]:
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
         "stream=width,height", "-of", "csv=p=0:s=x", str(video)],
        capture_output=True, text=True, timeout=60).stdout.strip()
    try:
        w, h = (int(x) for x in out.split("x")[:2])
        return w, h
    except ValueError:
        return 0, 0


def face_track(video: Path, sample_fps: float = 3.0, min_face_frac: float = 0.09,
               max_faces: int = 2) -> tuple[list[tuple[float, float]], int]:
    """Sample the video; return ([(t_sec, face_center_x_fraction)], n_sampled) for a real speaker.
    Skips frames with more than `max_faces` (photo grids / face-heavy b-roll) and faces smaller
    than `min_face_frac` of the width (thumbnails), so it follows the speaker, not graphics.
    ([], 0) if OpenCV is unavailable (caller then centers the crop)."""
    try:
        import cv2
    except ImportError:
        return [], 0
    cap = cv2.VideoCapture(str(video))
    fps = cap.get(cv2.CAP_PROP_FPS) or 30.0
    width = cap.get(cv2.CAP_PROP_FRAME_WIDTH) or 1.0
    cascade = cv2.CascadeClassifier(cv2.data.haarcascades + "haarcascade_frontalface_default.xml")
    step = max(1, round(fps / max(sample_fps, 0.5)))
    min_px = max(48, int(min_face_frac * width))
    track: list[tuple[float, float]] = []
    sampled, i = 0, 0
    while True:
        ok, frame = cap.read()
        if not ok:
            break
        if i % step == 0:
            sampled += 1
            gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
            faces = cascade.detectMultiScale(gray, scaleFactor=1.1, minNeighbors=6,
                                             minSize=(min_px, min_px))
            if 1 <= len(faces) <= max_faces:
                x, _y, fw, _fh = max(faces, key=lambda f: int(f[2]) * int(f[3]))
                track.append((i / fps, (x + fw / 2) / width))
        i += 1
    cap.release()
    return track, sampled


def _smooth(vals: list[float], win: int = 5) -> list[float]:
    out = []
    for i in range(len(vals)):
        a, b = max(0, i - win), min(len(vals), i + win + 1)
        out.append(sum(vals[a:b]) / (b - a))
    return out


def crop_x_expr(track: list[tuple[float, float]], scaled_w: int,
                crop_w: int = TARGET_W, jump: float = 0.05) -> str | None:
    """Piecewise ffmpeg crop-x expression following the (smoothed) face track. Hard-cuts only
    when the target shifts more than `jump` of the frame width. None if the track is empty."""
    if not track:
        return None
    hi = max(0, scaled_w - crop_w)

    def to_x(frac: float) -> int:
        return int(max(0, min(hi, frac * scaled_w - crop_w / 2)))

    sm = _smooth([f for _, f in track])
    segs: list[tuple[float, int]] = []
    cur = to_x(sm[0])
    thresh = jump * scaled_w
    for (t, _), f in zip(track, sm):
        x = to_x(f)
        if abs(x - cur) > thresh:
            segs.append((t, cur))
            cur = x
    segs.append((float("inf"), cur))
    if len(segs) == 1:
        return str(segs[0][1])
    expr = str(segs[-1][1])
    for end_t, x in reversed(segs[:-1]):
        expr = f"if(lt(t,{end_t:.3f}),{x},{expr})"
    return expr


def reframe(video: Path, out_path: Path, workdir: Path, *, mode: str = "face",
            size: tuple[int, int] = (TARGET_W, TARGET_H)) -> Path:
    """Scale to target height and crop a 9:16 window — face-following (mode='face') or static
    center (mode='center', a narrow source, or no faces). Raises RuntimeError on ffmpeg fail."""
    w, h = size
    sw, sh = _probe_dims(video)
    scaled_w = round(sw * h / sh / 2) * 2 if sh else 0
    expr = None
    if mode == "face" and scaled_w > w:
        track, sampled = face_track(video)
        # Only pan when a speaker-sized face is present most of the clip — otherwise the source
        # is a graphic / b-roll / turned-away shot and the safe center crop is better.
        if sampled and len(track) >= 0.45 * sampled:
            expr = crop_x_expr(track, scaled_w, crop_w=w)
    center_vf = f"scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"
    face_vf = None if (scaled_w <= w or expr is None) else f"scale=-2:{h},crop={w}:{h}:x='{expr}':y=0"
    tmp = workdir / "reframed.mp4"
    last_err = ""
    for vf in ([face_vf, center_vf] if face_vf else [center_vf]):
        cmd = ["ffmpeg", "-y", "-i", str(video), "-vf", vf, "-c:v", "libx264",
               "-c:a", "copy", "-preset", "veryfast", "-pix_fmt", "yuv420p", str(tmp)]
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
        if proc.returncode == 0 and tmp.exists():
            out_path.parent.mkdir(parents=True, exist_ok=True)
            shutil.move(str(tmp), str(out_path))
            return out_path
        last_err = proc.stderr.strip()[-400:]
    raise RuntimeError(f"reframe failed: {last_err}")
