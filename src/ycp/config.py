"""Configuration loading: settings.yaml + .env, with sane defaults.

Nothing here requires secrets. Missing .env values resolve to None and are
only enforced by the command that actually needs them (e.g. `ycp qc` needs Slack).
"""
from __future__ import annotations

import os
from functools import lru_cache
from pathlib import Path
from typing import Any

import yaml
from dotenv import load_dotenv

def _find_root() -> Path:
    """Locate the project root (which holds config/ and data/).

    Order: $YCP_HOME → nearest ancestor of CWD containing config/settings.yaml →
    package-relative fallback. This decouples config/data from where the package
    is installed, so a non-editable install (copied into site-packages) still
    finds the real project files when run from the repo (operators + cron cd here).
    """
    env_home = os.getenv("YCP_HOME")
    if env_home:
        return Path(env_home)
    cwd = Path.cwd()
    for d in (cwd, *cwd.parents):
        if (d / "config" / "settings.yaml").exists():
            return d
    return Path(__file__).resolve().parents[2]


ROOT = _find_root()
DATA_DIR = ROOT / "data"
DB_PATH = DATA_DIR / "clips.db"
SETTINGS_PATH = ROOT / "config" / "settings.yaml"


@lru_cache(maxsize=1)
def settings() -> dict[str, Any]:
    """Load tunable knobs from config/settings.yaml (cached)."""
    with SETTINGS_PATH.open() as fh:
        return yaml.safe_load(fh)


@lru_cache(maxsize=1)
def env() -> dict[str, str | None]:
    """Load .env once and return the credentials we care about."""
    load_dotenv(ROOT / ".env")
    return {
        "youtube_api_key": os.getenv("YOUTUBE_API_KEY") or None,
        "whop_api_key": os.getenv("WHOP_API_KEY") or None,
        "slack_bot_token": os.getenv("SLACK_BOT_TOKEN") or None,
        "slack_app_token": os.getenv("SLACK_APP_TOKEN") or None,
        "slack_qc_channel": os.getenv("SLACK_QC_CHANNEL") or None,
    }


def ensure_data_dir() -> None:
    DATA_DIR.mkdir(parents=True, exist_ok=True)
