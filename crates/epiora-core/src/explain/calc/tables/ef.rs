//! Etiologic/prevented fraction leaves (`ef_risk`/`ef_or`).
use super::*;
use crate::calc::counts::ef::{ef_or_parts, ef_risk_parts};

/// `etiologic`/`etiologic_or` — the percent-scaled fraction leaves.
/// `name` is the registry key (`efp`, `efe_or`, …); every shown
/// intermediate comes from `ef_risk_parts`/`ef_or_parts` — the
/// same helpers `ef_risk`/`ef_or` call.
pub(crate) fn ef_leaf(
    name: &str,
    t: &Table2x2,
    conf: f64,
    statistic: &str,
    side: &str,
    est: crate::stats::Estimate,
) -> Result<Explanation, EngineError> {
    let zz = z(conf);
    // The internals `ef_risk`/`ef_or` compute — the clamped
    // fractions and their bounds, with R's NaN-propagating clamp.
    let pr = ef_risk_parts(t, conf);
    let po = ef_or_parts(t, conf);

    let (fid, v) = match (name, side) {
        ("efp", "est") => ("ef_risk_efp", finite(&[("ro", pr.ro), ("ru", pr.ru)])),
        ("efp", "lower") => (
            "ef_risk_efp_lower",
            finite(&[("efp", pr.efp), ("var", pr.var_efp), ("z", zz)]),
        ),
        ("efp", "upper") => (
            "ef_risk_efp_upper",
            finite(&[("efp", pr.efp), ("var", pr.var_efp), ("z", zz)]),
        ),
        ("efe", "est") => ("ef_risk_efe", finite(&[("re", pr.re), ("ru", pr.ru)])),
        ("efe", "lower") => ("ef_risk_efe_lower", finite(&[("rr_lo", pr.rr_lo)])),
        ("efe", "upper") => ("ef_risk_efe_upper", finite(&[("rr_hi", pr.rr_hi)])),
        ("pfp", "est") => ("ef_risk_pfp", finite(&[("ru", pr.ru), ("ro", pr.ro)])),
        ("pfp", "lower") => ("ef_risk_pfp_lower", finite(&[("efp_hi", pr.efp_hi)])),
        ("pfp", "upper") => ("ef_risk_pfp_upper", finite(&[("efp_lo", pr.efp_lo)])),
        ("pfe", "est") => ("ef_risk_pfe", finite(&[("ru", pr.ru), ("re", pr.re)])),
        ("pfe", "lower") => ("ef_risk_pfe_lower", finite(&[("rr_hi", pr.rr_hi)])),
        ("pfe", "upper") => ("ef_risk_pfe_upper", finite(&[("rr_lo", pr.rr_lo)])),
        ("efp_or", "est") => ("ef_or_efpor", finite(&[("share", po.share), ("od", po.od)])),
        ("efp_or", "lower") => (
            "ef_or_efpor_lower",
            finite(&[("efpor", po.efpor), ("var", po.var_or), ("z", zz)]),
        ),
        ("efp_or", "upper") => (
            "ef_or_efpor_upper",
            finite(&[("efpor", po.efpor), ("var", po.var_or), ("z", zz)]),
        ),
        ("efe_or", "est") => ("ef_or_efeor", finite(&[("od", po.od)])),
        ("efe_or", "lower") => ("ef_or_efeor_lower", finite(&[("od_lo", po.od_lo)])),
        ("efe_or", "upper") => ("ef_or_efeor_upper", finite(&[("od_hi", po.od_hi)])),
        ("pfp_or", "est") => ("ef_or_pfpor", finite(&[("share", po.share), ("od", po.od)])),
        ("pfp_or", "lower") => ("ef_or_pfpor_lower", finite(&[("hi", po.efpor_hi)])),
        ("pfp_or", "upper") => ("ef_or_pfpor_upper", finite(&[("lo", po.efpor_lo)])),
        ("pfe_or", "est") => ("ef_or_pfeor", finite(&[("od", po.od)])),
        ("pfe_or", "lower") => ("ef_or_pfeor_lower", finite(&[("od_hi", po.od_hi)])),
        ("pfe_or", "upper") => ("ef_or_pfeor_upper", finite(&[("od_lo", po.od_lo)])),
        _ => return Err(unknown(statistic)),
    };
    Ok(closed(statistic, fid, v, est))
}

// --- two_by_two / two_by_n --------------------------------------------------
