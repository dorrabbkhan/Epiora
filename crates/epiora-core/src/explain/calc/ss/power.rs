//! Power leaves (proportion comparisons and mean difference).
use super::*;
use crate::calc::mean_diff;
use crate::calc::power::parts::{
    cc_inputs, cc_n1, cohort_inputs, group_ratio, or_cc, pooled_cc, pooled_cohort, powerz,
    powerz_mean, rr_cohort,
};

/// `power_unmatched_cc`/`power_cohort`'s `powerz` — the run's
/// Fleiss §3.18 inversion through `calc::power::parts` (`cc`
/// adjusts `n1` first).
fn powerz_of(id: &str, inputs: &Value, cc: bool) -> f64 {
    let za = z(conf(inputs));
    if id == "power_unmatched_cc" {
        let (p1, p2, n1, n2) = cc_inputs(inputs);
        let r = group_ratio(n1, n2);
        let (pbar, denom) = pooled_cc(p1, p2, r);
        let nn1 = if cc {
            cc_n1(n1, r, p1, p2, or_cc(p1, p2) > 1.0)
        } else {
            n1
        };
        powerz(nn1, p1, p2, r, za, pbar, denom)
    } else {
        let (p1, p2, n1, n2) = cohort_inputs(inputs);
        let r = group_ratio(n1, n2);
        let (pbar, denom) = pooled_cohort(p1, p2, n1, n2, r);
        let nn1 = if cc {
            cc_n1(n1, r, p1, p2, rr_cohort(p1, p2) > 1.0)
        } else {
            n1
        };
        powerz(nn1, p1, p2, r, za, pbar, denom)
    }
}

pub(crate) fn power_props(
    id: &str,
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;

    let [l] = segs else { return Err(u()) };
    match l.key.as_str() {
        "normal" | "normal_cc" => {
            let pz = powerz_of(id, inputs, l.key == "normal_cc");
            Ok(closed(
                statistic,
                "power_pct",
                finite(&[("powerz", pz)]),
                est,
            ))
        }
        "odds_ratio" => {
            let p1 = num(inputs, "percent_cases_exposed") / 100.0;
            let p2 = num(inputs, "percent_controls_exposed") / 100.0;
            Ok(closed(
                statistic,
                "or_from_props",
                finite(&[("p1", p1), ("p2", p2)]),
                est,
            ))
        }
        "risk_ratio" => {
            let p1 = num(inputs, "percent_exposed_outcome") / 100.0;
            let p2 = num(inputs, "percent_unexposed_outcome") / 100.0;
            Ok(closed(
                statistic,
                "risk_ratio_props",
                finite(&[("p1", p1), ("p2", p2)]),
                est,
            ))
        }
        "risk_difference" => {
            let p1 = num(inputs, "percent_exposed_outcome") / 100.0;
            let p2 = num(inputs, "percent_unexposed_outcome") / 100.0;
            Ok(closed(
                statistic,
                "risk_diff_props",
                finite(&[("p1", p1), ("p2", p2)]),
                est,
            ))
        }
        _ => Err(u()),
    }
}

pub(crate) fn power_mean_difference(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let (s1, s2) = mean_diff::sds(inputs)?;
    let n1 = cnt(inputs, "n_1") as f64;
    let n2 = cnt(inputs, "n_2") as f64;
    let r = group_ratio(n1, n2);
    let d = mean_diff::difference(inputs);
    let za = z(conf(inputs));
    let pz = powerz_mean(n1, r, d, s1, s2, za);

    match segs {
        [l] if l.key == "power" => Ok(closed(
            statistic,
            "power_pct",
            finite(&[("powerz", pz)]),
            est,
        )),
        _ => Err(u()),
    }
}
