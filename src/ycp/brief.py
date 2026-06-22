"""Weekly Double-Down Brief — the output that closes the loop.

Fully deterministic: pandas decides what to scale/kill/test and rule-based logic
writes the prose, referencing the actual creators/formats/hooks in the numbers.
No LLM — so it runs unattended on cron, is reproducible, and is unit-tested.
The brief is saved to the DB and (optionally) posted to Slack, and is meant to
become next week's sourcing spec.
"""
from __future__ import annotations

from datetime import date

import pandas as pd

from .config import settings
from .scoring import analyze


def _money(x: float) -> str:
    return f"${x:,.0f}" if x >= 100 else f"${x:,.2f}"


def _views(x: float) -> str:
    return f"{x/1_000_000:.1f}M" if x >= 1_000_000 else f"{x/1_000:.0f}K" if x >= 1000 else f"{x:.0f}"


def _table(df: pd.DataFrame, cols: list[str], limit: int) -> str:
    if df.empty:
        return "_(no combos with enough sample yet)_"
    return df.head(limit)[cols].to_markdown(index=False)


def _scale_section(scale: pd.DataFrame) -> str:
    if scale.empty:
        return ("Not enough sample yet to crown winners. Keep volume broad across "
                "creators/formats and let the next capture build signal.")
    lines = []
    for _, r in scale.head(settings()["brief"]["top_n"]).iterrows():
        lines.append(
            f"- **Make 3× more — {r['source_creator']} · {r['fmt']} · {r['hook_type']} hook**  "
            f"→ score {r['avg_score']}, {_views(r['avg_views'])} avg views over {int(r['n'])} clips "
            f"({_money(r['total_revenue'])})."
        )
    return "\n".join(lines)


def _kill_section(kill: pd.DataFrame) -> str:
    if kill.empty:
        return "Nothing is clearly dead yet — no combo is underperforming enough to cut."
    lines = []
    for _, r in kill.head(settings()["brief"]["top_n"]).iterrows():
        lines.append(
            f"- **Stop — {r['source_creator']} · {r['fmt']} · {r['hook_type']} hook**  "
            f"→ score {r['avg_score']}, only {_views(r['avg_views'])} avg views over {int(r['n'])} clips. "
            f"Reallocate those edits."
        )
    return "\n".join(lines)


def _test_section(a: dict) -> str:
    """Heuristic bets: pair the best overall hook with the best overall creator,
    and probe the length bucket adjacent to the current winner."""
    bets: list[str] = []
    top_hook = a["by_hook"].iloc[0]["hook_type"] if not a["by_hook"].empty else None
    top_creator = a["by_creator"].iloc[0]["source_creator"] if not a["by_creator"].empty else None
    top_fmt = a["by_format"].iloc[0]["fmt"] if not a["by_format"].empty else None
    if top_creator and top_hook:
        bets.append(f"- Pair the best hook (**{top_hook}**) with the best creator "
                    f"(**{top_creator}**) if you haven't already — verify the combo holds.")
    if top_fmt and top_hook:
        bets.append(f"- Push **{top_fmt}** into a second creator to see if the format "
                    f"travels or is creator-specific.")
    if not a["by_length"].empty:
        best_len = a["by_length"].iloc[0]["length_bucket"]
        bets.append(f"- Best length right now is **{best_len}** — test one bucket shorter "
                    f"to chase higher completion.")
    return "\n".join(bets) if bets else "Collect one more week of data before placing test bets."


def _money_section(scored: pd.DataFrame) -> str:
    if scored.empty:
        return "No revenue captured yet."
    whop = scored.get("whop_payout", pd.Series(dtype=float)).fillna(0).sum()
    ads = scored.get("ad_revenue", pd.Series(dtype=float)).fillna(0).sum()
    by_lane = (scored.assign(rev=scored.get("whop_payout", 0).fillna(0)
                             + scored.get("ad_revenue", 0).fillna(0))
               .groupby("lane")["rev"].sum().sort_values(ascending=False))
    by_plat = (scored.assign(rev=scored.get("whop_payout", 0).fillna(0)
                             + scored.get("ad_revenue", 0).fillna(0))
               .groupby("platform")["rev"].sum().sort_values(ascending=False))
    lane_line = ", ".join(f"{k} {_money(v)}" for k, v in by_lane.items())
    plat_line = ", ".join(f"{k} {_money(v)}" for k, v in by_plat.items())
    lead = ("Whop is carrying the revenue — keep feeding the cash engine."
            if whop >= ads else
            "Ad revenue is leading — owned channels are maturing, protect their YPP status.")
    return (f"- Whop bounties: **{_money(whop)}**  ·  Ad revenue: **{_money(ads)}**\n"
            f"- By lane: {lane_line}\n- By platform: {plat_line}\n- {lead}")


def build(df: pd.DataFrame, week_start: str | None = None) -> str:
    """Return the full Double-Down Brief as markdown (deterministic)."""
    top_n = settings()["brief"]["top_n"]
    week = week_start or date.today().isoformat()
    a = analyze(df)
    n_clips = 0 if df.empty else len(df)

    md = f"""# 📈 Double-Down Brief — week of {week}

_Generated from {n_clips} clips with metrics. Virality score 0–100; revenue in USD._

## 🟢 Scale
{_scale_section(a["scale"])}

## 🔴 Kill
{_kill_section(a["kill"])}

## 🆕 Test
{_test_section(a)}

## 💰 Where the money is
{_money_section(a["scored"])}

---

### Top combos (creator × format × hook)
{_table(a["by_combo"], ["source_creator", "fmt", "hook_type", "n", "avg_score", "avg_views", "total_revenue"], top_n)}

### By format
{_table(a["by_format"], ["fmt", "n", "avg_score", "avg_views", "total_revenue"], top_n)}

### By hook
{_table(a["by_hook"], ["hook_type", "n", "avg_score", "avg_views", "total_revenue"], top_n)}

### By length
{_table(a["by_length"], ["length_bucket", "n", "avg_score", "avg_views"], top_n)}

### By platform
{_table(a["by_platform"], ["platform", "n", "avg_score", "avg_views", "total_revenue"], len(a["by_platform"]))}

> Next step: this brief is next week's sourcing spec. Point Stage 1 at the 🟢 combos.
"""
    return md
