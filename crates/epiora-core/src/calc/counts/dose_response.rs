//! `dose_response` — per-stratum and crude odds ratios of each
//! level against the baseline, the pooled MH OR per level, and the
//! extended Mantel–Haenszel trend chi-squares (Rosner continuity
//! correction and Schlesselman uncorrected) of
//! `fixtures/R/methods/dose_response.R`.

use super::shared::*;
use crate::error::EngineError;
use serde_json::{json, Value};

/// One stratum's `[score, cases, controls]` rows.
pub(crate) fn stratum_table(v: &Value) -> Vec<[f64; 3]> {
    v.as_array()
        .map(|rows| {
            rows.iter()
                .map(|r| {
                    let r = r.as_array();
                    let g = |i: usize| {
                        r.and_then(|x| x.get(i))
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0)
                    };
                    [g(0), g(1), g(2)]
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `dr_or_levels` — crude OR `ad/bc` of level i against the baseline
/// row (the epiR Wald estimate; the baseline returns 1, the
/// ad/bc fallback on cells epiR cannot score).
pub(crate) fn or_levels(s: &[[f64; 3]]) -> Vec<f64> {
    let (abase, bbase) = (s[0][1], s[0][2]);
    s.iter().map(|l| (l[1] * bbase) / (l[2] * abase)).collect()
}

/// `(num, den)` — the MH pooled sums of level `i` vs baseline across
/// `strata`; `num/den` is the `dr_mh_or` estimate. Shared by `mh_or`
/// and the explain trace (v1.0 Task 10, spec §6).
pub(crate) fn mh_or_parts(strata: &[Vec<[f64; 3]>], i: usize) -> (f64, f64) {
    let (mut num, mut den) = (0.0, 0.0);
    for s in strata {
        let (a, b) = (s[i][1], s[i][2]);
        let (ab, bb) = (s[0][1], s[0][2]);
        let tt = a + bb + b + ab;
        num += a * bb / tt;
        den += b * ab / tt;
    }
    (num, den)
}

/// `dr_mh_or` — MH OR of level i vs baseline pooled over strata.
pub(crate) fn mh_or(strata: &[Vec<[f64; 3]>]) -> Vec<f64> {
    let nlev = strata.first().map_or(0, |s| s.len());
    (1..nlev)
        .map(|i| {
            let (num, den) = mh_or_parts(strata, i);
            num / den
        })
        .collect()
}

/// The `xmh` pooled sums — `(v1sum, vsum)`, the observed-minus-
/// expected score sum and its variance across strata. `xmh` and the
/// explain trace share them so a trace reports the intermediates
/// the run itself used.
pub(crate) fn xmh_parts(strata: &[Vec<[f64; 3]>]) -> (f64, f64) {
    let (mut vsum, mut v1sum) = (0.0, 0.0);
    for s in strata {
        let a: Vec<f64> = s.iter().map(|l| l[1]).collect();
        let b: Vec<f64> = s.iter().map(|l| l[2]).collect();
        let x: Vec<f64> = s.iter().map(|l| l[0]).collect();
        let m: Vec<f64> = s.iter().map(|l| l[1] + l[2]).collect();
        let t1: f64 = a.iter().zip(&x).map(|(a, x)| a * x).sum();
        let t2: f64 = m.iter().zip(&x).map(|(m, x)| m * x).sum();
        let t3: f64 = m.iter().zip(&x).map(|(m, x)| m * x * x).sum();
        let (n1, n2): (f64, f64) = (a.iter().sum(), b.iter().sum());
        let n = n1 + n2;
        vsum += (n1 * n2 * (n * t3 - t2 * t2)) / (n * n * (n - 1.0));
        v1sum += t1 - (n1 / n) * t2;
    }
    (v1sum, vsum)
}

/// `xmh_trend` — the extended MH trend chi-square (df 1).
pub(crate) fn xmh(strata: &[Vec<[f64; 3]>], correct: bool) -> [f64; 2] {
    let (v1sum, vsum) = xmh_parts(strata);
    let num = if correct {
        (v1sum.abs() - 0.5).max(0.0)
    } else {
        v1sum
    };
    let chi = num * num / vsum;
    [chi, chi_p(chi, 1.0)]
}

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let err = |reason: &str| EngineError::CalcInputOutOfRange {
        input: "strata".into(),
        reason: reason.into(),
    };
    let items = inputs["table"].as_array().cloned().unwrap_or_default();
    let nested = !items.is_empty()
        && items
            .iter()
            .all(|e| e.as_array().is_some_and(|r| r.iter().all(Value::is_array)));
    // The flat form is the single stratum; the nested form a list of
    // strata tables — `validate` has already shaped each element.
    let strata: Vec<Vec<[f64; 3]>> = if nested {
        items.iter().map(stratum_table).collect()
    } else {
        vec![stratum_table(&inputs["table"])]
    };
    // The `strata` input declares the stratum count — a list that
    // disagrees with it is outside the contract, as is a stratum set
    // whose level counts differ (the crude table and the MH loop
    // index levels by position).
    if strata.len() as u64 != cnt(inputs, "strata") {
        return Err(err("bad_table_shape"));
    }
    if strata.iter().any(|s| s.len() != strata[0].len()) {
        return Err(err("bad_table_shape"));
    }
    let crude: Vec<[f64; 3]> = strata
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
        .unwrap_or_default();
    let cc = xmh(&strata, true);
    let un = xmh(&strata, false);
    Ok(json!({
        "strata": strata.iter().map(|s| json!({
            "odds_ratio": or_levels(s).iter().map(|&v| leaf(v)).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "crude": {
            "odds_ratio": or_levels(&crude).iter().map(|&v| leaf(v)).collect::<Vec<_>>(),
        },
        "mh_or": mh_or(&strata).iter().map(|&v| leaf(v)).collect::<Vec<_>>(),
        "xmh": {
            "cc": { "value": leaf(cc[0]), "p": leaf(cc[1]) },
            "uncorrected": { "value": leaf(un[0]), "p": leaf(un[1]) },
        },
    }))
}
