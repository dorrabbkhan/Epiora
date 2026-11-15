//! `r_by_c` — Pearson's chi-square on an r×c table with the
//! expected-cell diagnostics (Cochran's criteria) the corpus
//! records.

use super::shared::*;
use crate::error::EngineError;
use serde_json::{json, Value};

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let t = count_table(inputs, "table");
    let r = t.len();
    let c = t.first().map_or(0, |x| x.len());
    let rows: Vec<f64> = t.iter().map(|row| row.iter().sum::<u64>() as f64).collect();
    let cols: Vec<f64> = (0..c)
        .map(|j| t.iter().map(|row| row[j] as f64).sum())
        .collect();
    let n: f64 = rows.iter().sum();
    let mut chi = 0.0;
    let (mut lt5, mut lt1) = (0u64, 0u64);
    for (i, row) in t.iter().enumerate() {
        for (j, &cell) in row.iter().enumerate() {
            let e = rows[i] * cols[j] / n;
            if e < 5.0 {
                lt5 += 1;
            }
            if e < 1.0 {
                lt1 += 1;
            }
            if e > 0.0 {
                let o = cell as f64;
                chi += (o - e) * (o - e) / e;
            }
        }
    }
    let df = (r - 1) * (c - 1);
    // A zero row or column margin makes an expected cell 0 and
    // `chisq.test` returns NaN — the corpus records those leaves null.
    let degenerate = rows.contains(&0.0) || cols.contains(&0.0);
    Ok(json!({
        "chisq": {
            "value": if degenerate { undefined("zero_expected") } else { leaf(chi) },
            "df": leaf(df as f64),
            "p": if degenerate { undefined("zero_expected") } else { leaf(chi_p(chi, df as f64)) },
        },
        "expected_lt5": {
            "count": leaf(lt5 as f64),
            "percent": leaf(lt5 as f64 / (r * c) as f64 * 100.0),
        },
        "expected_lt1": { "count": leaf(lt1 as f64) },
    }))
}
