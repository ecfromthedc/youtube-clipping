"""Deterministic demo data so the closed loop produces a real brief with zero creds.

Engineered so the winners/losers are obvious — that's the point: run `ycp demo`
and confirm the brief's 🟢 Scale / 🔴 Kill sections name the combos you'd expect.
No randomness (reproducible): per-clip variation comes from the index.
"""
from __future__ import annotations

from pathlib import Path

from . import db

# (creator, format, hook, lane, quality 0..1) — quality drives views/retention/$$
COMBOS = [
    ("Flagrant",      "debate-moment", "question",         "whop",  0.95),  # clear winner
    ("ModernWisdom",  "story-payoff",  "cliffhanger",      "owned", 0.80),
    ("Flagrant",      "list",          "bold-claim",       "whop",  0.60),
    ("ModernWisdom",  "reaction",      "pattern-interrupt","owned", 0.45),
    ("RandomVlogger", "reaction",      "pattern-interrupt","owned", 0.12),  # clear loser
]
PLATFORMS = ["youtube", "tiktok", "instagram"]
LENGTHS = [18, 28, 33, 41, 52]
PER_COMBO = 6  # > min_sample(4) so every combo surfaces in rollups


def seed(db_path: Path | None = None) -> int:
    db.init_db(db_path)
    n = 0
    for ci, (creator, fmt, hook, lane, q) in enumerate(COMBOS):
        for k in range(PER_COMBO):
            clip_id = f"demo-{ci}-{k:02d}"
            platform = PLATFORMS[k % len(PLATFORMS)]
            length = LENGTHS[k % len(LENGTHS)]
            # deterministic spread around the combo's quality
            jitter = ((k * 37) % 11 - 5) / 100.0          # -0.05 .. +0.05
            quality = max(0.02, min(1.0, q + jitter))
            views = int(2_000 + quality**2 * 480_000)      # 2K .. ~480K
            retention = round(20 + quality * 65, 1)        # 20% .. 85%
            whop = round(views / 1000 * 2.0, 2) if lane == "whop" else 0.0
            ad_rev = round(views / 1000 * 0.04, 2) if lane == "owned" else 0.0

            db.insert_clip({
                "clip_id": clip_id,
                "source_creator": creator,
                "channel": f"{creator}-clips",
                "platform": platform,
                "lane": lane,
                "fmt": fmt,
                "hook_type": hook,
                "length_sec": length,
                "status": "posted",
                "post_url": f"https://example.com/{clip_id}",
            }, db_path)
            db.insert_metric({
                "clip_id": clip_id,
                "views": views,
                "retention_pct": retention,
                "rpm": round(ad_rev / (views / 1000), 3) if ad_rev else None,
                "ad_revenue": ad_rev,
                "whop_payout": whop,
            }, db_path)
            n += 1
    return n
