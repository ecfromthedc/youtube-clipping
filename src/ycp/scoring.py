"""Scoring engine — the deterministic core of the closed loop.

Turns per-clip metrics into a 0–100 virality score, then rolls up by dimension
(source creator, format, hook, length, platform) so we can see which *combos*
over-index. Pure pandas: fully reproducible and unit-tested. No LLM here — the
LLM only writes prose in brief.py; the numbers are always deterministic.
"""
from __future__ import annotations

import numpy as np
import pandas as pd

from .config import settings

ACTION_DIMS = ["source_creator", "fmt", "hook_type"]  # the actionable combo


def _minmax(s: pd.Series) -> pd.Series:
    """Scale to 0..1. Constant or empty series -> 0.5 (neutral, no signal)."""
    if s.empty:
        return s
    lo, hi = s.min(), s.max()
    if not np.isfinite(lo) or not np.isfinite(hi) or hi == lo:
        return pd.Series(0.5, index=s.index)
    return (s - lo) / (hi - lo)


def length_bucket(seconds: float | int | None) -> str:
    buckets = settings()["scoring"]["length_buckets"]
    if seconds is None or (isinstance(seconds, float) and np.isnan(seconds)):
        return "unknown"
    for lo, hi in zip(buckets, buckets[1:]):
        if lo <= seconds < hi:
            return f"{lo}-{hi}s"
    return f"{buckets[-1]}s+"


def add_derived(df: pd.DataFrame) -> pd.DataFrame:
    """Add revenue_per_1k and length_bucket. Returns a new frame (immutable in)."""
    if df.empty:
        return df.copy()
    out = df.copy()
    views = out["views"].clip(lower=0).fillna(0)
    revenue = out.get("ad_revenue", 0).fillna(0) + out.get("whop_payout", 0).fillna(0)
    with np.errstate(divide="ignore", invalid="ignore"):
        out["revenue_per_1k"] = np.where(views > 0, revenue / (views / 1000.0), 0.0)
    out["retention_pct"] = out.get("retention_pct", pd.Series(index=out.index)).fillna(0)
    out["length_bucket"] = out["length_sec"].apply(length_bucket)
    return out


def compute_scores(df: pd.DataFrame, weights: dict | None = None) -> pd.DataFrame:
    """Add a 0–100 `virality_score` blending views, retention, revenue/1k."""
    if df.empty:
        return df.copy()
    w = weights or settings()["scoring"]["weights"]
    out = add_derived(df)
    n_views = _minmax(np.log1p(out["views"].clip(lower=0).fillna(0)))
    n_ret = _minmax(out["retention_pct"].clip(lower=0))
    n_rev = _minmax(out["revenue_per_1k"].clip(lower=0))
    score = (
        w["views_7d"] * n_views
        + w["retention"] * n_ret
        + w["revenue_per_1k"] * n_rev
    )
    out["virality_score"] = (score * 100).round(1)
    return out


def rollup(df: pd.DataFrame, dims: list[str], min_sample: int | None = None) -> pd.DataFrame:
    """Group scored clips by `dims`; return mean score/views, total revenue, n.

    Combos with fewer than `min_sample` clips are dropped (not enough signal).
    """
    if df.empty or "virality_score" not in df:
        return pd.DataFrame(columns=[*dims, "n", "avg_score", "avg_views", "total_revenue"])
    min_sample = settings()["scoring"]["min_sample"] if min_sample is None else min_sample
    df = df.copy()
    df["_revenue"] = df.get("ad_revenue", 0).fillna(0) + df.get("whop_payout", 0).fillna(0)
    g = (
        df.groupby(dims, dropna=False)
        .agg(
            n=("clip_id", "count"),
            avg_score=("virality_score", "mean"),
            avg_views=("views", "mean"),
            total_revenue=("_revenue", "sum"),
        )
        .reset_index()
    )
    g = g[g["n"] >= min_sample]
    g["avg_score"] = g["avg_score"].round(1)
    g["avg_views"] = g["avg_views"].round(0)
    g["total_revenue"] = g["total_revenue"].round(2)
    return g.sort_values("avg_score", ascending=False).reset_index(drop=True)


def scale_and_kill(
    rolled: pd.DataFrame, scale_q: float | None = None, kill_q: float | None = None
) -> tuple[pd.DataFrame, pd.DataFrame]:
    """Split a rollup into top-quantile (scale) and bottom-quantile (kill)."""
    cfg = settings()["scoring"]
    scale_q = cfg["scale_quantile"] if scale_q is None else scale_q
    kill_q = cfg["kill_quantile"] if kill_q is None else kill_q
    if rolled.empty:
        return rolled, rolled
    hi = rolled["avg_score"].quantile(scale_q)
    lo = rolled["avg_score"].quantile(kill_q)
    scale = rolled[rolled["avg_score"] >= hi].reset_index(drop=True)
    kill = rolled[rolled["avg_score"] <= lo].sort_values("avg_score").reset_index(drop=True)
    return scale, kill


def analyze(df: pd.DataFrame) -> dict:
    """One call -> everything the brief needs: scored clips + rollups + verdicts."""
    scored = compute_scores(df)
    combo = rollup(scored, ACTION_DIMS)
    scale, kill = scale_and_kill(combo)
    return {
        "scored": scored,
        "by_combo": combo,
        "by_creator": rollup(scored, ["source_creator"]),
        "by_format": rollup(scored, ["fmt"]),
        "by_hook": rollup(scored, ["hook_type"]),
        "by_length": rollup(scored, ["length_bucket"]),
        "by_platform": rollup(scored, ["platform"]),
        "scale": scale,
        "kill": kill,
    }
