//! Sample-size leaves (Kelsey/Fleiss/mean-difference/proportion).
use super::*;
use crate::calc::mean_diff;
use crate::special::normal_quantile;

pub(crate) fn ss_proportion(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let n_pop = cnt(inputs, "population_size") as f64;
    let freq = num(inputs, "expected_freq");
    let margin_pct = num(inputs, "margin_error");
    let d = margin_pct / 100.0;
    let deff = num(inputs, "design_effect");
    let clusters = cnt(inputs, "clusters") as f64;
    let p = freq / 100.0;
    let factor = freq * (100.0 - freq) / (margin_pct * margin_pct);

    let [blk, l] = segs else { return Err(u()) };
    let lv = level(&l.key).ok_or_else(u)?;
    let za = z(lv);
    let za2 = za * za;
    let oe_real = deff * n_pop * p * (1.0 - p) / ((d * d / za2) * (n_pop - 1.0) + p * (1.0 - p));
    let n0 = za2 * factor;
    let n_raw = libm::rint(n0 / (1.0 + n0 / n_pop));
    let cs_real = deff * n_raw / clusters;
    let cs = cs_real.ceil();

    let (fid, v) = match blk.key.as_str() {
        "n_openepi" => ("ceil", finite(&[("x", oe_real)])),
        "n_openepi_real" => (
            "ss_prop_openepi",
            finite(&[
                ("deff", deff),
                ("n_pop", n_pop),
                ("p", p),
                ("d", d),
                ("za", za),
            ]),
        ),
        "n_ei7" | "cluster_size" => ("ceil", finite(&[("x", cs_real)])),
        "n_ei7_real" | "cluster_size_real" => (
            "ss_prop_cluster_real",
            finite(&[("deff", deff), ("n_raw", n_raw), ("clusters", clusters)]),
        ),
        "total" => (
            "ss_prop_total",
            finite(&[("cluster_size", cs), ("clusters", clusters)]),
        ),
        _ => return Err(u()),
    };
    Ok(closed(statistic, fid, v, est))
}

/// The `p1`/`p2` proportions each two-proportion calculator derives
/// — `ss_unmatched_cc`'s OR inversion and `ss_cohort`'s four forms.
fn ss_p1p2(id: &str, inputs: &Value) -> (f64, f64) {
    if id == "ss_unmatched_cc" {
        let p2 = num(inputs, "percent_controls_exposed") / 100.0;
        let p1 = inputs
            .get("percent_cases_exposed")
            .and_then(Value::as_f64)
            .map_or_else(
                || {
                    let or = num(inputs, "odds_ratio");
                    p2 * or / (1.0 + p2 * (or - 1.0))
                },
                |pct| pct / 100.0,
            );
        (p1, p2)
    } else {
        let p2 = num(inputs, "percent_unexposed_outcome") / 100.0;
        let p1 = if let Some(pct) = inputs
            .get("percent_exposed_outcome")
            .and_then(Value::as_f64)
        {
            pct / 100.0
        } else if let Some(or) = inputs.get("odds_ratio").and_then(Value::as_f64) {
            p2 * or / (1.0 + p2 * (or - 1.0))
        } else if let Some(rr) = inputs.get("risk_ratio").and_then(Value::as_f64) {
            p2 * rr
        } else {
            p2 + num(inputs, "risk_difference") / 100.0
        };
        (p1, p2)
    }
}

/// `two_props`' `vn` per method — the run's formulas verbatim.
fn ss_vn(method: &str, p1: f64, p2: f64, r: f64, za: f64, zb: f64) -> f64 {
    if (0.0..=1.0).contains(&p1) && p1 == p2 {
        return f64::INFINITY;
    }
    let p1 = if (0.0..=1.0).contains(&p1) {
        p1
    } else {
        f64::NAN
    };
    let pbar = (p1 + r * p2) / (1.0 + r);
    let qbar = 1.0 - pbar;
    let d = p1 - p2;
    let dd = d * d;
    match method {
        "kelsey" => (za + zb) * (za + zb) * pbar * qbar * (r + 1.0) / (dd * r),
        "fleiss" => {
            let fnum = za * libm::sqrt((r + 1.0) * pbar * qbar)
                + zb * libm::sqrt(r * p1 * (1.0 - p1) + p2 * (1.0 - p2));
            fnum * fnum / (r * dd)
        }
        _ => {
            let fnum = za * libm::sqrt((r + 1.0) * pbar * qbar)
                + zb * libm::sqrt(r * p1 * (1.0 - p1) + p2 * (1.0 - p2));
            let vf = fnum * fnum / (r * dd);
            vf * libm::pow(
                1.0 + libm::sqrt(1.0 + 2.0 * (r + 1.0) / (vf * r * d.abs())),
                2.0,
            ) / 4.0
        }
    }
}

pub(crate) fn ss_two_props(
    id: &str,
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let (p1, p2) = ss_p1p2(id, inputs);
    let r = num(
        inputs,
        if id == "ss_unmatched_cc" {
            "ratio_controls_per_case"
        } else {
            "ratio_unexposed_per_exposed"
        },
    );
    let za = z(conf(inputs));
    let zb = normal_quantile(num(inputs, "power") / 100.0).unwrap_or(f64::NAN);

    // The two group-key names of this calculator's methods.
    let (g1, g2) = if id == "ss_unmatched_cc" {
        ("cases", "controls")
    } else {
        ("exposed", "unexposed")
    };

    let [m, l] = segs else { return Err(u()) };
    let method_fid = match m.key.as_str() {
        "kelsey" => "ss_kelsey",
        "fleiss" => "ss_fleiss",
        "fleiss_cc" => "ss_fleiss_cc",
        _ => return Err(u()),
    };
    let vn = ss_vn(&m.key, p1, p2, r, za, zb);
    let g2r = vn * r;

    let (fid, v) = match l.key.as_str() {
        k if k == g1 => ("ceil", finite(&[("x", vn)])),
        k if k == format!("{g1}_real") => (
            method_fid,
            finite(&[("za", za), ("zb", zb), ("p1", p1), ("p2", p2), ("r", r)]),
        ),
        k if k == g2 => ("ceil", finite(&[("x", g2r)])),
        k if k == format!("{g2}_real") => ("ss_group2_real", finite(&[("vn", vn), ("r", r)])),
        "total" => ("sum_two", finite(&[("a", vn.ceil()), ("b", g2r.ceil())])),
        "total_real" => ("sum_two", finite(&[("a", vn), ("b", g2r)])),
        _ => return Err(u()),
    };
    Ok(closed(statistic, fid, v, est))
}

pub(crate) fn ss_mean_difference(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let (s1, s2) = mean_diff::sds(inputs)?;
    let ratio = num(inputs, "ratio");
    let diff = mean_diff::difference(inputs);
    let za = z(conf(inputs));
    let zb = normal_quantile(num(inputs, "power") / 100.0).unwrap_or(f64::NAN);
    let vn = (s1 * s1 + s2 * s2 / ratio) * (za + zb) * (za + zb) / (diff * diff);
    let n1 = vn.ceil();
    let vn2 = n1 * ratio;
    let n2 = vn2.ceil();

    let [l] = segs else { return Err(u()) };
    let (fid, v) = match l.key.as_str() {
        "n1" => ("ceil", finite(&[("x", vn)])),
        "n1_real" => (
            "ss_mean_diff",
            finite(&[
                ("s1", s1),
                ("s2", s2),
                ("ratio", ratio),
                ("za", za),
                ("zb", zb),
                ("diff", diff),
            ]),
        ),
        "n2" => ("ceil", finite(&[("x", vn2)])),
        "n2_real" => ("ss_mean_diff_n2", finite(&[("n1", n1), ("ratio", ratio)])),
        "total" => ("sum_two", finite(&[("a", n1), ("b", n2)])),
        "total_real" => ("sum_two", finite(&[("a", vn), ("b", vn2)])),
        _ => return Err(u()),
    };
    Ok(closed(statistic, fid, v, est))
}
