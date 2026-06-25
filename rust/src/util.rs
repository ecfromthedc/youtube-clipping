//! Small shared helpers.
use chrono::SecondsFormat;

/// UTC now as ISO-8601, second precision — matches the Python db.now().
pub fn utc_now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn today_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Insert thousands separators into a (possibly signed) integer string. "15000" → "15,000".
fn group_thousands(s: &str) -> String {
    let (sign, digits) = match s.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", s),
    };
    let len = digits.len();
    let mut out = String::with_capacity(len + len / 3 + sign.len());
    out.push_str(sign);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Python `f"{n:,}"` — integer with thousands separators. "15000" → "15,000".
pub fn comma(n: i64) -> String {
    group_thousands(&n.to_string())
}

/// Mirror brief.py / scoreboard.py `_money`: `$1,234` for ≥100, `$x.xx` below. Rust `{:.N}`
/// uses round-half-to-even, matching Python's format rounding.
pub fn money(x: f64) -> String {
    if x >= 100.0 {
        format!("${}", group_thousands(&format!("{x:.0}")))
    } else {
        let s = format!("{x:.2}");
        let (int_part, frac) = s.split_once('.').unwrap_or((s.as_str(), "00"));
        format!("${}.{frac}", group_thousands(int_part))
    }
}

/// Mirror `_views`: 6.1M / 431K / 9.
pub fn views(x: f64) -> String {
    if x >= 1_000_000.0 {
        format!("{:.1}M", x / 1_000_000.0)
    } else if x >= 1000.0 {
        format!("{:.0}K", x / 1000.0)
    } else {
        format!("{x:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_matches_python() {
        assert_eq!(money(0.0), "$0.00");
        assert_eq!(money(1.0), "$1.00");
        assert_eq!(money(90.27), "$90.27");
        assert_eq!(money(103.49), "$103");
        assert_eq!(money(2500.0), "$2,500");
        assert_eq!(money(15000.0), "$15,000");
    }

    #[test]
    fn views_matches_python() {
        assert_eq!(views(0.0), "0");
        assert_eq!(views(8872.0), "9K");
        assert_eq!(views(431176.0), "431K");
        assert_eq!(views(6_100_000.0), "6.1M");
    }
}
