//! `r_by_c` — the R×C chi-square leaves.
use super::*;
use crate::calc::counts::shared::{self};

pub(crate) fn r_by_c(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let t = shared::count_table(inputs, "table");
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

    match segs {
        [f, l] if f.key == "chisq" => match l.key.as_str() {
            "value" => {
                let mut v = Vals::new();
                for (i, row) in t.iter().enumerate() {
                    for (j, &cell) in row.iter().enumerate() {
                        let k = i * c + j;
                        v.insert(format!("o_{k}"), cell as f64);
                        let e = rows[i] * cols[j] / n;
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
                finite(&[("rows", r as f64), ("cols", c as f64)]),
                est,
            )),
            "p" => Ok(closed(
                statistic,
                "chi_sf",
                finite(&[("x", chi), ("df", df as f64)]),
                est,
            )),
            _ => Err(u()),
        },
        [f, l] if f.key == "expected_lt5" || f.key == "expected_lt1" => {
            let count = if f.key == "expected_lt5" { lt5 } else { lt1 };
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
                    finite(&[("count", lt5 as f64), ("total_cells", (r * c) as f64)]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        _ => Err(u()),
    }
}

// --- dose_response -----------------------------------------------------------
