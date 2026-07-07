from ycp.refine import _secs, plan


def test_secs_parsing():
    assert _secs(-2) == -2.0
    assert _secs("2 earlier") == -2.0
    assert _secs("1.5 later") == 1.5
    assert _secs("3") == 3.0


def test_plan_no_provenance(tmp_path):
    from ycp import db
    dbp = tmp_path / "t.db"
    db.init_db(dbp)
    db.insert_clip({"clip_id": "x-00", "channel": "ai-frontier", "platform": "youtube",
                    "lane": "owned", "fmt": "auto-clip", "hook_type": "q", "length_sec": 30}, dbp)
    r = plan("x-00", [{"type": "crop", "value": "fix it"}], dbp)
    assert r["ok"] is False and "re-source" in r["reason"]


def test_plan_adjusts_bounds(tmp_path):
    from ycp import db
    dbp = tmp_path / "t.db"
    db.init_db(dbp)
    db.insert_clip({"clip_id": "y-00", "channel": "ai-frontier", "platform": "youtube",
                    "lane": "owned", "fmt": "auto-clip", "hook_type": "q", "length_sec": 30,
                    "source_url": "http://x", "clip_start": 100.0, "clip_end": 130.0,
                    "post_title": "old hook", "source_creator": "X"}, dbp)
    r = plan("y-00", [{"type": "start", "value": -2}, {"type": "end", "value": 1.5},
                      {"type": "hook", "value": "new hook"}], dbp)
    assert r["ok"] and r["start"] == 98.0 and r["end"] == 131.5 and r["title"] == "new hook"


def test_pin_inserts_then_recut_plans(tmp_path):
    from ycp import refine
    db = tmp_path / "t.db"
    # clip with no row at all -> pin inserts a minimal row with provenance
    r = refine.pin("zz99-00", "https://youtu.be/ABC", 100.0, 130.0,
                   creator="Naval", title="a hook", channel="ai-frontier", db_path=db)
    assert r["ok"] and r["inserted"]
    plan = refine.plan("zz99-00", [{"type": "crop", "value": "tighten"}], db_path=db)
    assert plan["ok"] and plan["start"] == 100.0 and plan["end"] == 130.0
    # pinning again updates in place (no duplicate row, no insert)
    r2 = refine.pin("zz99-00", "https://youtu.be/ABC", 105.0, 140.0, db_path=db)
    assert r2["ok"] and not r2["inserted"]
    assert refine.plan("zz99-00", [], db_path=db)["start"] == 105.0
