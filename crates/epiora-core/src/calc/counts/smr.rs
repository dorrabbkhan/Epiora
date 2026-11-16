//! `smr` — the standardized mortality/morbidity ratio and its
//! battery of tests and CIs: `exactci::poisson.exact` mid-p/central,
//! `epitools::pois.byar`, and the `fixtures/R/methods/poisson.R`
//! shortcuts. `conf_level` accepts any percentage — the corpus's
//! `conf_97` fixture exercises the conditional Ury–Wiggins and
//! Vandenbroucke branches — so this input is `Percent`, not
//! `ConfLevel` (recorded in `docs/decisions.md`).

use super::shared::*;
use crate::error::EngineError;
use serde_json::{json, Value};

/// `byar_stat` — Byar's approximation: signed normal deviate
/// squared with `v = O` when `O > E` else `O + 1`.
pub(crate) fn byar(o: f64, e: f64) -> [f64; 2] {
    let v = if o > e { o } else { o + 1.0 };
    let x2 = libm::sqrt(9.0 * v) * (1.0 - 1.0 / (9.0 * v) - libm::pow(e / v, 1.0 / 3.0));
    [x2 * x2, chi_p(x2 * x2, 1.0)]
}

/// `ury_wiggins` — the shortcut CI, defined only at 90/95/99.
pub(crate) fn ury_wiggins(o: f64, e: f64, conf: f64) -> [f64; 2] {
    let zz = z(conf);
    let add = if o > 50.0 { 0.0 } else { 0.1 };
    let so = libm::sqrt(o);
    if conf == 99.0 {
        [(o - zz * so + 2.0) / e, (o + zz * so + 3.0 + add) / e]
    } else if conf == 95.0 {
        [(o - zz * so + 1.0) / e, (o + zz * so + 2.0 + add) / e]
    } else if conf == 90.0 {
        [(o - zz * so + 0.65) / e, (o + zz * so + 1.65 + add) / e]
    } else {
        [f64::NAN, f64::NAN]
    }
}

/// `vandenbroucke` — the 95% shortcut; undefined elsewhere.
pub(crate) fn vandenbroucke(o: f64, e: f64, conf: f64) -> [f64; 2] {
    if conf != 95.0 {
        return [f64::NAN, f64::NAN];
    }
    let zv = crate::special::normal_quantile(0.975).unwrap_or(1.959963984540054);
    let so = libm::sqrt(o);
    [
        (so - zv * 0.5) * (so - zv * 0.5) / e,
        (so + zv * 0.5) * (so + zv * 0.5) / e,
    ]
}

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let o = cnt(inputs, "observed") as f64;
    let e = num(inputs, "expected");
    let conf = conf(inputs);
    let conf_pct = conf;
    let zz = z(conf_pct);

    let (fisher_p, midp_p) = pois_pvals(o as u64, e);
    let [ml, mu] = pois_midp_ci(o as u64, conf);
    let [fl, fu] = pois_fisher_ci(o as u64, conf);
    let [bl, bu] = pois_byar(o as u64, conf);
    let b = byar(o, e);
    let cs = (o - e) * (o - e) / e;
    let uw = ury_wiggins(o, e, conf_pct);
    let vb = vandenbroucke(o, e, conf_pct);

    Ok(json!({
        "estimate": leaf(o / e),
        "test": {
            "midp_p": leaf(midp_p),
            "fisher_p": leaf(fisher_p),
            "byar_chisq": leaf(b[0]),
            "byar_p": leaf(b[1]),
            "chisq_value": leaf(cs),
            "chisq_p": leaf(chi_p(cs, 1.0)),
        },
        "ci": {
            "midp": { "lower": leaf(ml / e), "upper": leaf(mu / e) },
            "fisher": { "lower": leaf(fl / e), "upper": leaf(fu / e) },
            "byar": { "lower": leaf(bl / e), "upper": leaf(bu / e) },
            "rothman_greenland": {
                "lower": leaf(libm::exp(libm::log(o / e) - zz / libm::sqrt(o))),
                "upper": leaf(libm::exp(libm::log(o / e) + zz / libm::sqrt(o))),
            },
            "ury_wiggins": { "lower": leaf(uw[0]), "upper": leaf(uw[1]) },
            "vandenbroucke": { "lower": leaf(vb[0]), "upper": leaf(vb[1]) },
        },
    }))
}
