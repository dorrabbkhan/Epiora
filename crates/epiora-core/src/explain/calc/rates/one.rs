//! `rate_one` — a single person-time rate and its CI set.
use super::*;
use crate::calc::counts::shared::num;
use crate::calc::person_time::rate_one::{per_unit, pu_scale};

pub(crate) fn rate_one(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let cases = cnt(inputs, "cases");
    let pt = num(inputs, "person_time");
    let conf = conf(inputs);
    let pu = per_unit(inputs, cases, pt);
    let scale = pu_scale(pu, pt);
    let zz = z(conf);

    match segs {
        [f] if f.key == "estimate" => Ok(closed(
            statistic,
            "rate",
            vals(&[("cases", cases as f64), ("pt", pt), ("per_unit", pu)]),
            est,
        )),
        [f, m, l] if f.key == "ci" => {
            let side = l.key.as_str();
            if side != "lower" && side != "upper" {
                return Err(u());
            }
            match m.key.as_str() {
                "midp" => Ok(method(
                    statistic,
                    "method.pois_rate_midp",
                    vals(&[("x", cases as f64), ("scale", scale), ("conf", conf)]),
                    est,
                )),
                "fisher" => Ok(method(
                    statistic,
                    "method.pois_rate_fisher",
                    vals(&[("x", cases as f64), ("scale", scale), ("conf", conf)]),
                    est,
                )),
                "normal" => Ok(closed(
                    statistic,
                    if side == "lower" {
                        "rate_taylor_lower"
                    } else {
                        "rate_taylor_upper"
                    },
                    vals(&[("x", cases as f64), ("pt", pt), ("z", zz), ("per_unit", pu)]),
                    est,
                )),
                "byar" => Ok(closed(
                    statistic,
                    if side == "lower" {
                        "pois_byar_scaled_lower"
                    } else {
                        "pois_byar_scaled_upper"
                    },
                    vals(&[("x", cases as f64), ("z", zz), ("scale", scale)]),
                    est,
                )),
                "rothman_greenland" => Ok(closed(
                    statistic,
                    if side == "lower" {
                        "rate_rg_lower"
                    } else {
                        "rate_rg_upper"
                    },
                    vals(&[("o", cases as f64), ("pt", pt), ("z", zz), ("per_unit", pu)]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        _ => Err(u()),
    }
}
