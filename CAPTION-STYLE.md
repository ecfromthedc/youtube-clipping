# CAPTION-STYLE.md — the look we cut captions to

The visual target for burned-in captions across owned channels. Reference photo:
`reference/caption-style.png` — @hotseatYT (Hot Seat). *(photo to be added — see PR/commit.)*

## The look (what the photo shows)
- Heavy display font — official cut is **TikTok Sans Overlay** (wght 650, opsz 36,
  vendored in `assets/tiktok-font/`; Arial Black / Impact as fallbacks). Photo is
  all-caps; **we run lowercase** — we borrow the photo's position + color, not its case.
- **Golden yellow** fill `#FFDE00` (≈ code `ACTIVE = (255, 222, 0)`).
- **Fat black outline** on every glyph — legible over any footage.
- **2 lines max**, centered, sitting in the **lower third** (≈65–70% down).
- Channel **`@handle` watermark** top-left, small, low-opacity white.

## Maps to the `captions:` knobs (settings.yaml → src/ycp/captions.py)
- `case: lower` — **keep lowercase** unless the new channel style says otherwise.
  Photo is uppercase, but we only adopt its **position + color**, not its case.
- `size_pct: ~9` — captions sized **down one notch** from the photo's ~10%.
- font / stroke / color / 2-line lower-third layout already match the code.

## Hook design — the counter-piece (top third)
The hook is the framing question; captions are the spoken line. Same **font + fat
black stroke** as captions (shared DNA) so they read as one family, but flipped to
create hierarchy instead of competing:
- **Position:** top third, centered, 2 lines max. (Captions own the bottom.)
- **Case:** lowercase — brand-consistent with captions.
- **Color:** white fill, with the **single operative word in caption-yellow
  `#FFDE00`** — that hot-word is the through-line the eye links top → bottom.
- **Size:** full size — roughly **equal to (or a touch above) caption height**.
  The captions are the ones sized down a notch, not the hook.
- **Hold:** stays up ≥ `hook_hold_sec` (7s default). ✅ existing knob.

## Open tuning calls (don't silently change — decide per channel)
1. **All-yellow vs. word-by-word highlight.** Photo is fully yellow; the renderer
   highlights the *active* word yellow and leaves the rest white. Pick per channel:
   static-yellow caption blocks (this photo) or animated word highlight.
2. **`@handle` watermark.** Visible in the photo, **not burned in by the pipeline
   yet.** Add as a per-channel overlay if we want it on every clip.

<!-- ponytail: reference doc, not code. captions.py constants are the source of truth;
this captures the *target* + the per-channel divergences so the renderer can be tuned to it. -->
