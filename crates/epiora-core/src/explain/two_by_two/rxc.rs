//! `rxc` — the Pearson chi-square of the pooled R×C table (v1.0
//! Task 10, spec §6).

use super::super::path::Segment;
use super::super::steps::{closed, leaf};
use super::super::{options, Explanation};
use super::{ctx, finite};
use crate::dataset::Dataset;
use crate::error::EngineError;
use crate::procedures::{tables, TablesOptions, TablesResult};
use serde_json::Value;

pub(crate) fn explain_rxc(
    ds: &Dataset,
    opts_json: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let opts: TablesOptions = options(opts_json)?;
    let out = tables(ds, &opts, &ctx())?;
    let (stats, counts) = match &out.result {
        TablesResult::RxC { stats, counts, .. } => (stats, counts),
        _ => return Err(super::super::path::unknown(statistic)),
    };
    let tree = serde_json::to_value(stats).map_err(|e| EngineError::InvalidOptions {
        reason: format!("result does not serialize: {e}"),
    })?;
    let est = leaf(&tree, segs, statistic)?;
    let u = || super::super::path::unknown(statistic);

    let p = crate::stats::rxc_parts(counts);
    let (rows, cols) = (p.rows, p.cols);
    let cell = |r: usize, c: usize| counts[r].get(c).copied().unwrap_or(0) as f64;
    let stat_v = p.statistic;

    match segs {
        [f, l] if f.key == "chi_square" && l.key == "df" => Ok(closed(
            statistic,
            "chi_df",
            finite(&[("rows", rows as f64), ("cols", cols as f64)]),
            est,
        )),
        [f, l] if f.key == "chi_square" && l.key == "statistic" => {
            let mut v = std::collections::BTreeMap::new();
            for r in 0..rows {
                for c in 0..cols {
                    let i = r * cols + c;
                    v.insert(format!("o_{i}"), cell(r, c));
                    let e = p.expected[i];
                    if e.is_finite() {
                        v.insert(format!("e_{i}"), e);
                    }
                }
            }
            Ok(closed(statistic, "chi_square_cells", v, est))
        }
        [f, l] if f.key == "chi_square" && l.key == "p_value" => Ok(closed(
            statistic,
            "chi_sf",
            finite(&[("x", stat_v), ("df", p.df)]),
            est,
        )),
        _ => Err(u()),
    }
}
