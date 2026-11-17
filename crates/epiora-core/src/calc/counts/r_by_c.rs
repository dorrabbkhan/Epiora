//! `r_by_c` — Pearson's chi-square on an r×c table with the
//! expected-cell diagnostics (Cochran's criteria) the corpus
//! records. The expected cells and the chi-square sum come from
//! `stats::rxc_parts` — the same intermediates the `rxc` procedure
//! and the explain trace use.

use super::shared::*;
use crate::error::EngineError;
use crate::stats::rxc_parts;
use serde_json::{json, Value};

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let t = count_table(inputs, "table");
    let p = rxc_parts(&t);
    let (r, c) = (p.rows, p.cols);
    let df = (r - 1) * (c - 1);
    // A zero row or column margin makes an expected cell 0 and
    // `chisq.test` returns NaN — the corpus records those leaves null.
    let degenerate = p.row_totals.contains(&0.0) || p.col_totals.contains(&0.0);
    Ok(json!({
        "chisq": {
            "value": if degenerate { undefined("zero_expected") } else { leaf(p.statistic) },
            "df": leaf(df as f64),
            "p": if degenerate { undefined("zero_expected") } else { leaf(chi_p(p.statistic, df as f64)) },
        },
        "expected_lt5": {
            "count": leaf(p.lt5 as f64),
            "percent": leaf(p.lt5 as f64 / (r * c) as f64 * 100.0),
        },
        "expected_lt1": { "count": leaf(p.lt1 as f64) },
    }))
}
