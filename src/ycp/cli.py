"""`ycp` command-line entrypoint — one subcommand per pipeline stage.

  ycp init                      create the database
  ycp demo                      seed demo data + print a Double-Down Brief (no creds)
  ycp source                    Stage 1: build today's ranked source queue (yt-dlp)
  ycp qc-post                   Stage 3: post pending clips to Slack for approval
  ycp qc-listen                 Stage 3: run the Slack reaction listener (blocks)
  ycp capture                   Stage 5: snapshot public views (+ --whop-csv FILE)
  ycp brief                     Stage 5: generate the weekly brief (+ --post-slack)
  ycp clip <url>                Stage 2: hybrid yt-dlp+whisper+ffmpeg vertical clips
"""
from __future__ import annotations

import argparse
import sys

from . import brief as brief_mod
from . import capture as capture_mod
from . import db
from . import mock as mock_mod
from . import sourcing as sourcing_mod
from .config import ROOT


def _cmd_init(_: argparse.Namespace) -> int:
    db.init_db()
    print("✓ database ready at data/clips.db")
    return 0


def _cmd_demo(args: argparse.Namespace) -> int:
    demo_db = ROOT / "data" / "demo.db"
    if demo_db.exists():
        demo_db.unlink()
    n = mock_mod.seed(demo_db)
    df = db.clips_with_latest_metrics(demo_db)
    md = brief_mod.build(df, week_start="DEMO")
    print(f"✓ seeded {n} demo clips\n")
    print(md)
    out = ROOT / "data" / "demo-brief.md"
    out.write_text(md)
    print(f"\n✓ brief written to {out}")
    return 0


def _cmd_source(args: argparse.Namespace) -> int:
    queue = sourcing_mod.run()
    md = sourcing_mod.render_queue_md(queue)
    out = ROOT / "data" / "source-queue.md"
    out.write_text(md)
    print(md)
    print(f"\n✓ {len(queue)} videos queued · written to {out}")
    return 0


def _cmd_qc_post(_: argparse.Namespace) -> int:
    from . import slack_qc
    count = slack_qc.post_pending()
    print(f"✓ posted {count} clips to Slack QC")
    return 0


def _cmd_qc_listen(_: argparse.Namespace) -> int:
    from . import slack_qc
    slack_qc.run_listener()
    return 0


def _cmd_capture(args: argparse.Namespace) -> int:
    n = capture_mod.capture_public()
    print(f"✓ captured public views for {n} clips")
    if args.whop_csv:
        m = capture_mod.import_whop_csv(args.whop_csv)
        print(f"✓ imported {m} Whop payouts from {args.whop_csv}")
    return 0


def _cmd_brief(args: argparse.Namespace) -> int:
    df = db.clips_with_latest_metrics()
    md = brief_mod.build(df)
    db.save_brief(week_start=__import__("datetime").date.today().isoformat(), content=md)
    out = ROOT / "data" / "latest-brief.md"
    out.write_text(md)
    print(md)
    if args.post_slack:
        from . import slack_qc
        slack_qc.post_brief(md)
        print("\n✓ posted to Slack")
    print(f"\n✓ brief saved to DB + {out}")
    return 0


def _cmd_clip(args: argparse.Namespace) -> int:
    from pathlib import Path

    from . import clip as clip_mod
    created = clip_mod.run(args.url, max_clips=args.max, lane=args.lane,
                           source_creator=args.creator, channel=args.channel,
                           hook_cta=args.hook_cta, title=args.title, cta=args.cta,
                           gameplay=Path(args.gameplay) if args.gameplay else None)
    if not created:
        print("✗ no clips produced (check the URL / yt-dlp / whisper output)")
        return 1
    print(f"✓ produced {len(created)} clips → data/clips/ (status: pending_qc)")
    for c in created:
        print(f"  · {c['clip_id']}  {c['len']}s  score {c['score']}  “{c['preview']}…”")
    print("\nNext: `ycp qc-post` to send them to Slack for approval.")
    return 0


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="ycp", description="YouTube clipping closed-loop ops")
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("init", help="create the database").set_defaults(fn=_cmd_init)
    sub.add_parser("demo", help="seed demo data + print a brief (no creds)").set_defaults(fn=_cmd_demo)
    sub.add_parser("source", help="build today's ranked source queue").set_defaults(fn=_cmd_source)
    sub.add_parser("qc-post", help="post pending clips to Slack").set_defaults(fn=_cmd_qc_post)
    sub.add_parser("qc-listen", help="run Slack approval listener (blocks)").set_defaults(fn=_cmd_qc_listen)
    cap = sub.add_parser("capture", help="snapshot views / import Whop payouts")
    cap.add_argument("--whop-csv", help="path to a Whop payout CSV export")
    cap.set_defaults(fn=_cmd_capture)
    br = sub.add_parser("brief", help="generate the weekly Double-Down Brief")
    br.add_argument("--post-slack", action="store_true", help="also post to the QC channel")
    br.set_defaults(fn=_cmd_brief)
    cl = sub.add_parser("clip", help="hybrid pipeline: url -> vertical captioned clips")
    cl.add_argument("url", help="source video URL (YouTube etc.)")
    cl.add_argument("--max", type=int, default=6, help="max clips to produce (default 6)")
    cl.add_argument("--lane", default="whop", choices=["whop", "owned"])
    cl.add_argument("--creator", default="unknown", help="source creator label")
    cl.add_argument("--channel", default="clips", help="target posting channel label")
    cl.add_argument("--hook-cta", action="store_true", help="burn hook title + CTA banner")
    cl.add_argument("--title", help="explicit hook title (else auto-picked from transcript)")
    cl.add_argument("--cta", default="Subscribe for more", help="CTA banner text")
    cl.add_argument("--gameplay", help="path to a gameplay loop to split-screen under clips")
    cl.set_defaults(fn=_cmd_clip)
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv if argv is not None else sys.argv[1:])
    return args.fn(args)


if __name__ == "__main__":
    raise SystemExit(main())
