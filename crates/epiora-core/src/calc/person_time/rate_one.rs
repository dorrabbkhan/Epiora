//! `rate_one` — one Poisson rate with the five CI methods the corpus
//! records: mid-p and Fisher (`exactci::poisson.exact`), normal
//! (`epitools::pois.approx`), Byar (`epitools::pois.byar`) and
//! Rothman–Greenland (`poisson.R::rg_ci`), all scaled by
//! `per_unit / person_time`.

use super::super::counts::shared::{self, leaf};
use crate::stats::{self as rates};
use crate::EngineError;
use serde_json::{json, Value};

/// `per_unit`, or OpenEpi's `PersonTime1.js` auto-scale when omitted:
/// the smallest power of 10 that makes the raw rate `>= 1` (a rate of
/// 0 keeps `1` — OpenEpi's `while (rate < 1)` loop never exits there).
pub(crate) fn per_unit(inputs: &Value, cases: u64, pt: f64) -> f64 {
    match inputs.get("per_unit").and_then(Value::as_f64) {
        Some(u) => u,
        None => {
            let (mut u, mut r) = (1.0, cases as f64 / pt);
            while r < 1.0 && r > 0.0 {
                r *= 10.0;
                u *= 10.0;
            }
            u
        }
    }
}

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let cases = shared::cnt(inputs, "cases");
    let pt = shared::num(inputs, "person_time");
    let conf = shared::conf(inputs);
    let pu = per_unit(inputs, cases, pt);
    let scale = pu / pt;
    // Review Focus 3: at zero events every method's lower limit is
    // exactly 0 and every upper is finite — except Rothman–Greenland,
    // whose log formula degenerates to R's `exp(-Inf)=0` / `NaN`, so
    // its upper is `{"undefined": "out_of_range"}` like `smr`'s.
    let zero = cases == 0;
    let ci = |pair: [f64; 2]| {
        let [lo, hi] = pair;
        json!({
            "lower": leaf(if zero && lo != 0.0 { 0.0 } else { lo }),
            "upper": leaf(hi),
        })
    };
    let [_, na_lo, na_hi] = rates::pois_approx(cases, pt, conf);
    let [rg_lo, rg_hi] = rates::rg_ci(cases, pt, conf);
    let midp = shared::pois_midp_ci(cases, conf);
    let fisher = shared::pois_fisher_ci(cases, conf);
    let byar = shared::pois_byar(cases, conf);
    Ok(json!({
        "estimate": leaf(cases as f64 / pt * pu),
        "ci": {
            "midp": ci([midp[0] * scale, midp[1] * scale]),
            "fisher": ci([fisher[0] * scale, fisher[1] * scale]),
            "normal": ci([na_lo * pu, na_hi * pu]),
            "byar": ci([byar[0] * scale, byar[1] * scale]),
            "rothman_greenland": ci([rg_lo * pu, rg_hi * pu]),
        },
    }))
}
