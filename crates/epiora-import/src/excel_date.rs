//! Excel serial-date conversion (v0.3 spec §6.2, plan Task 4): days
//! since 1970-01-01 in the 1900 and 1904 systems. The 1900 system
//! inherits Excel's leap-year bug: serial 60 (the phantom
//! 1900-02-29) is `None` and serials below it are shifted by one.
//! A serial with a non-zero time part is `None`.

/// 1900-01-01 in days since 1970-01-01: serials < 60 use it directly.
const DAYS_1900_EARLY: i64 = 25568;
/// The post-bug offset: serials ≥ 61 are `serial - 25569` because the
/// phantom serial 60 already accounts for one day.
const DAYS_1900: i64 = 25569;
/// 1904-01-01 in days since 1970-01-01.
const DAYS_1904: i64 = 24107;

/// Converts an Excel serial to days since 1970-01-01. `None` when the
/// serial has a time part (`serial.fract() != 0`, which also covers
/// non-finite values), is the 1900-system serial 60, or falls outside
/// `i32`.
pub fn serial_to_days(serial: f64, is_1904: bool) -> Option<i32> {
    if serial.fract() != 0.0 || !serial.is_finite() {
        return None;
    }
    if serial < i64::MIN as f64 || serial > i64::MAX as f64 {
        return None;
    }
    let s = serial as i64;
    let days = if is_1904 {
        s.checked_sub(DAYS_1904)?
    } else if s == 60 {
        return None;
    } else if s < 60 {
        s.checked_sub(DAYS_1900_EARLY)?
    } else {
        s.checked_sub(DAYS_1900)?
    };
    i32::try_from(days).ok()
}

/// Days since 1970-01-01 → `YYYY-MM-DD` (Howard Hinnant's
/// `civil_from_days`, the inverse of `infer::days_from_civil` — same
/// formula as `epiora-core`'s `tabulate::categories::iso_date`).
pub(crate) fn days_to_iso(z: i32) -> String {
    let z = i64::from(z) + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", y + i64::from(m <= 2), m, d)
}
