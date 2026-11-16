//! `binomial`, `poisson`, `proportion` — exact tails are method
//! traces; CIs report the helper intermediates.
use super::*;

pub(crate) fn binomial(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let x = cnt(inputs, "numerator");
    let n = cnt(inputs, "denominator");
    let p = num(inputs, "expected_percent") / 100.0;
    match segs {
        [f, l] if f.key == "tail" => {
            let fid = match l.key.as_str() {
                "lt" => "method.binom_tail_lt",
                "le" => "method.binom_tail_le",
                "ge" => "method.binom_tail_ge",
                "gt" => "method.binom_tail_gt",
                "eq" => "method.binom_eq",
                _ => return Err(u()),
            };
            Ok(method(
                statistic,
                fid,
                vals(&[("x", x as f64), ("n", n as f64), ("p", p)]),
                est,
            ))
        }
        [f] if f.key == "p_2t" => {
            let t = binom_tails(x, n, p);
            Ok(closed(
                statistic,
                "binom_p_2t",
                finite(&[("tail_le", t.le), ("tail_ge", t.ge)]),
                est,
            ))
        }
        [f, l] if f.key == "ci_95_count" => {
            let [cl, cu] = binom_cp_ci(x, n, 95.0);
            let bound = match l.key.as_str() {
                "lower" => cl,
                "upper" => cu,
                _ => return Err(u()),
            };
            Ok(closed(
                statistic,
                "binom_cp_count",
                finite(&[("bound", bound), ("n", n as f64)]),
                est,
            ))
        }
        _ => Err(u()),
    }
}

pub(crate) fn poisson(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let o = cnt(inputs, "observed");
    let e = num(inputs, "expected");
    match segs {
        [f, l] if f.key == "tail" => {
            let fid = match l.key.as_str() {
                "lt" => "method.pois_tail_lt",
                "le" => "method.pois_tail_le",
                "ge" => "method.pois_tail_ge",
                "gt" => "method.pois_tail_gt",
                "eq" => "method.pois_eq",
                _ => return Err(u()),
            };
            Ok(method(
                statistic,
                fid,
                vals(&[("x", o as f64), ("lambda", e)]),
                est,
            ))
        }
        _ => Err(u()),
    }
}

pub(crate) fn proportion(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let x = cnt(inputs, "numerator");
    let n = cnt(inputs, "denominator");
    let conf = conf(inputs);
    let pop = cnt(inputs, "population_size") as f64;
    match segs {
        [f] if f.key == "estimate" => Ok(closed(
            statistic,
            "proportion",
            vals(&[("x", x as f64), ("n", n as f64)]),
            est,
        )),
        [f, m, l] if f.key == "ci" => {
            let fid = match (m.key.as_str(), l.key.as_str()) {
                ("wald", "lower") => "prop_wald_lower",
                ("wald", "upper") => "prop_wald_upper",
                ("agresti_coull", "lower") => "prop_ac_lower",
                ("agresti_coull", "upper") => "prop_ac_upper",
                ("wilson", "lower") => "prop_scoreci_lower",
                ("wilson", "upper") => "prop_scoreci_upper",
                ("wilson_fpc", "lower") => "prop_wilson_fpc_lower",
                ("wilson_fpc", "upper") => "prop_wilson_fpc_upper",
                ("score_cc", "lower") => "prop_score_cc_lower",
                ("score_cc", "upper") => "prop_score_cc_upper",
                ("clopper_pearson", "lower") | ("clopper_pearson", "upper") => {
                    return Ok(method(
                        statistic,
                        "method.clopper_pearson",
                        xnz(x, n, conf),
                        est,
                    ));
                }
                ("midp", "lower") | ("midp", "upper") => {
                    return Ok(method(statistic, "method.midp_ci", xnz(x, n, conf), est));
                }
                _ => return Err(u()),
            };
            let mut v = xnz(x, n, conf);
            if m.key == "wilson_fpc" {
                v.insert("pop".into(), pop);
            }
            Ok(closed(statistic, fid, v, est))
        }
        [f, l] if f.key == "ztest" => {
            let p = x as f64 / n as f64;
            let p0 = inputs
                .get("compare_with")
                .and_then(Value::as_f64)
                .unwrap_or(f64::NAN)
                / 100.0;
            match l.key.as_str() {
                "z" => Ok(closed(
                    statistic,
                    "prop_ztest_z",
                    finite(&[("p", p), ("p0", p0), ("n", n as f64)]),
                    est,
                )),
                "p" => {
                    let zv = (p - p0) / libm::sqrt(p0 * (1.0 - p0) / n as f64);
                    Ok(closed(
                        statistic,
                        "normal_cdf_2t",
                        finite(&[("z", zv)]),
                        est,
                    ))
                }
                _ => Err(u()),
            }
        }
        _ => Err(u()),
    }
}
