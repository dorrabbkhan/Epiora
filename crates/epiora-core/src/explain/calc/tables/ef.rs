//! Etiologic/prevented fraction leaves (`ef_risk`/`ef_or`).
use super::*;

/// `etiologic`/`etiologic_or` — the percent-scaled fraction leaves.
/// `name` is the registry key (`efp`, `efe_or`, …); `v` the
/// `[est, lower, upper]` `ef_risk`/`ef_or` produced.
pub(crate) fn ef_leaf(
    name: &str,
    t: &Table2x2,
    conf: f64,
    statistic: &str,
    side: &str,
    est: crate::stats::Estimate,
) -> Result<Explanation, EngineError> {
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (n1, n0, r1, r2, tt) = (a + b, c + d, a + c, b + d, a + b + c + d);
    let (re, ru, ro) = (a / n1, c / n0, r1 / tt);
    let rr = re / ru;
    let od = (a * d) / (b * c);
    let zz = z(conf);
    // The internals `ef_risk`/`ef_or` compute — the clamped
    // fractions and their bounds, with R's NaN-propagating clamp.
    let rclamp = |x: f64| {
        if x.is_nan() {
            x
        } else {
            x.clamp(-1.0, 1.0)
        }
    };
    let se_rr = libm::sqrt((1.0 - re) / (n1 * re) + (1.0 - ru) / (n0 * ru));
    let (rr_lo, rr_hi) = (rr * libm::exp(-zz * se_rr), rr * libm::exp(zz * se_rr));
    let se_or = libm::sqrt(1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d);
    let (od_lo, od_hi) = (od * libm::exp(-zz * se_or), od * libm::exp(zz * se_or));
    let var_efp = c * tt * (a * d * (tt - c) + b * c * c) / (r1 * r1 * r1 * n0 * n0 * n0);
    let efp = rclamp((ro - ru) / ro);
    // The bounds ef_risk emits: NaN propagates, else clamped.
    let efp_lo = {
        let v = efp - zz * libm::sqrt(var_efp);
        if v.is_nan() {
            v
        } else {
            v.max(-1.0)
        }
    };
    let efp_hi = {
        let v = efp + zz * libm::sqrt(var_efp);
        if v.is_nan() {
            v
        } else {
            v.min(1.0)
        }
    };
    let share = b / r2;
    let efpor = rclamp((share * (od - 1.0)) / (share * (od - 1.0) + 1.0));
    let vv = (c * r2) / (d * r1);
    let var_or = vv * vv * (a / (c * r1) + b / (d * r2));
    // The clamped efpor bounds ef_or emits (pfpor's limits use them).
    let efpor_lo = {
        let v = efpor - zz * libm::sqrt(var_or);
        if v.is_nan() {
            v
        } else {
            v.max(-1.0)
        }
    };
    let efpor_hi = {
        let v = efpor + zz * libm::sqrt(var_or);
        if v.is_nan() {
            v
        } else {
            v.min(1.0)
        }
    };

    let (fid, v) = match (name, side) {
        ("efp", "est") => ("ef_risk_efp", finite(&[("ro", ro), ("ru", ru)])),
        ("efp", "lower") => (
            "ef_risk_efp_lower",
            finite(&[("efp", efp), ("var", var_efp), ("z", zz)]),
        ),
        ("efp", "upper") => (
            "ef_risk_efp_upper",
            finite(&[("efp", efp), ("var", var_efp), ("z", zz)]),
        ),
        ("efe", "est") => ("ef_risk_efe", finite(&[("re", re), ("ru", ru)])),
        ("efe", "lower") => ("ef_risk_efe_lower", finite(&[("rr_lo", rr_lo)])),
        ("efe", "upper") => ("ef_risk_efe_upper", finite(&[("rr_hi", rr_hi)])),
        ("pfp", "est") => ("ef_risk_pfp", finite(&[("ru", ru), ("ro", ro)])),
        ("pfp", "lower") => ("ef_risk_pfp_lower", finite(&[("efp_hi", efp_hi)])),
        ("pfp", "upper") => ("ef_risk_pfp_upper", finite(&[("efp_lo", efp_lo)])),
        ("pfe", "est") => ("ef_risk_pfe", finite(&[("ru", ru), ("re", re)])),
        ("pfe", "lower") => ("ef_risk_pfe_lower", finite(&[("rr_hi", rr_hi)])),
        ("pfe", "upper") => ("ef_risk_pfe_upper", finite(&[("rr_lo", rr_lo)])),
        ("efp_or", "est") => ("ef_or_efpor", finite(&[("share", share), ("od", od)])),
        ("efp_or", "lower") => (
            "ef_or_efpor_lower",
            finite(&[("efpor", efpor), ("var", var_or), ("z", zz)]),
        ),
        ("efp_or", "upper") => (
            "ef_or_efpor_upper",
            finite(&[("efpor", efpor), ("var", var_or), ("z", zz)]),
        ),
        ("efe_or", "est") => ("ef_or_efeor", finite(&[("od", od)])),
        ("efe_or", "lower") => ("ef_or_efeor_lower", finite(&[("od_lo", od_lo)])),
        ("efe_or", "upper") => ("ef_or_efeor_upper", finite(&[("od_hi", od_hi)])),
        ("pfp_or", "est") => ("ef_or_pfpor", finite(&[("share", share), ("od", od)])),
        ("pfp_or", "lower") => ("ef_or_pfpor_lower", finite(&[("hi", efpor_hi)])),
        ("pfp_or", "upper") => ("ef_or_pfpor_upper", finite(&[("lo", efpor_lo)])),
        ("pfe_or", "est") => ("ef_or_pfeor", finite(&[("od", od)])),
        ("pfe_or", "lower") => ("ef_or_pfeor_lower", finite(&[("od_hi", od_hi)])),
        ("pfe_or", "upper") => ("ef_or_pfeor_upper", finite(&[("od_lo", od_lo)])),
        _ => return Err(unknown(statistic)),
    };
    Ok(closed(statistic, fid, v, est))
}

// --- two_by_two / two_by_n --------------------------------------------------
