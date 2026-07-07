//! Stage 2.5 — HOOK OPTIMIZER. Parity port of `src/ycp/hooks.py`.
//!
//! The hook is the highest-leverage lever on a clip's virality, so generation routes to a
//! strong model via **DeepSeek** (key from `DEEPSEEK_API_KEY`); scoring + selection are pure
//! and deterministic. With no key it falls back to the transcript heuristic so the pipeline
//! never hard-breaks. GUARDRAIL (HANDOFF §10): `looks_safe` is the last-line code check that
//! drops any hook naming a protected-group target before it can be used.
use std::collections::HashSet;
use std::path::Path;

use anyhow::Result;
use serde_json::Value;

use crate::enhance::pick_title;
use crate::{config, util};

const DEFAULT_MODEL: &str = "deepseek-chat";
const DEEPSEEK_URL: &str = "https://api.deepseek.com/chat/completions";

// Minimal fallback if config/hook-playbook.md is missing — keeps the agent functional.
const FALLBACK_PLAYBOOK: &str = "You are a world-class short-form viral hook writer for faceless YouTube/TikTok clip channels. Write the on-screen TITLE hook that stops the scroll in the first second. Speed to value (the hook IS the value), tension in the first 5 words, open a curiosity gap, be specific, MAX 10 words, no emojis/hashtags/quotes, never clickbait you can't pay off. ALWAYS write the hook entirely in lowercase. Use punctuation to cue the payoff — a trailing colon to tease what's coming (e.g. \"when your friend doesn't know what's coming:\") and correct apostrophes. The hook MUST cue the specific thing that happens in THIS clip (no generic hooks). Use the 5 hook types — Contrarian, Labeling, Curiosity Gap, Reframe, Pattern Interrupt — pick the types most likely to succeed for THIS clip. Respond ONLY with JSON: {\"hooks\": [{\"text\": \"...\", \"type\": \"Curiosity Gap\", \"fit\": 0.0}]}.";

// Curiosity / stakes / specificity signals that correlate with stop-scroll hooks.
const CURIOSITY: &[&str] = &[
    "why", "how", "what", "secret", "nobody", "actually", "truth", "really", "reason", "behind",
    "before", "until", "happens",
];
const STAKES: &[&str] = &[
    "never",
    "stop",
    "mistake",
    "wrong",
    "worst",
    "lost",
    "ruined",
    "destroyed",
    "exposed",
    "caught",
    "regret",
    "warning",
    "fired",
    "broke",
    "scam",
    "lie",
    "lying",
    "trap",
];
const PERSONAL: &[&str] = &["you", "your", "you're", "youre"];
const FINANCE_TERMS: &[&str] = &["money", "broke", "rich", "debt", "cash", "$"];
const DEBATE_TERMS: &[&str] = &["vs", "destroys", "owns", "wrong", "fight"];

// Last-line safety net: hooks naming a protected class as the target get dropped.
const UNSAFE_TERMS: &[&str] = &[
    "retard", "retarded", "tranny", "faggot", "fag", "n-word", "nigger", "kike", "spic", "chink",
    "groomer",
];

/// A raw hook candidate from the model, normalized to {text, type, fit}.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub text: String,
    pub typ: String,
    pub fit: f64,
}

/// The hook agent's answer — the winning candidate's text + its style (so the loop can learn
/// which hook STYLE won), or the heuristic fallback marked `heuristic`.
#[derive(Debug, Clone, PartialEq)]
pub struct Hook {
    pub text: String,
    pub typ: String,
}

fn count_in(wset: &HashSet<&str>, terms: &[&str]) -> usize {
    terms.iter().filter(|t| wset.contains(*t)).count()
}

/// Deterministic 'how stop-scroll is this title?' score. Higher = better. Pure.
pub fn score_hook(hook: &str, angle: &str) -> f64 {
    let h = hook.trim();
    if h.is_empty() {
        return 0.0;
    }
    let lower = h.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    let n = words.len();
    let wset: HashSet<&str> = words.iter().copied().collect();
    let mut score = 1.0_f64;
    if h.ends_with('?') {
        score += 0.8;
    }
    score += 0.7 * count_in(&wset, CURIOSITY) as f64;
    score += 0.6 * count_in(&wset, STAKES) as f64;
    if PERSONAL.iter().any(|t| wset.contains(t)) {
        score += 0.5;
    }
    // ponytail: ASCII digits only; Python str.isdigit also matches Unicode digit chars, which
    // realistic hooks never use. Widen if a non-ASCII-digit hook ever appears.
    if h.chars().any(|c| c.is_ascii_digit()) {
        score += 0.5;
    }
    // Length: punchy 3-10 words is the sweet spot; taper outside it.
    if (3..=10).contains(&n) {
        score += 1.0;
    } else {
        score -= (((n as f64) - 6.0).abs() / 5.0).min(1.5);
    }
    if angle == "finance" && FINANCE_TERMS.iter().any(|t| wset.contains(t)) {
        score += 0.4;
    }
    if (angle == "debate" || angle == "agitation") && DEBATE_TERMS.iter().any(|t| wset.contains(t))
    {
        score += 0.4;
    }
    util::round_to(score, 3)
}

/// Last-line guardrail: reject hooks containing a slur / protected-group target.
pub fn looks_safe(hook: &str) -> bool {
    let low = hook.to_lowercase();
    !UNSAFE_TERMS.iter().any(|bad| low.contains(bad))
}

/// Python `str(x)` for the JSON values text/type can hold (DeepSeek sends strings; the rest are
/// defensive). Mirrors `str(raw.get(...))`.
fn py_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "None".to_string(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// Python `float(raw.get("fit", 0.5))` clamped to [0,1]; bad/missing → 0.5.
fn parse_fit(v: Option<&Value>) -> f64 {
    let raw = match v {
        None => 0.5,
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.5),
        Some(Value::String(s)) => s.trim().parse::<f64>().unwrap_or(0.5),
        Some(Value::Bool(b)) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Some(_) => 0.5, // null/array/object → Python TypeError → default
    };
    raw.clamp(0.0, 1.0)
}

/// Normalize one raw hook into a Candidate. Accepts a dict (preferred) or a bare string.
/// Returns None if there's no usable text. Mirrors `_coerce_candidate`.
fn coerce_candidate(raw: &Value) -> Option<Candidate> {
    match raw {
        Value::String(s) => {
            let text = s.trim().to_string();
            (!text.is_empty()).then(|| Candidate {
                text,
                typ: String::new(),
                fit: 0.5,
            })
        }
        Value::Object(map) => {
            let text = map.get("text").map(py_str).unwrap_or_default();
            let text = text.trim().to_string();
            if text.is_empty() {
                return None;
            }
            Some(Candidate {
                text,
                typ: map.get("type").map(py_str).unwrap_or_default(),
                fit: parse_fit(map.get("fit")),
            })
        }
        _ => None,
    }
}

/// Blend the agent's context-fit likelihood (primary) with the deterministic stop-scroll
/// heuristic (backstop), plus a nudge toward learned-winner types. Pure. Mirrors `_combined_score`.
fn combined_score(c: &Candidate, angle: &str, prefer_types: &[String]) -> f64 {
    let heuristic = (score_hook(&c.text, angle) / 5.0).min(1.0);
    let mut base = 0.6 * c.fit + 0.4 * heuristic;
    if !prefer_types.is_empty() && prefer_types.iter().any(|t| t == &c.typ) {
        base += 0.1;
    }
    base
}

fn trim_words(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let joined = words
        .iter()
        .take(max_words)
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
    if words.len() > max_words {
        format!("{joined}…")
    } else {
        joined
    }
}

fn final_type(typ: &str) -> String {
    if typ.is_empty() {
        "uncategorized".to_string()
    } else {
        typ.to_string()
    }
}

/// Pick the highest combined-score candidate (FIRST on ties, matching Python `max`). Pure.
fn select_best(
    candidates: &[Candidate],
    angle: &str,
    max_words: usize,
    prefer_types: &[String],
) -> Option<Hook> {
    let first = candidates.first()?;
    let mut best_c = first;
    let mut best_s = combined_score(first, angle, prefer_types);
    for c in &candidates[1..] {
        let s = combined_score(c, angle, prefer_types);
        if s > best_s {
            // strictly greater → keep the FIRST max (Python `max` parity)
            best_s = s;
            best_c = c;
        }
    }
    Some(Hook {
        text: trim_words(&best_c.text, max_words),
        typ: final_type(&best_c.typ),
    })
}

/// Best hook per distinct type, top-k by score (stable: ties keep model order). Pure.
fn select_variants(
    cands: &[Candidate],
    angle: &str,
    k: usize,
    max_words: usize,
    prefer_types: &[String],
    moment: &str,
) -> Vec<Hook> {
    if cands.is_empty() {
        let fb = pick_title(moment, max_words);
        return vec![Hook {
            text: if looks_safe(&fb) { fb } else { String::new() },
            typ: "heuristic".to_string(),
        }];
    }
    let mut indexed: Vec<(&Candidate, f64)> = cands
        .iter()
        .map(|c| (c, combined_score(c, angle, prefer_types)))
        .collect();
    // Descending score; sort_by is stable so equal scores keep model order (Python sorted reverse).
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<Hook> = Vec::new();
    for (c, _) in indexed {
        let typ = final_type(&c.typ);
        if seen.insert(typ.clone()) {
            out.push(Hook {
                text: trim_words(&c.text, max_words),
                typ,
            });
            if out.len() == k {
                break;
            }
        }
    }
    out
}

fn playbook(root: &Path) -> String {
    std::fs::read_to_string(root.join("config").join("hook-playbook.md"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| FALLBACK_PLAYBOOK.to_string())
}

fn resolve_model(root: &Path) -> String {
    config::load_settings(root)
        .ok()
        .and_then(|s| s["hooks"]["model"].as_str().map(str::to_string))
        .unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

/// POST to DeepSeek and return the raw `hooks` array. Errors bubble up; caller maps to [].
fn call_deepseek(
    key: &str,
    model: &str,
    system: &str,
    prompt: &str,
    timeout: u64,
) -> Result<Vec<Value>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()?;
    let resp = client
        .post(DEEPSEEK_URL)
        .header("Authorization", format!("Bearer {key}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": prompt},
            ],
            "response_format": {"type": "json_object"},
            "temperature": 1.3,    // DeepSeek's recommended range for creative writing
            "max_tokens": 1024,
        }))
        .send()?
        .error_for_status()?;
    let body: Value = resp.json()?;
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("deepseek: no message content"))?;
    let parsed: Value = serde_json::from_str(content)?;
    Ok(parsed
        .get("hooks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

/// Ask DeepSeek for N hook candidates with type + self-rated fit. [] on any failure (no key,
/// network error, bad JSON) so the caller falls back to the heuristic. Mirrors `generate_candidates`.
pub fn generate_candidates(
    root: &Path,
    moment: &str,
    n: usize,
    angle: &str,
    model: Option<&str>,
    timeout: u64,
    prefer_types: &[String],
) -> Vec<Candidate> {
    let key = match config::env_var(root, "DEEPSEEK_API_KEY") {
        Some(k) => k,
        None => return vec![],
    };
    let model = model
        .map(str::to_string)
        .unwrap_or_else(|| resolve_model(root));
    let angle_line = if angle.is_empty() {
        String::new()
    } else {
        format!("This clip's angle: {angle}.\n")
    };
    let prefer_line = if prefer_types.is_empty() {
        String::new()
    } else {
        format!(
            "These hook types are currently WINNING for this channel: {}. Lean toward them when they fit this clip.\n",
            prefer_types.join(", ")
        )
    };
    let trimmed: String = moment.trim().chars().take(1500).collect();
    let prompt = format!(
        "{angle_line}{prefer_line}Transcript of the clip moment:\n\"\"\"{trimmed}\"\"\"\n\nPick the hook types most likely to succeed for THIS clip, then write {n} distinct hook titles as JSON (each with text, type, and a fit score)."
    );
    match call_deepseek(&key, &model, &playbook(root), &prompt, timeout) {
        Ok(raw) => raw.iter().filter_map(coerce_candidate).collect(),
        Err(_) => vec![],
    }
}

/// The hook agent's answer as {text, type} — the winning candidate, else the heuristic.
/// Always usable + safe. Mirrors `best`.
pub fn best(
    root: &Path,
    moment: &str,
    angle: &str,
    n: usize,
    max_words: usize,
    model: Option<&str>,
    prefer_types: &[String],
) -> Hook {
    let candidates: Vec<Candidate> =
        generate_candidates(root, moment, n, angle, model, 30, prefer_types)
            .into_iter()
            .filter(|c| looks_safe(&c.text))
            .collect();
    if let Some(h) = select_best(&candidates, angle, max_words, prefer_types) {
        return h;
    }
    let fallback = pick_title(moment, max_words);
    Hook {
        text: if looks_safe(&fallback) {
            fallback
        } else {
            String::new()
        },
        typ: "heuristic".to_string(),
    }
}

/// Up to K hook variants in DIFFERENT styles for A/B testing a hero clip. Degrades to the
/// heuristic when the model can't produce diverse angles. Mirrors `variants`.
pub fn variants(
    root: &Path,
    moment: &str,
    angle: &str,
    k: usize,
    max_words: usize,
    model: Option<&str>,
    prefer_types: &[String],
) -> Vec<Hook> {
    let cands: Vec<Candidate> = generate_candidates(
        root,
        moment,
        std::cmp::max(k * 2, 6),
        angle,
        model,
        30,
        prefer_types,
    )
    .into_iter()
    .filter(|c| looks_safe(&c.text))
    .collect();
    select_variants(&cands, angle, k, max_words, prefer_types, moment)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(text: &str, typ: &str, fit: f64) -> Candidate {
        Candidate {
            text: text.to_string(),
            typ: typ.to_string(),
            fit,
        }
    }

    // ── score_hook (mirrors test_hooks.py) ──────────────────────────────────
    #[test]
    fn score_rewards_curiosity_and_stakes_over_bland() {
        assert!(
            score_hook("Why nobody tells you this money mistake", "")
                > score_hook("A video about some financial topics today", "")
        );
    }

    #[test]
    fn score_rewards_punchy_length() {
        assert!(
            score_hook("This always happens before a crash", "")
                > score_hook(
                    "Here is a very long winded title that simply will not stop going on and on",
                    ""
                )
        );
    }

    #[test]
    fn finance_angle_bonus() {
        assert!(
            score_hook("Watch this before you go broke", "finance")
                > score_hook("Watch this before you act", "")
        );
    }

    #[test]
    fn looks_safe_blocks_slurs_allows_opinion() {
        assert!(looks_safe("This economic policy is a complete scam"));
        assert!(!looks_safe("these people are groomers"));
    }

    // ── selection (mirrors the monkeypatched best/variants tests) ────────────
    #[test]
    fn best_falls_back_when_no_candidates() {
        let h = select_best(&[], "", 10, &[]);
        assert!(h.is_none());
        // best() itself wraps the heuristic — exercised via select_variants empty path too.
    }

    #[test]
    fn picks_highest_scoring_candidate() {
        let cands = vec![
            cand("a bland line here", "Reframe", 0.3),
            cand("Why this money mistake ruins you", "Curiosity Gap", 0.9),
        ];
        let h = select_best(&cands, "finance", 10, &[]).unwrap();
        assert_eq!(h.text, "Why this money mistake ruins you");
    }

    #[test]
    fn context_fit_drives_selection() {
        let cands = vec![
            cand("Why nobody tells you this", "Curiosity Gap", 0.2),
            cand("Buy a business not a job", "Contrarian", 0.95),
        ];
        let h = select_best(&cands, "finance", 10, &[]).unwrap();
        assert_eq!(h.text, "Buy a business not a job");
    }

    #[test]
    fn best_returns_text_and_type() {
        let cands = vec![cand(
            "the cardio pace that adds years:",
            "Curiosity Gap",
            0.9,
        )];
        let h = select_best(&cands, "", 10, &[]).unwrap();
        assert_eq!(h.text, "the cardio pace that adds years:");
        assert_eq!(h.typ, "Curiosity Gap");
    }

    #[test]
    fn prefer_types_nudge_breaks_the_tie() {
        let cands = vec![
            cand("identical style line", "Reframe", 0.72),
            cand("identical style line", "Pattern Interrupt", 0.70),
        ];
        let h = select_best(&cands, "", 10, &["Pattern Interrupt".to_string()]).unwrap();
        assert_eq!(h.typ, "Pattern Interrupt");
    }

    #[test]
    fn variants_returns_distinct_styles() {
        let cands = vec![
            cand("contrarian take here", "Contrarian", 0.9),
            cand("another contrarian one", "Contrarian", 0.8),
            cand("curiosity gap hook", "Curiosity Gap", 0.85),
            cand("pattern interrupt line", "Pattern Interrupt", 0.7),
        ];
        let v = select_variants(&cands, "", 3, 10, &[], "moment");
        let types: HashSet<&str> = v.iter().map(|h| h.typ.as_str()).collect();
        assert_eq!(v.len(), 3);
        assert_eq!(types.len(), 3); // 3 DIFFERENT angles
    }

    #[test]
    fn variants_degrades_to_heuristic() {
        let v = select_variants(
            &[],
            "",
            3,
            10,
            &[],
            "Why does nobody talk about this? It matters.",
        );
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].typ, "heuristic");
        assert!(!v[0].text.is_empty());
    }

    // ── coerce (mirrors test_coerce_accepts_string_and_dict) ─────────────────
    #[test]
    fn coerce_accepts_string_and_dict() {
        assert_eq!(
            coerce_candidate(&Value::String("Hook text".into()))
                .unwrap()
                .text,
            "Hook text"
        );
        let d = serde_json::json!({"text": "X", "type": "Reframe", "fit": "0.8"});
        assert_eq!(coerce_candidate(&d).unwrap().fit, 0.8);
        assert!(coerce_candidate(&serde_json::json!({"text": ""})).is_none());
        let bad = serde_json::json!({"text": "Y", "fit": "bad"});
        assert_eq!(coerce_candidate(&bad).unwrap().fit, 0.5); // bad fit → default
    }

    #[test]
    fn uncategorized_when_type_blank() {
        let cands = vec![cand("a bare string hook", "", 0.5)];
        let h = select_best(&cands, "", 10, &[]).unwrap();
        assert_eq!(h.typ, "uncategorized");
    }
}
