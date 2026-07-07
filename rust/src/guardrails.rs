//! Guardrails enforced IN CODE (HANDOFF §10) — parity port of `src/ycp/guardrails.py`.
//!
//! Two gates protect the whole operation; a single strike pattern can demonetize a
//! faceless network channel-wide:
//!  1. Avoid-list gate (sourcing) — runs BEFORE the score; disqualified creators and
//!     music/casino/licensed-IP titles never enter the queue.
//!  2. Publish gate (distribution) — last line before auto-posting: must be TRANSFORMED
//!     (our hook/caption, not a raw reupload) and carry no copyrighted-music signal.
//!
//! Pure + deterministic — these are load-bearing for auto-posting, so they're verifiable.

// ── Avoid-list (SOURCE-INTELLIGENCE.md §Avoid + HANDOFF §3 mega-creator ruling) ──
// Normalized substrings matched against creator name AND handle.
const AVOID_CREATORS: &[&str] = &[
    "joe rogan",
    "jre",
    "andrew tate",
    "tate",
    "fresh & fit",
    "freshandfit",
    "myrongainesx",
    "mssp",
    "matt and shane",
    "matt & shane",
    "jordan peterson",
    "andrew huberman",
    "huberman",
    "peter attia",
    "plaqueboymax",
    "lacy",
    "caedrel",
    // Mega-creators: turbo-only or AVOID as owned lanes (HANDOFF §3).
    "mrbeast",
    "mr beast",
    "ishowspeed",
    "speed",
    "kai cenat",
    "taylor swift",
];

// Title red flags → music / gambling / licensed-IP = Content-ID / DMCA risk.
const AVOID_TITLE_TERMS: &[&str] = &[
    "official music video",
    "official video",
    "lyric",
    "lyrics",
    "ft.",
    "feat.",
    "concert",
    "live performance",
    "music video",
    "casino",
    "slots",
    "gambling",
    "betting",
    "stake.com",
    "full album",
    "official audio",
];

/// A sourcing candidate (mirrors the Python dict keys creator_allowed/filter_creators read).
#[derive(Debug, Clone)]
#[allow(dead_code)] // ported for parity; wired when sourcing cuts over
pub struct Creator {
    pub name: String,
    pub url: String,
    pub handle: String,
}

/// The publish-gate view of a clip (mirrors the Python dict keys publish_allowed reads).
#[derive(Debug, Clone, Default)]
pub struct ClipGate {
    pub transformed: bool,
    pub has_music: bool,
    pub title: String,
}

/// Lowercase, drop `@`→space, then strip everything but [a-z0-9 ] (chars removed, not spaced).
fn norm(s: &str) -> String {
    s.to_lowercase()
        .replace('@', " ")
        .chars()
        .filter(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == ' ')
        .collect::<String>()
        .trim()
        .to_string()
}

/// False if the creator is on the avoid-list (gate runs BEFORE scoring).
pub fn creator_allowed(name: &str, handle: &str) -> bool {
    let blob = format!("{} {}", norm(name), norm(handle));
    !AVOID_CREATORS.iter().any(|b| blob.contains(&norm(b)))
}

/// Screen a single source video's title for music/casino/licensed-IP flags.
pub fn source_allowed(title: &str) -> (bool, String) {
    let low = title.to_lowercase();
    for term in AVOID_TITLE_TERMS {
        if low.contains(term) {
            return (false, format!("title flag: '{term}'"));
        }
    }
    (true, String::new())
}

/// Split creators into (allowed, dropped-names) by the avoid-list. Pure.
#[allow(dead_code)] // ported for parity; wired when sourcing cuts over
pub fn filter_creators(creators: &[Creator]) -> (Vec<Creator>, Vec<String>) {
    let mut allowed = Vec::new();
    let mut dropped = Vec::new();
    for c in creators {
        // `url or handle` — url if non-empty, else handle (matches Python `.get(...) or .get(...)`).
        let handle = if c.url.is_empty() { &c.handle } else { &c.url };
        if creator_allowed(&c.name, handle) {
            allowed.push(c.clone());
        } else {
            dropped.push(c.name.clone());
        }
    }
    (allowed, dropped)
}

/// Last gate before auto-posting (QC is auto). Require transformation + no music.
/// Conservative: a clip that isn't explicitly transformed is treated as NOT transformed.
pub fn publish_allowed(clip: &ClipGate) -> (bool, String) {
    if !clip.transformed {
        return (
            false,
            "not transformed (raw reupload risks channel-wide demonetization)".to_string(),
        );
    }
    if clip.has_music {
        return (
            false,
            "copyrighted-music signal — Content-ID would claim it".to_string(),
        );
    }
    source_allowed(&clip.title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creator_avoid_list_gate() {
        assert!(!creator_allowed("Joe Rogan", ""));
        assert!(!creator_allowed("", "@MrBeast"));
        assert!(!creator_allowed("iShowSpeed", "speedy"));
        assert!(creator_allowed("Some Random Debater", "@debate_daily"));
    }

    #[test]
    fn title_flags_block_music_and_gambling() {
        assert!(!source_allowed("New Single (Official Music Video)").0);
        assert_eq!(
            source_allowed("Best Slots win EVER").1,
            "title flag: 'slots'"
        );
        assert!(source_allowed("Heated debate on the economy").0);
    }

    #[test]
    fn filter_creators_splits_allowed_and_dropped() {
        let creators = vec![
            Creator {
                name: "Joe Rogan".into(),
                url: "".into(),
                handle: "jre".into(),
            },
            Creator {
                name: "Indie Debater".into(),
                url: "https://x.com/indie".into(),
                handle: "".into(),
            },
        ];
        let (allowed, dropped) = filter_creators(&creators);
        assert_eq!(allowed.len(), 1);
        assert_eq!(allowed[0].name, "Indie Debater");
        assert_eq!(dropped, vec!["Joe Rogan".to_string()]);
    }

    #[test]
    fn publish_gate_requires_transform_then_no_music_then_clean_title() {
        // not transformed → blocked first.
        let (ok, why) = publish_allowed(&ClipGate {
            transformed: false,
            has_music: false,
            title: "clean".into(),
        });
        assert!(!ok);
        assert!(why.starts_with("not transformed"));
        // transformed but music → blocked.
        let (ok, why) = publish_allowed(&ClipGate {
            transformed: true,
            has_music: true,
            title: "clean".into(),
        });
        assert!(!ok);
        assert!(why.contains("copyrighted-music"));
        // transformed, no music, dirty title → blocked by title screen.
        let (ok, _) = publish_allowed(&ClipGate {
            transformed: true,
            has_music: false,
            title: "live performance".into(),
        });
        assert!(!ok);
        // all clear → allowed.
        let (ok, why) = publish_allowed(&ClipGate {
            transformed: true,
            has_music: false,
            title: "the debate that broke the internet".into(),
        });
        assert!(ok);
        assert_eq!(why, "");
    }
}
