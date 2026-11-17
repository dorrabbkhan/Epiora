//! Sample-size leaves (Kelsey/Fleiss/mean-difference/proportion).
use super::*;
use crate::calc::mean_diff;
use crate::calc::sample_size;
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
    let factor = sample_size::proportion::margin_factor(freq, margin_pct);

    let [blk, l] = segs else { return Err(u()) };
    let lv = level(&l.key).ok_or_else(u)?;
    let za = z(lv);
    // The run's per-level chain — `level_parts` in
    // `calc::sample_size::proportion`.
    let parts = sample_size::proportion::level_parts(deff, n_pop, p, d, factor, clusters, za);
    let (oe_real, n_raw, cs_real, cs) = (parts.oe_real, parts.n_raw, parts.cs_real, parts.cs);

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
/// — the run path's own `p1p2` helpers.
fn ss_p1p2(id: &str, inputs: &Value) -> (f64, f64) {
    if id == "ss_unmatched_cc" {
        sample_size::unmatched_cc::p1p2(inputs)
    } else {
        sample_size::cohort::p1p2(inputs)
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
    let vn = sample_size::two_props::vn(&m.key, p1, p2, r, za, zb);
    let tp = sample_size::two_props::block_parts(vn, r);
    let g2r = tp.g2r;

    let (fid, v) = match l.key.as_str() {
        k if k == g1 => ("ceil", finite(&[("x", vn)])),
        k if k == format!("{g1}_real") => (
            method_fid,
            finite(&[("za", za), ("zb", zb), ("p1", p1), ("p2", p2), ("r", r)]),
        ),
        k if k == g2 => ("ceil", finite(&[("x", g2r)])),
        k if k == format!("{g2}_real") => ("ss_group2_real", finite(&[("vn", vn), ("r", r)])),
        "total" => ("sum_two", finite(&[("a", tp.a), ("b", tp.b)])),
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
    let vn = sample_size::mean_difference::vn(s1, s2, ratio, za, zb, diff);
    let (n1, vn2, n2) = sample_size::mean_difference::n1_n2(vn, ratio);

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
