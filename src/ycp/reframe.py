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

from .config import ROOT

TARGET_W, TARGET_H = 1080, 1920
# Zoom-to-speaker: if the face is smaller than ZOOM_TRIGGER_FH of frame height (a wide stage
# shot), scale up so it reaches ~ZOOM_TARGET_FH, capped at ZOOM_MAX. Keeps keynote/TED speakers
# from standing tiny in the middle of the vertical frame.
ZOOM_TRIGGER_FH, ZOOM_TARGET_FH, ZOOM_MAX = 0.16, 0.26, 2.3

# YuNet DNN face detector (OpenCV's own, no extra dep) — bundled ONNX. Far better than the Haar
# cascade on profiles / angled / small / off-centre faces (the cases that framed the lamp).
# Resolve under ROOT (repo root), NOT __file__ — the package installs non-editable into
# site-packages, so a __file__-relative path would miss the bundled assets.
YUNET_MODEL = ROOT / "assets" / "models" / "face_detection_yunet_2023mar.onnx"
_YUNET_CACHE: list = []  # [detector] or [None] once resolved (lazy, cached)


def _yunet():
    """Cached cv2.FaceDetectorYN, or None if cv2/model unavailable (caller falls back to Haar)."""
    if _YUNET_CACHE:
        return _YUNET_CACHE[0]
    det = None
    try:
        import cv2
        if hasattr(cv2, "FaceDetectorYN") and YUNET_MODEL.exists():
            det = cv2.FaceDetectorYN.create(str(YUNET_MODEL), "", (320, 320), score_threshold=0.6)
    except Exception:  # noqa: BLE001  (any cv2/model issue → fall back to Haar)
        det = None
    _YUNET_CACHE.append(det)
    return det


# SFace recognition (identity-lock) — OpenCV's own, no extra dep. Follows ONE consistent face
# across host/guest cuts in a 2-cam interview, instead of framing whoever is biggest per frame.
SFACE_MODEL = ROOT / "assets" / "models" / "face_recognition_sface_2021dec.onnx"
_SFACE_CACHE: list = []
IDENTITY_SIM = 0.36   # cosine ≥ this → same person (OpenCV SFace default ~0.363)


def _sface():
    """Cached cv2.FaceRecognizerSF, or None if unavailable (then we use largest-face-per-frame)."""
    if _SFACE_CACHE:
        return _SFACE_CACHE[0]
    rec = None
    try:
        import cv2
        if hasattr(cv2, "FaceRecognizerSF") and SFACE_MODEL.exists():
            rec = cv2.FaceRecognizerSF.create(str(SFACE_MODEL), "")
    except Exception:  # noqa: BLE001
        rec = None
    _SFACE_CACHE.append(rec)
    return rec


HOSTS_DIR = ROOT / "assets" / "hosts"
_HOST_EMB_CACHE: list = []


def _host_embeddings() -> list:
    """SFace embeddings of known interviewers (one face image each in assets/hosts/). identity-lock
    excludes these so it follows the GUEST, not the host. Empty list if none/cv2 unavailable."""
    if _HOST_EMB_CACHE:
        return _HOST_EMB_CACHE[0]
    embs: list = []
    det, rec = _yunet(), _sface()
    if det is not None and rec is not None and HOSTS_DIR.exists():
        import cv2
        det2 = cv2.FaceDetectorYN.create(str(YUNET_MODEL), "", (320, 320), score_threshold=0.6)
        for img_path in sorted(HOSTS_DIR.glob("*")):
            if img_path.suffix.lower() not in (".jpg", ".jpeg", ".png"):
                continue
            frame = cv2.imread(str(img_path))
            if frame is None:
                continue
            det2.setInputSize((frame.shape[1], frame.shape[0]))
            _, faces = det2.detect(frame)
            if faces is not None and len(faces):
                f = max(faces, key=lambda r: float(r[2]) * float(r[3]))
                try:
                    embs.append(rec.feature(rec.alignCrop(frame, f.reshape(1, -1))))
                except Exception:  # noqa: BLE001
                    pass
    _HOST_EMB_CACHE.append(embs)
    return embs


def _identity_lock_on() -> bool:
    try:
        from .config import settings
        return bool(settings().get("reframe", {}).get("identity_lock", True))
    except Exception:  # noqa: BLE001
        return True


def _cosine(a, b) -> float:
    import numpy as np
    a = np.asarray(a, dtype=float).ravel()
    b = np.asarray(b, dtype=float).ravel()
    na, nb = float(np.linalg.norm(a)), float(np.linalg.norm(b))
    return float(a @ b / (na * nb)) if na and nb else 0.0


def _dominant_track(records: list[tuple[float, float, object]],
                    sim_thresh: float = IDENTITY_SIM,
                    exclude_embs: list | None = None) -> list[tuple[float, float]]:
    """records = [(t, center_x_frac, embedding)]. Cluster by face identity (cosine), return the
    appearances [(t, x_frac)] of the MOST-PRESENT identity — the person being featured. If
    `exclude_embs` is given (known interviewers/hosts), clusters matching a host are dropped so
    we follow the GUEST, not the host. Falls back to all clusters if every face is a host. Pure
    (numpy cosine), so it unit-tests without cv2."""
    clusters: list[dict] = []
    for t, cx, emb in records:
        best, bi = -1.0, -1
        for k, c in enumerate(clusters):
            s = _cosine(c["cent"], emb)
            if s > best:
                best, bi = s, k
        if bi >= 0 and best >= sim_thresh:
            clusters[bi]["apps"].append((t, cx))
        else:
            clusters.append({"cent": emb, "apps": [(t, cx)]})
    if not clusters:
        return []
    pool = clusters
    if exclude_embs:
        guests = [c for c in clusters
                  if not any(_cosine(c["cent"], h) >= sim_thresh for h in exclude_embs)]
        pool = guests or clusters       # if everyone matches a host, don't break — keep all
    return sorted(max(pool, key=lambda c: len(c["apps"]))["apps"])


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


def face_track(video: Path, sample_fps: float = 3.0, min_face_frac: float = 0.06,
               max_faces: int = 2) -> tuple[list[tuple[float, float]], int, float, float]:
    """Sample the video; return (x_track, n_sampled, median_face_height_frac, median_face_y_frac).
    x_track = [(t_sec, face_center_x_fraction)] for the featured speaker (identity-locked when SFace
    is on). The geometry (median face height + y as fractions of frame height) lets the caller zoom
    in on a speaker who's small in a wide shot. Uses YuNet (DNN) when available, else the Haar
    cascade. ([], 0, 0.0, 0.5) if OpenCV is unavailable (caller then centers the crop)."""
    try:
        import cv2
    except ImportError:
        return [], 0, 0.0, 0.5
    import statistics
    cap = cv2.VideoCapture(str(video))
    fps = cap.get(cv2.CAP_PROP_FPS) or 30.0
    width = cap.get(cv2.CAP_PROP_FRAME_WIDTH) or 1.0
    height = cap.get(cv2.CAP_PROP_FRAME_HEIGHT) or 1.0
    det = _yunet()
    rec = _sface() if (det is not None and _identity_lock_on()) else None
    cascade = None if det else cv2.CascadeClassifier(
        cv2.data.haarcascades + "haarcascade_frontalface_default.xml")
    if det is not None:
        det.setInputSize((int(width), int(height)))
    step = max(1, round(fps / max(sample_fps, 0.5)))
    min_px = max(40, int(min_face_frac * width))
    track: list[tuple[float, float]] = []            # largest-face-per-frame (no SFace)
    records: list[tuple[float, float, object]] = []  # (t, cx_frac, embedding) for identity-lock
    geoms: list[tuple[float, float]] = []            # (height_frac, y_center_frac) of kept faces
    sampled, i = 0, 0
    while True:
        ok, frame = cap.read()
        if not ok:
            break
        if i % step == 0:
            sampled += 1
            t = i / fps
            if det is not None:
                _, faces = det.detect(frame)
                rows = [f for f in (faces if faces is not None else []) if f[2] >= min_px]
            else:
                gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
                rows = [list(b) + [0] * 11 for b in cascade.detectMultiScale(
                    gray, scaleFactor=1.1, minNeighbors=6, minSize=(min_px, min_px))]
            if not (1 <= len(rows) <= max_faces):
                i += 1
                continue
            biggest = max(rows, key=lambda r: float(r[2]) * float(r[3]))
            geoms.append((float(biggest[3]) / height,
                          (float(biggest[1]) + float(biggest[3]) / 2) / height))
            if rec is not None:
                # Embed each face → identity-lock resolves the featured person after the scan.
                for f in rows:
                    cx = (float(f[0]) + float(f[2]) / 2) / width
                    try:
                        emb = rec.feature(rec.alignCrop(frame, f.reshape(1, -1)))
                        records.append((t, cx, emb))
                    except Exception:  # noqa: BLE001  (alignment can fail on edge crops)
                        track.append((t, cx))  # still usable for the largest-face fallback
            else:
                track.append((t, (float(biggest[0]) + float(biggest[2]) / 2) / width))
        i += 1
    cap.release()
    fh_med = statistics.median([h for h, _ in geoms]) if geoms else 0.0
    y_med = statistics.median([y for _, y in geoms]) if geoms else 0.5
    if rec is not None and records:
        return _dominant_track(records, exclude_embs=_host_embeddings()), sampled, fh_med, y_med
    return track, sampled, fh_med, y_med


def face_coverage(video: Path, start: float, end: float, n: int = 6) -> float:
    """Fraction of n frames sampled across [start, end] that contain a speaker-sized face.
    A cheap gate so clip selection prefers speaker-on-camera windows over slide / b-roll
    stretches (a talk that cut to a full-screen slide has ~0 coverage). 1.0 when we can't
    tell (no cv2/detector) so it never wrongly penalises a window."""
    try:
        import cv2
    except ImportError:
        return 1.0
    det = _yunet()
    if det is None:
        return 1.0
    cap = cv2.VideoCapture(str(video))
    width = cap.get(cv2.CAP_PROP_FRAME_WIDTH) or 1.0
    height = cap.get(cv2.CAP_PROP_FRAME_HEIGHT) or 1.0
    det.setInputSize((int(width), int(height)))
    min_px = max(40, int(0.06 * width))
    hits = seen = 0
    for k in range(n):
        t = start + (end - start) * (k + 0.5) / n
        cap.set(cv2.CAP_PROP_POS_MSEC, t * 1000.0)
        ok, frame = cap.read()
        if not ok:
            continue
        seen += 1
        _, faces = det.detect(frame)
        if any(f[2] >= min_px for f in (faces if faces is not None else [])):
            hits += 1
    cap.release()
    return hits / seen if seen else 1.0


def first_face_time(video: Path, start: float, end: float, max_skip: float = 4.0,
                    n: int = 9) -> float:
    """First time in [start, start+max_skip] where a speaker-sized face appears, so the caller can
    trim a speaker-less opening and start the clip ON the speaker. Returns `start` if a face is
    already there at the start, or if none appears within max_skip (then the coverage gate decides)."""
    try:
        import cv2
    except ImportError:
        return start
    det = _yunet()
    if det is None:
        return start
    cap = cv2.VideoCapture(str(video))
    width = cap.get(cv2.CAP_PROP_FRAME_WIDTH) or 1.0
    height = cap.get(cv2.CAP_PROP_FRAME_HEIGHT) or 1.0
    det.setInputSize((int(width), int(height)))
    min_px = max(40, int(0.06 * width))
    hi = min(end, start + max_skip)
    for k in range(n):
        t = start + (hi - start) * k / max(1, n - 1)
        cap.set(cv2.CAP_PROP_POS_MSEC, t * 1000.0)
        ok, frame = cap.read()
        if not ok:
            continue
        _, faces = det.detect(frame)
        if any(f[2] >= min_px for f in (faces if faces is not None else [])):
            cap.release()
            return t                      # first frame the speaker is on camera
    cap.release()
    return start


def face_band(video: Path, t: float = 0.3) -> tuple[float, float] | None:
    """Normalized (y_top, y_bottom) of the largest face at time `t`, or None if no face/cv2.
    Used to place the hook in clear space instead of over the speaker's face."""
    try:
        import cv2
    except ImportError:
        return None
    det = _yunet()
    if det is None:
        return None
    cap = cv2.VideoCapture(str(video))
    if t:
        cap.set(cv2.CAP_PROP_POS_MSEC, t * 1000.0)
    ok, frame = cap.read()
    cap.release()
    if not ok or frame is None:
        return None
    h, w = frame.shape[:2]
    det.setInputSize((w, h))
    _, faces = det.detect(frame)
    if faces is None or not len(faces):
        return None
    f = max(faces, key=lambda r: float(r[2]) * float(r[3]))   # largest face
    y0 = max(0.0, float(f[1]) / h)
    y1 = min(1.0, (float(f[1]) + float(f[3])) / h)
    return (y0, y1)


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
    import statistics
    w, h = size
    sw, sh = _probe_dims(video)
    face_vf = None
    if mode == "face" and sh:
        track, sampled, fh_med, y_med = face_track(video)
        # Zoom when the speaker's face is small (a wide stage shot) so they fill the vertical
        # frame instead of standing tiny in the middle. We scale UP by `zoom` then crop, which
        # makes the face occupy ~ZOOM_TARGET_FH of the output height.
        zoom = min(ZOOM_MAX, ZOOM_TARGET_FH / fh_med) if fh_med and fh_med < ZOOM_TRIGGER_FH else 1.0
        zh = round(h * zoom / 2) * 2
        zw = round(sw * zh / sh / 2) * 2
        y = 0
        if zoom > 1.05 and zw >= w:                  # head slightly above centre, body below
            y = int(max(0, min(zh - h, y_med * zh - h * 0.42)))
        if zw > w and track:
            # Pan to follow the speaker (at the zoomed scale when zooming). Sparse detections →
            # lock to the median position rather than snapping to dead-centre (which frames the lamp).
            if sampled and len(track) >= 0.25 * sampled:
                expr = crop_x_expr(track, zw, crop_w=w)
            else:
                frac = statistics.median(f for _, f in track)
                expr = str(int(max(0, min(zw - w, frac * zw - w / 2))))
            if expr is not None:
                scale = f"{zw}:{zh}" if zoom > 1.05 else f"-2:{h}"
                face_vf = f"scale={scale},crop={w}:{h}:x='{expr}':y={y}"
    center_vf = f"scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"
    tmp = workdir / "reframed.mp4"
    last_err = ""
    for vf in ([face_vf, center_vf] if face_vf else [center_vf]):
        cmd = ["ffmpeg", "-y", "-i", str(video), "-vf", vf, "-c:v", "libx264", "-crf", "18",
               "-c:a", "copy", "-preset", "veryfast", "-pix_fmt", "yuv420p", str(tmp)]
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
        if proc.returncode == 0 and tmp.exists():
            out_path.parent.mkdir(parents=True, exist_ok=True)
            shutil.move(str(tmp), str(out_path))
            return out_path
        last_err = proc.stderr.strip()[-400:]
    raise RuntimeError(f"reframe failed: {last_err}")
