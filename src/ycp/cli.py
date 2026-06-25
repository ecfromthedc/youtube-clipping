"""`ycp` command-line entrypoint — one subcommand per pipeline stage.

  ycp init                      create the database
  ycp demo                      seed demo data + print a Double-Down Brief (no creds)
  ycp source                    Stage 1: build today's ranked source queue (yt-dlp)
  ycp qc-post                   Stage 3: dispatch pending clips for review (slack|telegram|local)
  ycp qc-listen                 Stage 3: run the approval listener for the active channel
  ycp capture                   Stage 5: snapshot public views
  ycp brief                     Stage 5: generate the weekly brief (+ --post-slack)
  ycp clip <url>                Stage 2: hybrid yt-dlp+whisper+ffmpeg vertical clips
  ycp scoreboard                Race to $15K — the gamified game state (+ --demo)
  ycp autopilot                 chain the daily loop end-to-end (+ --skip-source/--no-clip)
"""
from __future__ import annotations

import argparse
import sys

from . import brief as brief_mod
from . import capture as capture_mod
from . import db
from . import mock as mock_mod
from . import scoreboard as scoreboard_mod
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
    from . import qc
    r = qc.dispatch_pending()
    if "dispatched" in r:
        print(f"✓ dispatched {r['dispatched']} clips for review via {r['channel']}")
    else:
        print(f"✓ auto-QC: {r.get('approved', 0)} approved, "
              f"{r.get('rejected', 0)} rejected (guardrails)")
    return 0


def _cmd_qc_listen(_: argparse.Namespace) -> int:
    from . import qc
    qc.collect()
    return 0


def _cmd_qc_decide(args: argparse.Namespace) -> int:
    from . import qc
    qc.decide(args.clip_id, args.decision, reviewer="cli")
    print(f"✓ {args.decision} {args.clip_id}")
    return 0


def _cmd_capture(args: argparse.Namespace) -> int:
    n = capture_mod.capture_public()
    print(f"✓ captured public views for {n} clips")
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
                           gameplay=Path(args.gameplay) if args.gameplay else None,
                           angle=args.angle, captions_on=not args.no_captions,
                           window_sec=int(args.window * 60) if args.window else None)
    if not created:
        print("✗ no clips produced (check the URL / yt-dlp / whisper output)")
        return 1
    print(f"✓ produced {len(created)} clips → data/clips/ (status: pending_qc)")
    for c in created:
        print(f"  · {c['clip_id']}  {c['len']}s  score {c['score']}  “{c['preview']}…”")
    print("\nNext: `ycp qc-post` to send them to Slack for approval.")
    return 0


def _cmd_autopilot(args: argparse.Namespace) -> int:
    from . import autopilot as autopilot_mod
    results = autopilot_mod.run(
        max_videos=args.max_videos,
        skip_source=args.skip_source,
        do_clip=not args.no_clip,
        hook_cta=not args.no_hook,
    )
    failures = [r for r in results if not r.ok]
    return 1 if failures else 0


def _cmd_scoreboard(args: argparse.Namespace) -> int:
    if args.demo:
        demo_db = ROOT / "data" / "demo.db"
        if not demo_db.exists():
            mock_mod.seed(demo_db)
        df = db.clips_with_latest_metrics(demo_db)
    else:
        df = db.clips_with_latest_metrics()
    md = scoreboard_mod.build(df)
    out = ROOT / "SCOREBOARD.md"
    out.write_text(md)
    print(md)
    print(f"\n✓ scoreboard written to {out}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="ycp", description="YouTube clipping closed-loop ops")
    sub = p.add_subparsers(dest="cmd", required=True)
    sub.add_parser("init", help="create the database").set_defaults(fn=_cmd_init)
    sub.add_parser("demo", help="seed demo data + print a brief (no creds)").set_defaults(fn=_cmd_demo)
    sub.add_parser("source", help="build today's ranked source queue").set_defaults(fn=_cmd_source)
    sub.add_parser("qc-post", help="post pending clips to Slack").set_defaults(fn=_cmd_qc_post)
    sub.add_parser("qc-listen", help="run the approval listener for the active channel (blocks)").set_defaults(fn=_cmd_qc_listen)
    qa = sub.add_parser("qc-approve", help="approve a clip by id (local/manual review)")
    qa.add_argument("clip_id")
    qa.set_defaults(fn=_cmd_qc_decide, decision="approve")
    qr = sub.add_parser("qc-reject", help="reject a clip by id")
    qr.add_argument("clip_id")
    qr.set_defaults(fn=_cmd_qc_decide, decision="reject")
    cap = sub.add_parser("capture", help="snapshot public views")
    cap.set_defaults(fn=_cmd_capture)
    br = sub.add_parser("brief", help="generate the weekly Double-Down Brief")
    br.add_argument("--post-slack", action="store_true", help="also post to the QC channel")
    br.set_defaults(fn=_cmd_brief)
    cl = sub.add_parser("clip", help="hybrid pipeline: url -> vertical captioned clips")
    cl.add_argument("url", help="source video URL (YouTube etc.)")
    cl.add_argument("--max", type=int, default=6, help="max clips to produce (default 6)")
    cl.add_argument("--lane", default="owned", choices=["owned"])
    cl.add_argument("--creator", default="unknown", help="source creator label")
    cl.add_argument("--channel", default="clips", help="target posting channel label")
    cl.add_argument("--hook-cta", action="store_true", help="burn hook title + CTA banner")
    cl.add_argument("--angle", default="", help="hook angle: debate|agitation|finance (tunes the hook agent)")
    cl.add_argument("--title", help="explicit hook title (else hook agent writes one)")
    cl.add_argument("--cta", default="Subscribe for more", help="CTA banner text")
    cl.add_argument("--no-captions", action="store_true",
                    help="skip our word-by-word captions (defer to a source that already has them)")
    cl.add_argument("--gameplay", help="path to a gameplay loop to split-screen under clips")
    cl.add_argument("--window", type=float, metavar="MIN",
                    help="only process the first MIN minutes of the source (bounds long podcasts)")
    cl.set_defaults(fn=_cmd_clip)
    sb = sub.add_parser("scoreboard", help="Race to $15K — the gamified game state")
    sb.add_argument("--demo", action="store_true", help="render from demo data")
    sb.set_defaults(fn=_cmd_scoreboard)
    ap = sub.add_parser("autopilot",
                        help="chain the daily loop: source→clip→qc→capture→brief→scoreboard")
    ap.add_argument("--max-videos", type=int, default=5,
                    help="max new source videos to clip this run (default 5)")
    ap.add_argument("--skip-source", action="store_true",
                    help="reuse the existing DB queue instead of re-fetching (fast)")
    ap.add_argument("--no-clip", action="store_true",
                    help="run the chain but skip the (slow) clip stage")
    ap.add_argument("--no-hook", action="store_true",
                    help="don't burn the hook title + CTA overlay")
    ap.set_defaults(fn=_cmd_autopilot)
    return p


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv if argv is not None else sys.argv[1:])
    return args.fn(args)


if __name__ == "__main__":
    raise SystemExit(main())
