//! The civil-calendar arithmetic the date/time functions share:
//! Hinnant's `days_from_civil`/`civil_from_days` (integer-only,
//! proleptic Gregorian — deterministic everywhere the engine runs)
//! and the MMWR week-start helpers `EPIWEEK` uses. No `EV` knowledge:
//! every function here is a pure integer map.

/// Days since 1970-01-01 for a valid civil date; `None` when the
/// calendar rejects it (`new DateTime(y, m, d)` throwing → null).
pub(crate) fn days_from_civil_checked(y: i32, m: u32, d: u32) -> Option<i32> {
    if !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// `days_from_civil` (Howard Hinnant's civil-calendar algorithm).
pub(crate) fn days_from_civil(y: i32, m: u32, d: u32) -> i32 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = ((m + 9) % 12) as i32;
    let doy = (153 * mp + 2) / 5 + d as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `civil_from_days` — `(year, month, day)` of a day offset.
pub(crate) fn civil_from_days(z: i32) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `(year, month, day, hour, minute, second)` of an epoch-second.
pub(crate) fn civil_from_secs(secs: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days as i32);
    (
        y,
        m,
        d,
        (rem / 3600) as u32,
        (rem / 60 % 60) as u32,
        (rem % 60) as u32,
    )
}

/// Epoch-seconds of a civil instant; `None` when the date is invalid.
pub(crate) fn secs_from_civil_checked(
    y: i32,
    m: u32,
    d: u32,
    h: u32,
    mi: u32,
    s: u32,
) -> Option<i64> {
    if h > 23 || mi > 59 || s > 59 {
        return None;
    }
    days_from_civil_checked(y, m, d)
        .map(|days| i64::from(days) * 86_400 + (h * 3600 + mi * 60 + s) as i64)
}

/// `GetMMWRStart(year, fdw)`: the MMWR week-1 start nearest Jan 1 —
/// the first week holding ≥4 days of the new year. `fdw` is 0-based
/// (0=Sunday).
fn mmwr_start_year(y: i32, fdw: i32) -> i32 {
    let jan1 = days_from_civil(y, 1, 1);
    let fdoy = day_of_week(jan1) as i32;
    if fdoy <= fdw + 3 {
        jan1 + fdw - fdoy
    } else {
        jan1 + 7 - fdw - fdoy
    }
}

/// `GetMMWRStart(date, fdw)`: the MMWR start of the MMWR year the date
/// belongs to.
pub(crate) fn mmwr_start(days: i32, fdw: i32) -> i32 {
    let (y, ..) = civil_from_days(days);
    let prev = mmwr_start_year(y - 1, fdw);
    let cur = mmwr_start_year(y, fdw);
    let next = mmwr_start_year(y + 1, fdw);
    if cur - days > 0 {
        prev
    } else if next - days > 0 {
        cur
    } else {
        next
    }
}

/// Day of week, 0=Sunday.
fn day_of_week(days: i32) -> u32 {
    // 1970-01-01 was a Thursday (4).
    (days.rem_euclid(7) + 4) as u32 % 7
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
    }
}
