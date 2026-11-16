//! `rate_compare` — the OpenEpi Person Time 2 calculator: per stratum
//! (and over the crude table) the two rates, the pooled z-score, the
//! Wald rate ratio, the Taylor/Byar rate difference, the
//! Martin–Austin CMLE rate ratio and the etiologic/prevented
//! fractions; for `k > 1` strata also the adjusted block. Ported from
//! `fixtures/R/calc.R::rate_compare_table`/`rate_compare_write` and
//! `fixtures/R/methods/rates.R`.

use super::super::counts::shared::{leaf, stat_ci, undefined};
use super::martin_pt::{call_too_large, cmle_rr, PtStratum};
use crate::calc::input::MAX_COUNT;
use crate::stats::{self as rates};
use crate::EngineError;
use serde_json::{json, Value};

/// Spec §5.3 / inventory `## rate_compare`: each `strata` element is
/// one 2×2 — row 0 case counts (non-negative integers `< 2^53`; an
/// integral real like `4e15` counts, since JSON writes it with an
/// exponent) and row 1 person-time (finite, `> 0`) — a per-row
/// contract the uniform `CellKind::Real` table cannot express, so it
/// is checked here as typed `calc_input_out_of_range` errors.
fn check_strata(inputs: &Value) -> Result<(), EngineError> {
    let err = |reason: &str| EngineError::CalcInputOutOfRange {
        input: "strata".into(),
        reason: reason.into(),
    };
    let items = inputs
        .get("strata")
        .and_then(Value::as_array)
        .ok_or_else(|| err("bad_table_shape"))?;
    let nested = !items.is_empty()
        && items
            .iter()
            .all(|e| e.as_array().is_some_and(|r| r.iter().all(Value::is_array)));
    let tables: Vec<&[Value]> = if nested {
        items
            .iter()
            .map(|t| t.as_array().map(Vec::as_slice).unwrap_or(&[]))
            .collect()
    } else {
        vec![items.as_slice()]
    };
    for rows in tables {
        let cells: Vec<&[Value]> = rows
            .iter()
            .map(|r| r.as_array().map(Vec::as_slice).unwrap_or(&[]))
            .collect();
        if rows.len() != 2 || cells.iter().any(|r| r.len() != 2) {
            return Err(err("bad_table_shape"));
        }
        let is_count = |c: &Value| {
            c.as_f64()
                .is_some_and(|x| x >= 0.0 && x.fract() == 0.0 && x < MAX_COUNT as f64)
        };
        if !cells[0].iter().all(is_count) {
            return Err(err("not_a_count"));
        }
        if !cells[1]
            .iter()
            .all(|c| c.as_f64().is_some_and(|x| x.is_finite() && x > 0.0))
        {
            return Err(err("below_min"));
        }
    }
    Ok(())
}

/// The `strata` input — `[[a,b],[n1,n0]]` or a list of those — as
/// `PtStratum`s (the same shape rule as `counts::shared::strata2x2`).
fn strata(inputs: &Value) -> Vec<PtStratum> {
    let cell = |v: Option<&Value>| v.and_then(Value::as_f64).unwrap_or(0.0);
    let one = |t: &Value| PtStratum {
        a: cell(t.get(0).and_then(|r| r.get(0))),
        b: cell(t.get(0).and_then(|r| r.get(1))),
        n1: cell(t.get(1).and_then(|r| r.get(0))),
        n0: cell(t.get(1).and_then(|r| r.get(1))),
    };
    inputs
        .get("strata")
        .and_then(Value::as_array)
        .map(|items| {
            if !items.is_empty()
                && items
                    .iter()
                    .all(|e| e.as_array().is_some_and(|r| r.iter().all(Value::is_array)))
            {
                items.iter().map(one).collect()
            } else {
                vec![one(&Value::Array(items.clone()))]
            }
        })
        .unwrap_or_default()
}

/// `pt_rate_stat` — est plus the `pois.approx` CI, scaled by
/// `per_unit`.
fn rate_stat(cases: f64, pt: f64, pu: f64, conf: f64) -> Value {
    let z = super::super::counts::shared::z(conf);
    let x = cases;
    json!({
        "est": leaf(x / pt * pu),
        "lower": leaf((x - z * libm::sqrt(x)) / pt * pu),
        "upper": leaf((x + z * libm::sqrt(x)) / pt * pu),
    })
}

/// The five `cmle_rr` leaves: finite roots, or `too_large` when the
/// exact work exceeds the D1 cap — or when `call_capped` says the
/// call's Σm1 crossed the T4 total-support cap, which marks every
/// stratum's and the crude block's leaves at once.
fn cmle_block(t: &PtStratum, conf: f64, call_capped: bool) -> Value {
    let result = if call_capped {
        None
    } else {
        cmle_rr(std::slice::from_ref(t), conf)
    };
    let [est, ml, mu, fl, fu] = match result {
        Some(v) => v,
        None => {
            return json!({
                "est": undefined("too_large"),
                "midp_lower": undefined("too_large"),
                "midp_upper": undefined("too_large"),
                "fisher_lower": undefined("too_large"),
                "fisher_upper": undefined("too_large"),
            });
        }
    };
    json!({
        "est": leaf(est),
        "midp_lower": leaf(ml),
        "midp_upper": leaf(mu),
        "fisher_lower": leaf(fl),
        "fisher_upper": leaf(fu),
    })
}

/// `rate_compare_table` — one stratum's (or the crude table's) full
/// block. `call_capped` is the T4 whole-call `cmle_rr` verdict.
fn table_block(t: &PtStratum, pu: f64, conf: f64, call_capped: bool) -> Value {
    let (a, b, n1, n0) = (t.a, t.b, t.n1, t.n0);
    let zs = rates::pt_zscore(a, b, n1, n0);
    let rr = rates::rate_ratio(a, b, n1, n0, conf);
    let rd = rates::rd_byar(a, b, n1, n0, conf);
    let ef = rates::ef_person_time(a, b, n1, n0, conf);
    json!({
        "zscore": { "value": leaf(zs[0]), "p_1t": leaf(zs[1]), "p_2t": leaf(zs[2]) },
        "rate": {
            "exposed": rate_stat(a, n1, pu, conf),
            "unexposed": rate_stat(b, n0, pu, conf),
            "overall": rate_stat(a + b, n1 + n0, pu, conf),
        },
        "rate_ratio": stat_ci(rr),
        "rate_difference": {
            "est": leaf(rd[0] * pu),
            "lower": leaf(rd[1] * pu),
            "upper": leaf(rd[2] * pu),
        },
        "cmle_rr": cmle_block(t, conf, call_capped),
        "etiologic": {
            "efp": stat_ci([ef[0][0] * 100.0, ef[0][1] * 100.0, ef[0][2] * 100.0]),
            "efe": stat_ci([ef[1][0] * 100.0, ef[1][1] * 100.0, ef[1][2] * 100.0]),
            "pfp": stat_ci([ef[2][0] * 100.0, ef[2][1] * 100.0, ef[2][2] * 100.0]),
            "pfe": stat_ci([ef[3][0] * 100.0, ef[3][1] * 100.0, ef[3][2] * 100.0]),
        },
    })
}

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    check_strata(inputs)?;
    let strata = strata(inputs);
    let pu = super::super::counts::shared::num(inputs, "per_unit");
    let conf = super::super::counts::shared::conf(inputs);
    let crude = strata.iter().fold(
        PtStratum {
            a: 0.0,
            b: 0.0,
            n1: 0.0,
            n0: 0.0,
        },
        |m, t| PtStratum {
            a: m.a + t.a,
            b: m.b + t.b,
            n1: m.n1 + t.n1,
            n0: m.n0 + t.n0,
        },
    );
    // Owner decision T4 (PR #78): the total-support cap applies to
    // the whole call — decide it once, before any per-stratum work.
    let call_capped = call_too_large(&strata);
    let mut out = json!({
        "strata": strata
            .iter()
            .map(|t| table_block(t, pu, conf, call_capped))
            .collect::<Vec<_>>(),
        "crude": table_block(&crude, pu, conf, call_capped),
    });
    if strata.len() > 1 {
        let rows: Vec<[f64; 4]> = strata.iter().map(|t| [t.a, t.b, t.n1, t.n0]).collect();
        let adj = rates::adjusted_rates(&rows, conf);
        out["adjusted"] = json!({
            "zscore": { "value": leaf(adj.zscore[0]), "p": leaf(adj.zscore[1]) },
            "direct_rr": stat_ci(adj.direct_rr),
            "mh_rr": stat_ci(adj.mh_rr),
            "direct_rd": stat_ci(adj.direct_rd),
            "breslow_day": {
                "rr_chisq": leaf(adj.rr[0]),
                "rr_p": leaf(adj.rr[1]),
                "rd_chisq": leaf(adj.rd[0]),
                "rd_p": leaf(adj.rd[1]),
            },
        });
    }
    Ok(out)
}
