//! `dataset_csv` — the deterministic CSV a generated `analysis.R`
//! reads back (v1.0 Task 17, spec §8.1). UTF-8, header row, `,`
//! delimiter, `\n` line endings, reals in shortest round-trip form
//! (`ryu`), dates ISO `YYYY-MM-DD`, booleans `TRUE`/`FALSE`.
//!
//! Missing values and empty strings must stay distinct from each
//! other and from the text `"NA"` (Review Focus 4): a missing cell
//! is written as a per-column *marker* — the shortest run of `∅`
//! that no text cell of that column already contains — so a literal
//! `"∅"` (or the text `"NA"`, or `""`) can never be mistaken for a
//! missing value when the script's `x[x == marker] <- NA` pass runs.
//! Non-text columns always use a single `∅` (their cells can never
//! hold it). `missing_markers` returns the markers so the script
//! generator can embed them.

use crate::dataset::{ColumnData, Dataset};

/// The marker alphabet: `∅`, `∅∅`, `∅∅∅`, … — the first run not
/// present among the column's text cells wins.
fn marker_for(data: &ColumnData) -> String {
    let mut marker = String::from('\u{2205}');
    if let ColumnData::Text(cells) = data {
        while cells.iter().flatten().any(|s| s == &marker) {
            marker.push('\u{2205}');
        }
    }
    marker
}

/// The per-column missing markers `dataset_csv` writes, in column
/// order. The R generator embeds them in the `*_col` reader calls.
pub fn missing_markers(ds: &Dataset) -> Vec<String> {
    ds.columns().iter().map(|c| marker_for(&c.data)).collect()
}

/// Days since 1970-01-01 → `YYYY-MM-DD` (Howard Hinnant's
/// `civil_from_days`, the same inversion `tabulate` uses for
/// category labels — including pre-epoch dates). `pub(crate)` for
/// `r_script`'s `as.Date` literals.
pub(crate) fn iso_date(z: i32) -> String {
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

/// Seconds since midnight → `hh:mm:ss` — the `Time` display (v1.1
/// spec §4.3), shared by the R-export dataset cells.
pub(crate) fn clock_time(t: i32) -> String {
    format!("{:02}:{:02}:{:02}", t / 3600, (t % 3600) / 60, t % 60)
}

/// Seconds since 1970-01-01 (no zone) → `YYYY-MM-DD hh:mm:ss` — the
/// `DateTime` display (v1.1 spec §4.3). The day part is `i64` domain:
/// extreme `DateTime` values exceed the `i32` day range.
pub(crate) fn iso_datetime(t: i64) -> String {
    let z = t.div_euclid(86_400) + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let s = t.rem_euclid(86_400);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y + i64::from(m <= 2),
        m,
        d,
        s / 3600,
        (s % 3600) / 60,
        s % 60
    )
}

fn real(x: f64) -> String {
    ryu::Buffer::new().format_finite(x).to_string()
}

/// RFC 4180 quoting: a field containing `,` `"` `\n` or `\r` is
/// wrapped in quotes with its quotes doubled.
fn field(out: &mut String, text: &str) {
    if text.contains([',', '"', '\n', '\r']) {
        out.push('"');
        for ch in text.chars() {
            if ch == '"' {
                out.push('"');
            }
            out.push(ch);
        }
        out.push('"');
    } else {
        out.push_str(text);
    }
}

fn row(out: &mut String, cells: &mut Vec<String>) {
    let mut first = true;
    for cell in cells.drain(..) {
        if !first {
            out.push(',');
        }
        first = false;
        field(out, &cell);
    }
    out.push('\n');
}

/// The dataset as CSV bytes (see the module doc). Deterministic:
/// columns in dataset order, rows in record order.
pub fn dataset_csv(ds: &Dataset) -> Vec<u8> {
    let markers = missing_markers(ds);
    let n = ds.row_count();
    let mut out = String::new();
    row(
        &mut out,
        &mut ds
            .columns()
            .iter()
            .map(|c| c.name.clone())
            .collect::<Vec<_>>(),
    );
    let mut cells = Vec::with_capacity(ds.columns().len());
    for i in 0..n {
        for (j, c) in ds.columns().iter().enumerate() {
            let marker = &markers[j];
            let cell = match &c.data {
                ColumnData::Integer(v) => match v[i] {
                    Some(x) => x.to_string(),
                    None => marker.clone(),
                },
                ColumnData::Real(v) => match v[i] {
                    Some(x) => real(x),
                    None => marker.clone(),
                },
                ColumnData::Text(v) => match &v[i] {
                    Some(s) => s.clone(),
                    None => marker.clone(),
                },
                ColumnData::Boolean(v) => match v[i] {
                    Some(b) => if b { "TRUE" } else { "FALSE" }.to_string(),
                    None => marker.clone(),
                },
                ColumnData::Date(v) => match v[i] {
                    Some(d) => iso_date(d),
                    None => marker.clone(),
                },
                ColumnData::Time(v) => match v[i] {
                    Some(t) => clock_time(t),
                    None => marker.clone(),
                },
                ColumnData::DateTime(v) => match v[i] {
                    Some(t) => iso_datetime(t),
                    None => marker.clone(),
                },
            };
            cells.push(cell);
        }
        row(&mut out, &mut cells);
    }
    out.into_bytes()
}
