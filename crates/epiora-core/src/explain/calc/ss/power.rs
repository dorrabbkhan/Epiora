//! Power leaves (proportion comparisons and mean difference).
use super::*;
use crate::calc::mean_diff;

/// `power_unmatched_cc`/`power_cohort`'s `powerz` — the run's
/// Fleiss §3.18 inversion (`cc` adjusts `n1` first).
fn powerz(id: &str, inputs: &Value, cc: bool) -> (f64, f64, f64, f64) {
    let za = z(conf(inputs));
    let (p1_key, p2_key, n1_key, n2_key) = if id == "power_unmatched_cc" {
        (
            "percent_cases_exposed",
            "percent_controls_exposed",
            "n_cases",
            "n_controls",
        )
    } else {
        (
            "percent_exposed_outcome",
            "percent_unexposed_outcome",
            "n_exposed",
            "n_unexposed",
        )
    };
    let p1 = num(inputs, p1_key) / 100.0;
    let p2 = num(inputs, p2_key) / 100.0;
    let n1 = cnt(inputs, n1_key) as f64;
    let n2 = cnt(inputs, n2_key) as f64;
    let r = n2 / n1;

    let q1 = 1.0 - p1;
    let q2 = 1.0 - p2;
    let d = p1 - p2;
    let (pbar, denom) = if id == "power_unmatched_cc" {
        ((p1 + r * p2) / (1.0 + r), libm::sqrt(p1 * q1 + p2 * q2 / r))
    } else {
        (
            (n1 * p1 + n2 * p2) / (n1 + n2),
            libm::sqrt(p2 * q2 + p1 * q1 / r),
        )
    };
    let qbar = 1.0 - pbar;

    // The continuity correction adjusts `n1` by ±(r+1)/(r·d) on the
    // implied measure's direction — the run's `nn1`.
    let nn1 = if !cc {
        n1
    } else {
        let measure = if id == "power_unmatched_cc" {
            (p1 * q2) / (p2 * q1)
        } else {
            p1 / p2
        };
        if measure > 1.0 {
            n1 - (r + 1.0) / (r * d)
        } else {
            n1 + (r + 1.0) / (r * d)
        }
    };
    let pz =
        (libm::sqrt(nn1.abs() * d * d) - za * libm::sqrt((1.0 + 1.0 / r) * pbar * qbar)) / denom;
    (pz, p1, p2, d)
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
            let (pz, _, _, _) = powerz(id, inputs, l.key == "normal_cc");
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
    let r = n2 / n1;
    let d = mean_diff::difference(inputs);
    let za = z(conf(inputs));
    let powerz = -za + libm::sqrt(n1) * d / libm::sqrt(s1 * s1 + s2 * s2 / r);

    match segs {
        [l] if l.key == "power" => Ok(closed(
            statistic,
            "power_pct",
            finite(&[("powerz", powerz)]),
            est,
        )),
        _ => Err(u()),
    }
}
