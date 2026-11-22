//! Times as the hub writes them (RFC 3339, from Go's time.Time), read with the stdlib.

use std::time::{SystemTime, UNIX_EPOCH};

/// 2026-11-22T10:00:00.123-04:00 → seconds since the Unix epoch.
pub fn parse(ts: &str) -> Option<i64> {
    let num = |r: std::ops::Range<usize>| ts.get(r)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, s) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let rest = ts
        .get(19..)?
        .trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let offset = match rest.as_bytes().first()? {
        b'Z' | b'z' => 0,
        sign @ (b'+' | b'-') => {
            let off = (rest.get(1..3)?.parse::<i64>().ok()? * 60 + rest.get(4..6)?.parse::<i64>().ok()?) * 60;
            if *sign == b'-' { -off } else { off }
        }
        _ => return None,
    };
    Some(days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + s - offset)
}

/// Days from 1970-01-01 to a date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 2026-11-20T08:30:00Z → 2026-11-20 08:30.
// ponytail: shows the hub's clock as it comes, without converting it to this machine's zone.
pub fn short(ts: &str) -> String {
    ts.get(..16).unwrap_or(ts).replace('T', " ")
}

/// How long from start to end (or to now, while it runs): 42s, 3m 05s.
pub fn took(start: &str, end: Option<&str>) -> Option<String> {
    let end = match end {
        Some(e) => parse(e)?,
        None => now(),
    };
    let s = (end - parse(start)?).max(0);
    Some(if s < 60 {
        format!("{s}s")
    } else {
        format!("{}m {:02}s", s / 60, s % 60)
    })
}

#[cfg(test)]
mod tests {
    use super::{parse, short, took};

    #[test]
    fn reads_rfc3339() {
        assert_eq!(parse("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse("2026-11-22T10:00:00Z"), Some(1_795_341_600));
        assert_eq!(parse("2026-11-22T06:00:00.123456-04:00"), Some(1_795_341_600));
        assert_eq!(parse("2026-11-22"), None);
    }

    #[test]
    fn says_how_long() {
        assert_eq!(
            took("2026-11-22T10:00:00Z", Some("2026-11-22T10:00:42Z")).unwrap(),
            "42s"
        );
        assert_eq!(
            took("2026-11-22T10:00:00Z", Some("2026-11-22T10:03:05Z")).unwrap(),
            "3m 05s"
        );
        assert_eq!(short("2026-11-20T08:30:00Z"), "2026-11-20 08:30");
    }
}
