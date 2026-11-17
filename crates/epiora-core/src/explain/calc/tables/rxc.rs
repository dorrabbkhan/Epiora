//! `r_by_c` — the R×C chi-square leaves.
use super::*;
use crate::calc::counts::shared::{self};
use crate::stats::rxc_parts;

pub(crate) fn r_by_c(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let t = shared::count_table(inputs, "table");
    // The run's intermediates — `stats::rxc_parts`, the same helper
    // `calc::counts::r_by_c` and `stats::rxc` call.
    let p = rxc_parts(&t);

    match segs {
        [f, l] if f.key == "chisq" => match l.key.as_str() {
            "value" => {
                let mut v = Vals::new();
                for (i, row) in t.iter().enumerate() {
                    for (j, &cell) in row.iter().enumerate() {
                        let k = i * p.cols + j;
                        v.insert(format!("o_{k}"), cell as f64);
                        let e = p.expected.get(k).copied().unwrap_or(f64::NAN);
                        if e.is_finite() {
                            v.insert(format!("e_{k}"), e);
                        }
                    }
                }
                Ok(closed(statistic, "chi_square_cells", v, est))
            }
            "df" => Ok(closed(
                statistic,
                "chi_df",
                finite(&[("rows", p.rows as f64), ("cols", p.cols as f64)]),
                est,
            )),
            "p" => Ok(closed(
                statistic,
                "chi_sf",
                finite(&[("x", p.statistic), ("df", p.df)]),
                est,
            )),
            _ => Err(u()),
        },
        [f, l] if f.key == "expected_lt5" || f.key == "expected_lt1" => {
            let count = if f.key == "expected_lt5" {
                p.lt5
            } else {
                p.lt1
            };
            match (f.key.as_str(), l.key.as_str()) {
                (_, "count") => Ok(closed(
                    statistic,
                    "data",
                    vals(&[("value", count as f64)]),
                    est,
                )),
                ("expected_lt5", "percent") => Ok(closed(
                    statistic,
                    "cell_percent",
                    finite(&[
                        ("count", p.lt5 as f64),
                        ("total_cells", (p.rows * p.cols) as f64),
                    ]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        _ => Err(u()),
    }
}

// --- dose_response -----------------------------------------------------------
