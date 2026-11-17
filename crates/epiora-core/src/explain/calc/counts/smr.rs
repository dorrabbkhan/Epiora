//! `smr` — the standardized mortality ratio leaves.
use super::*;
use crate::calc::counts::smr::{byar, oe_chisq};

pub(crate) fn smr(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let o = cnt(inputs, "observed") as f64;
    let e = num(inputs, "expected");
    let conf = conf(inputs);
    let zz = z(conf);

    match segs {
        [f] if f.key == "estimate" => {
            Ok(closed(statistic, "smr", vals(&[("o", o), ("e", e)]), est))
        }
        [f, l] if f.key == "test" => match l.key.as_str() {
            "midp_p" => Ok(method(
                statistic,
                "method.pois_midp",
                vals(&[("o", o), ("e", e)]),
                est,
            )),
            "fisher_p" => Ok(method(
                statistic,
                "method.pois_fisher",
                vals(&[("o", o), ("e", e)]),
                est,
            )),
            "byar_chisq" => Ok(closed(
                statistic,
                "byar_chisq",
                vals(&[("o", o), ("e", e)]),
                est,
            )),
            "byar_p" => {
                let x2 = byar(o, e)[0];
                Ok(closed(
                    statistic,
                    "chi_sf",
                    finite(&[("x", x2), ("df", 1.0)]),
                    est,
                ))
            }
            "chisq_value" => Ok(closed(
                statistic,
                "chisq_oe",
                vals(&[("o", o), ("e", e)]),
                est,
            )),
            "chisq_p" => {
                let cs = oe_chisq(o, e);
                Ok(closed(
                    statistic,
                    "chi_sf",
                    finite(&[("x", cs), ("df", 1.0)]),
                    est,
                ))
            }
            _ => Err(u()),
        },
        [f, m, l] if f.key == "ci" => {
            let side = l.key.as_str();
            if side != "lower" && side != "upper" {
                return Err(u());
            }
            match m.key.as_str() {
                "midp" => Ok(method(
                    statistic,
                    "method.pois_midp_ci",
                    vals(&[("o", o), ("e", e), ("conf", conf)]),
                    est,
                )),
                "fisher" => Ok(method(
                    statistic,
                    "method.pois_fisher_ci",
                    vals(&[("o", o), ("e", e), ("conf", conf)]),
                    est,
                )),
                "byar" => Ok(closed(
                    statistic,
                    if side == "lower" {
                        "pois_byar_lower"
                    } else {
                        "pois_byar_upper"
                    },
                    vals(&[("x", o), ("z", zz), ("e", e)]),
                    est,
                )),
                "rothman_greenland" => Ok(closed(
                    statistic,
                    if side == "lower" {
                        "smr_rg_lower"
                    } else {
                        "smr_rg_upper"
                    },
                    vals(&[("o", o), ("e", e), ("z", zz)]),
                    est,
                )),
                "ury_wiggins" => Ok(closed(
                    statistic,
                    if side == "lower" {
                        "smr_ury_lower"
                    } else {
                        "smr_ury_upper"
                    },
                    vals(&[("o", o), ("e", e), ("z", zz), ("conf", conf)]),
                    est,
                )),
                "vandenbroucke" => Ok(closed(
                    statistic,
                    if side == "lower" {
                        "smr_vb_lower"
                    } else {
                        "smr_vb_upper"
                    },
                    vals(&[("o", o), ("e", e)]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        _ => Err(u()),
    }
}
