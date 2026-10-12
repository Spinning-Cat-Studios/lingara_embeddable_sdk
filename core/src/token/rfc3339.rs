//! RFC 3339 timestamps, as the server writes `expires_at`, to Unix seconds.
//! Just enough of the grammar for a token's expiry: no leap seconds, a
//! fraction dropped, `Z` or a `±HH:MM` offset. The crate takes no date
//! library for one subtraction.

/// `2026-10-01T12:00:00Z` → seconds since the Unix epoch, or `None`.
pub(crate) fn to_unix(text: &str) -> Option<i64> {
    let [year, month, day, hour, minute, second] = fields(text)?;
    let rest = text.get(19..)?;
    let zone = match rest.strip_prefix('.') {
        Some(frac) => frac.trim_start_matches(|c: char| c.is_ascii_digit()),
        None => rest,
    };
    let offset = offset_secs(zone)?;
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second - offset)
}

/// `YYYY-MM-DDTHH:MM:SS`'s six fields, when the shape and every range hold.
fn fields(text: &str) -> Option<[i64; 6]> {
    let b = text.as_bytes();
    let separators = [(4, b'-'), (7, b'-'), (13, b':'), (16, b':')];
    if b.len() < 20 || !matches!(b[10], b'T' | b't' | b' ') || separators.iter().any(|&(i, c)| b[i] != c) {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<i64> { text.get(from..to)?.parse::<u32>().ok().map(i64::from) };
    let f = [num(0, 4)?, num(5, 7)?, num(8, 10)?, num(11, 13)?, num(14, 16)?, num(17, 19)?];
    let ranges = [(1, 1, 12), (2, 1, 31), (3, 0, 23), (4, 0, 59), (5, 0, 60)];
    ranges.iter().all(|&(i, lo, hi)| (lo..=hi).contains(&f[i])).then_some(f)
}

/// `Z` is zero; `+05:30` is east of UTC, so it is subtracted.
fn offset_secs(zone: &str) -> Option<i64> {
    if zone.eq_ignore_ascii_case("z") {
        return Some(0);
    }
    let sign = match zone.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let (hours, minutes) = zone.get(1..)?.split_once(':')?;
    if hours.len() != 2 || minutes.len() != 2 {
        return None;
    }
    Some(sign * (hours.parse::<i64>().ok()? * 3_600 + minutes.parse::<i64>().ok()? * 60))
}

/// Days from 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Seconds since the epoch as an RFC 3339 UTC string: the inverse, for
/// tests and the clamp's log line.
pub(crate) fn from_unix(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rem / 3_600, rem % 3_600 / 60, rem % 60)
}
