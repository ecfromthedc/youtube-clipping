"""Analyze an IG reel with Gemini — study its FORMAT so we can clone it in Tides & Ships."""
import json, sys, os, time
sys.path.insert(0, "/Users/ericcromartie/Documents/Development/youtube-clipping")
os.chdir("/Users/ericcromartie/Documents/Development/youtube-clipping")

from ycp.config import env, settings

VIDEO = "/tmp/ig_reel.mp4"
MODEL = "gemini-2.5-flash"

PROMPT = """You are an expert short-form video producer analyzing a reference reel for format reverse-engineering.
Watch this reel carefully and break down EVERY creative and editorial choice that makes it work.

Return ONLY a JSON object with this exact schema:
{
  "format_name": "<short name for this format, e.g. 'countdown ranking', 'listicle tier list'>",
  "hook_analysis": {
    "first_3_seconds": "<exactly what happens in the first 3 seconds — visual + audio + on-screen text>",
    "why_it_hooks": "<1-2 lines: the psychological trigger>",
    "hook_score_1to10": <number>
  },
  "structure": [
    "<step 1 of the format, in order, e.g. '0-3s: bold question on screen + cold open'>",
    "<step 2...>"
  ],
  "visual_style": {
    "colors": "<dominant palette + accent colors>",
    "text_style": "<caption style: font weight, case, position, animation>",
    "transitions": "<cut style: hard cuts, zooms, etc>",
    "pacing": "<fast/slow, avg shot length>",
    "on_screen_graphics": "<lists, numbers, rankings, tier labels — describe exactly>"
  },
  "ranking_mechanic": {
    "present": <true/false>,
    "how_it_works": "<if present: exactly how the ranking/tiering is shown — what's ranked, how ranks are revealed, what the visual treatment is>",
    "revelation_pattern": "<if present: does it count down? count up? reveal best last? reveal best first?>"
  },
  "audio": {
    "music": "<genre/energy/tempo if recognizable>",
    "voiceover": "<AI or human? tone? pace?>",
    "sfx": "<sound effects used and when>"
  },
  "engagement_triggers": ["<thing 1 that drives rewatch/comment>", "<thing 2>"],
  "clone_recipe": {
    "template_steps": ["<concrete step 1 to reproduce this in an editor>", "<step 2>"],
    "data_needed": "<what list/ranking data a user would need to plug in>",
    "estimated_edit_time_min": <number>
  }
}

Be specific and observational. Reference exact moments (in seconds) when describing choices."""

from google import genai
from google.genai import types

client = genai.Client(api_key=env()["gemini_api_key"])
print("Uploading reel to Gemini Files API...", file=sys.stderr)
f = client.files.upload(file=VIDEO)
waited = 0
while getattr(f.state, "name", "") != "ACTIVE":
    if getattr(f.state, "name", "") == "FAILED" or waited >= 180:
        print(f"Upload failed/state={f.state}", file=sys.stderr); sys.exit(1)
    time.sleep(5); waited += 5
    f = client.files.get(name=f.name)
print(f"ACTIVE in {waited}s. Analyzing format...", file=sys.stderr)

resp = client.models.generate_content(
    model=MODEL,
    contents=[f, PROMPT],
    config=types.GenerateContentConfig(response_mime_type="application/json", temperature=0.3),
)
try: client.files.delete(name=f.name)
except: pass

print(json.dumps(json.loads(resp.text), indent=2))
