//! `rate_compare` — the OpenEpi Person Time 2 calculator: per stratum
//! (and over the crude table) the two rates, the pooled z-score, the
//! Wald rate ratio, the Taylor/Byar rate difference, the
//! Martin–Austin CMLE rate ratio and the etiologic/prevented
//! fractions; for `k > 1` strata also the adjusted block. Ported from
//! `fixtures/R/calc.R::rate_compare_table`/`rate_compare_write` and
//! `fixtures/R/methods/rates.R`.

use super::super::counts::shared::{leaf, stat_ci, undefined};
use super::martin_pt::{cmle_rr, PtStratum};
use crate::stats::{self as rates};
use crate::EngineError;
use serde_json::{json, Value};

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
/// exact work exceeds the D1 cap.
fn cmle_block(t: &PtStratum, conf: f64) -> Value {
    let [est, ml, mu, fl, fu] = match cmle_rr(std::slice::from_ref(t), conf) {
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
/// block.
fn table_block(t: &PtStratum, pu: f64, conf: f64) -> Value {
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
        "cmle_rr": cmle_block(t, conf),
        "etiologic": {
            "efp": stat_ci([ef[0][0] * 100.0, ef[0][1] * 100.0, ef[0][2] * 100.0]),
            "efe": stat_ci([ef[1][0] * 100.0, ef[1][1] * 100.0, ef[1][2] * 100.0]),
            "pfp": stat_ci([ef[2][0] * 100.0, ef[2][1] * 100.0, ef[2][2] * 100.0]),
            "pfe": stat_ci([ef[3][0] * 100.0, ef[3][1] * 100.0, ef[3][2] * 100.0]),
        },
    })
}

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
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
    let mut out = json!({
        "strata": strata.iter().map(|t| table_block(t, pu, conf)).collect::<Vec<_>>(),
        "crude": table_block(&crude, pu, conf),
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
