//! `table_leaf` — one 2×2 block's whole leaf surface.
use super::*;
use crate::calc::counts::shared::{self};

/// The per-table leaves both calculators share (`strata[i].*` and
/// `crude.*`); `which` is the first segment of the leaf path.
pub(crate) fn table_leaf(
    t: &Table2x2,
    conf: f64,
    statistic: &str,
    segs: &[Segment],
    est: crate::stats::Estimate,
    is_2n: bool,
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let [pe, pu, rr, rr_se, rd, rd_se, or_, or_se] = shared::wald_parts(t);
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (n1, n0) = (a + b, c + d);
    let zz = z(conf);

    match segs {
        [f, which, l] if f.key == "chisq" => chi_leaf(t, statistic, &which.key, &l.key, est),
        [f, l] if f.key == "fisher_exact" || f.key == "midp_exact" => {
            let fid = if f.key == "fisher_exact" {
                "method.fisher_exact_p"
            } else {
                "method.midp_p"
            };
            match l.key.as_str() {
                "p_1t" | "p_2t" => Ok(method(statistic, fid, cells(t), est)),
                _ => Err(u()),
            }
        }
        [f, l] if f.key == "exact" && is_2n => match l.key.as_str() {
            "fisher_1t" | "fisher_2t" => {
                Ok(method(statistic, "method.fisher_exact_p", cells(t), est))
            }
            "midp_2t" => Ok(method(statistic, "method.midp_p", cells(t), est)),
            _ => Err(u()),
        },
        [f, r, l] if f.key == "risk" => {
            let (x, n) = match r.key.as_str() {
                "exposed" => (t.a, n1 as u64),
                "unexposed" => (t.c, n0 as u64),
                "overall" => (t.a + t.c, (n1 + n0) as u64),
                _ => return Err(u()),
            };
            risk_leaf(x, n, conf, statistic, &l.key, est)
        }
        [f, l] if f.key == "risk_ratio" => match l.key.as_str() {
            "est" => Ok(closed(
                statistic,
                "risk_ratio",
                finite(&[("p1", pe), ("p0", pu)]),
                est,
            )),
            _ => stat_ci_wald_exp(statistic, &l.key, rr, rr_se, conf, est),
        },
        [f, l] if f.key == "risk_difference" => match l.key.as_str() {
            "est" => Ok(closed(
                statistic,
                "risk_difference_pct",
                finite(&[("p1", pe), ("p0", pu)]),
                est,
            )),
            "lower" => Ok(closed(
                statistic,
                "wald_lower_pct",
                finite(&[("est", rd), ("se", rd_se), ("z", zz)]),
                est,
            )),
            "upper" => Ok(closed(
                statistic,
                "wald_upper_pct",
                finite(&[("est", rd), ("se", rd_se), ("z", zz)]),
                est,
            )),
            _ => Err(u()),
        },
        [f, l] if f.key == "odds_ratio" => match l.key.as_str() {
            "est" => Ok(closed(statistic, "odds_ratio", cells(t), est)),
            _ => stat_ci_wald_exp(statistic, &l.key, or_, or_se, conf, est),
        },
        [f, m, l] if f.key == "cmle_or" || f.key == "odds_ratio_mle" => {
            match (m.key.as_str(), l.key.as_str()) {
                ("midp", "est") => Ok(method(statistic, "method.cmle", cells(t), est)),
                ("midp", "lower") | ("midp", "upper") => Ok(method(
                    statistic,
                    "method.midp_ci",
                    cells_conf(t, conf),
                    est,
                )),
                ("fisher", "lower") | ("fisher", "upper") => Ok(method(
                    statistic,
                    "method.fisher_ci",
                    cells_conf(t, conf),
                    est,
                )),
                _ => Err(u()),
            }
        }
        [f, name, l] if f.key == "etiologic" || f.key == "etiologic_or" => {
            ef_leaf(&name.key, t, conf, statistic, &l.key, est)
        }
        _ => Err(u()),
    }
}
