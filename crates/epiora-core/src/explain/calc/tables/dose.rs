//! `dose_response` — per-level odds ratios, MH and X²mh leaves.
use super::*;
use crate::calc::counts::dose_response as dr;

/// The nested `table` input as `[score, cases, controls]` strata —
/// the run's parsing.
pub(crate) fn dr_strata(inputs: &Value) -> Vec<Vec<[f64; 3]>> {
    let items = inputs["table"].as_array().cloned().unwrap_or_default();
    let nested = !items.is_empty()
        && items
            .iter()
            .all(|e| e.as_array().is_some_and(|r| r.iter().all(Value::is_array)));
    if nested {
        items.iter().map(dr::stratum_table).collect()
    } else {
        vec![dr::stratum_table(&inputs["table"])]
    }
}

/// The pooled stratum rows — the run's column sums.
pub(crate) fn dr_crude(strata: &[Vec<[f64; 3]>]) -> Vec<[f64; 3]> {
    strata
        .first()
        .map(|s| {
            s.iter()
                .enumerate()
                .map(|(i, l)| {
                    [
                        l[0],
                        strata.iter().map(|t| t[i][1]).sum(),
                        strata.iter().map(|t| t[i][2]).sum(),
                    ]
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `odds_ratio[j]` of one stratum or the crude table — `dr_or` of
/// level j against the baseline row.
pub(crate) fn dr_or_leaf(
    s: &[[f64; 3]],
    j: usize,
    statistic: &str,
    est: crate::stats::Estimate,
) -> Result<Explanation, EngineError> {
    if j >= s.len() {
        return Err(unknown(statistic));
    }
    Ok(closed(
        statistic,
        "dr_or",
        finite(&[
            ("a", s[j][1]),
            ("b", s[j][2]),
            ("a_base", s[0][1]),
            ("b_base", s[0][2]),
        ]),
        est,
    ))
}

pub(crate) fn dose_response(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let ss = dr_strata(inputs);
    let (cc, un) = (dr::xmh(&ss, true), dr::xmh(&ss, false));

    match segs {
        [f, l] if f.key == "crude" && l.key == "odds_ratio" => {
            let j = l.index.ok_or_else(u)?;
            dr_or_leaf(&dr_crude(&ss), j, statistic, est)
        }
        [f, l] if f.key == "strata" && l.key == "odds_ratio" => {
            let i = f.index.ok_or_else(u)?;
            let j = l.index.ok_or_else(u)?;
            dr_or_leaf(ss.get(i).ok_or_else(u)?, j, statistic, est)
        }
        [f] if f.key == "mh_or" => {
            let i = f.index.ok_or_else(u)?;
            let nlev = ss.first().map_or(0, |s| s.len());
            if i + 1 >= nlev {
                return Err(u());
            }
            let (num, den) = dr::mh_or_parts(&ss, i + 1);
            Ok(closed(
                statistic,
                "dr_mh_or",
                finite(&[("num", num), ("den", den)]),
                est,
            ))
        }
        [f, which, l] if f.key == "xmh" => {
            let (v1sum, vsum) = dr::xmh_parts(&ss);
            let (fid, x) = match which.key.as_str() {
                "cc" => ("xmh_cc", cc[0]),
                "uncorrected" => ("xmh", un[0]),
                _ => return Err(u()),
            };
            match l.key.as_str() {
                "value" => Ok(closed(
                    statistic,
                    fid,
                    finite(&[("v1sum", v1sum), ("vsum", vsum)]),
                    est,
                )),
                "p" => Ok(closed(
                    statistic,
                    "chi_sf",
                    finite(&[("x", x), ("df", 1.0)]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        _ => Err(u()),
    }
}
