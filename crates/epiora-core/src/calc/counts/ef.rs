//! `ef_risk`/`ef_or` — the OpenEpi etiologic and prevented
//! fractions of `tables.R`, split from `two_by_two.rs` in v1.0
//! Task 10 to keep both files under the ~400-line budget. The
//! `*_parts` functions expose every intermediate the explain
//! traces name (spec §6); the run and the trace share them.

use super::shared::z;
use crate::stats::Table2x2;

/// R's `max(-1, min(1, x))`: NaN passes through; Rust's
/// `clamp`/`min`/`max` would swallow it (they return the other
/// operand on NaN).
fn rclamp(x: f64) -> f64 {
    if x.is_nan() {
        x
    } else {
        x.clamp(-1.0, 1.0)
    }
}

/// `ef_risk`'s intermediates — the explain trace's named values
/// (v1.0 Task 10, spec §6).
pub(crate) struct EfRiskParts {
    /// `a/n1` — risk in the exposed.
    pub re: f64,
    /// `c/n0` — risk in the unexposed.
    pub ru: f64,
    /// `r1/tt` — overall risk.
    pub ro: f64,
    /// `rr·exp(−z·se_rr)`.
    pub rr_lo: f64,
    /// `rr·exp(z·se_rr)`.
    pub rr_hi: f64,
    /// The efp variance term of `tables.R`.
    pub var_efp: f64,
    /// Clamped `(ro−ru)/ro`.
    pub efp: f64,
    /// `efp − z·√var_efp`, NaN-propagating, floored at −1.
    pub efp_lo: f64,
    /// `efp + z·√var_efp`, NaN-propagating, capped at 1.
    pub efp_hi: f64,
    /// Clamped `(rr−1)/rr`.
    pub efe: f64,
    /// `(ru−ro)/ru`.
    pub pfp: f64,
    /// `(ru−re)/ru`.
    pub pfe: f64,
}

/// The intermediates of `ef_risk(t, conf)` — shared by the result
/// and the explain trace.
pub(crate) fn ef_risk_parts(t: &Table2x2, conf: f64) -> EfRiskParts {
    let zz = z(conf);
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (n1, n0, r1, tt) = (a + b, c + d, a + c, a + b + c + d);
    let (re, ru, ro) = (a / n1, c / n0, r1 / tt);
    let rr = re / ru;
    let se_rr = libm::sqrt((1.0 - re) / (n1 * re) + (1.0 - ru) / (n0 * ru));
    let (rr_lo, rr_hi) = (rr * libm::exp(-zz * se_rr), rr * libm::exp(zz * se_rr));
    let efp = rclamp((ro - ru) / ro);
    let efe = rclamp((rr - 1.0) / rr);
    let pfp = (ru - ro) / ru;
    let pfe = (ru - re) / ru;
    let var_efp = c * tt * (a * d * (tt - c) + b * c * c) / (r1 * r1 * r1 * n0 * n0 * n0);
    let efp_lo = if (efp - zz * libm::sqrt(var_efp)).is_nan() {
        f64::NAN
    } else {
        (efp - zz * libm::sqrt(var_efp)).max(-1.0)
    };
    let efp_hi = if (efp + zz * libm::sqrt(var_efp)).is_nan() {
        f64::NAN
    } else {
        (efp + zz * libm::sqrt(var_efp)).min(1.0)
    };
    EfRiskParts {
        re,
        ru,
        ro,
        rr_lo,
        rr_hi,
        var_efp,
        efp,
        efp_lo,
        efp_hi,
        efe,
        pfp,
        pfe,
    }
}

/// `ef_risk` — the OpenEpi risk-based etiologic/prevented
/// fractions (`fixtures/R/methods/tables.R`).
pub(crate) fn ef_risk(t: &Table2x2, conf: f64) -> [[f64; 3]; 4] {
    let p = ef_risk_parts(t, conf);
    [
        [p.efp, p.efp_lo, p.efp_hi],
        [
            p.efe,
            rclamp((p.rr_lo - 1.0) / p.rr_lo),
            rclamp((p.rr_hi - 1.0) / p.rr_hi),
        ],
        [
            p.pfp,
            1.0 - 1.0 / (1.0 - p.efp_hi),
            1.0 - 1.0 / (1.0 - p.efp_lo),
        ],
        [p.pfe, 1.0 - p.rr_hi, 1.0 - p.rr_lo],
    ]
}

/// `ef_or`'s intermediates — the explain trace's named values
/// (v1.0 Task 10, spec §6).
pub(crate) struct EfOrParts {
    /// `a·d/(b·c)` — the odds ratio.
    pub od: f64,
    /// `od·exp(−z·se_or)`.
    pub od_lo: f64,
    /// `od·exp(z·se_or)`.
    pub od_hi: f64,
    /// `b/r2` — the exposed share of the un-diseased.
    pub share: f64,
    /// `vv²·(a/(c·r1) + b/(d·r2))`, `vv = (c·r2)/(d·r1)`.
    pub var_or: f64,
    /// Clamped `share·(od−1)/(share·(od−1) + 1)`.
    pub efpor: f64,
    /// `efpor − z·√var_or`, NaN-propagating, floored at −1.
    pub efpor_lo: f64,
    /// `efpor + z·√var_or`, NaN-propagating, capped at 1.
    pub efpor_hi: f64,
    /// Clamped `(od−1)/od`.
    pub efeor: f64,
    /// `share·(1−od)`.
    pub pfpor: f64,
    /// `1−od`.
    pub pfeor: f64,
}

/// The intermediates of `ef_or(t, conf)` — shared by the result
/// and the explain trace.
pub(crate) fn ef_or_parts(t: &Table2x2, conf: f64) -> EfOrParts {
    let zz = z(conf);
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (r1, r2) = (a + c, b + d);
    let od = (a * d) / (b * c);
    let se_or = libm::sqrt(1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d);
    let (od_lo, od_hi) = (od * libm::exp(-zz * se_or), od * libm::exp(zz * se_or));
    let share = b / r2;
    let efpor = rclamp((share * (od - 1.0)) / (share * (od - 1.0) + 1.0));
    let efeor = rclamp((od - 1.0) / od);
    let pfpor = share * (1.0 - od);
    let pfeor = 1.0 - od;
    let vv = (c * r2) / (d * r1);
    let var_or = vv * vv * (a / (c * r1) + b / (d * r2));
    let efpor_lo = {
        let raw = efpor - zz * libm::sqrt(var_or);
        if raw.is_nan() {
            raw
        } else {
            raw.max(-1.0)
        }
    };
    let efpor_hi = {
        let raw = efpor + zz * libm::sqrt(var_or);
        if raw.is_nan() {
            raw
        } else {
            raw.min(1.0)
        }
    };
    EfOrParts {
        od,
        od_lo,
        od_hi,
        share,
        var_or,
        efpor,
        efpor_lo,
        efpor_hi,
        efeor,
        pfpor,
        pfeor,
    }
}

/// `ef_or` — the OR-derived fractions.
pub(crate) fn ef_or(t: &Table2x2, conf: f64) -> [[f64; 3]; 4] {
    let p = ef_or_parts(t, conf);
    [
        [p.efpor, p.efpor_lo, p.efpor_hi],
        [
            p.efeor,
            rclamp((p.od_lo - 1.0) / p.od_lo),
            rclamp((p.od_hi - 1.0) / p.od_hi),
        ],
        [
            p.pfpor,
            1.0 - 1.0 / (1.0 - p.efpor_hi),
            1.0 - 1.0 / (1.0 - p.efpor_lo),
        ],
        [p.pfeor, 1.0 - p.od_hi, 1.0 - p.od_lo],
    ]
}
