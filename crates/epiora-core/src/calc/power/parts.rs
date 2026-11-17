//! `powerz` — the Fleiss §3.18 power inversion shared by
//! `power_unmatched_cc`, `power_cohort`, `power_mean_difference`
//! and the explain traces (v1.0 Task 10, spec §6): each run and
//! each trace calls these helpers, so the shown intermediates are
//! the run's own values.

use crate::calc::counts::shared::{cnt, num};
use serde_json::Value;

/// `(p̄, denom)` — `power_unmatched_cc`'s pooled proportion and
/// standard error, `p̄ = (p1 + r·p2)/(1 + r)`,
/// `denom = √(p1·q1 + p2·q2/r)`.
pub(crate) fn pooled_cc(p1: f64, p2: f64, r: f64) -> (f64, f64) {
    let (q1, q2) = (1.0 - p1, 1.0 - p2);
    ((p1 + r * p2) / (1.0 + r), libm::sqrt(p1 * q1 + p2 * q2 / r))
}

/// `(p̄, denom)` — `power_cohort`'s count-weighted pooled
/// proportion and standard error, `p̄ = (n1·p1 + n2·p2)/(n1+n2)`,
/// `denom = √(p2·q2 + p1·q1/r)`.
pub(crate) fn pooled_cohort(p1: f64, p2: f64, n1: f64, n2: f64, r: f64) -> (f64, f64) {
    let (q1, q2) = (1.0 - p1, 1.0 - p2);
    (
        (n1 * p1 + n2 * p2) / (n1 + n2),
        libm::sqrt(p2 * q2 + p1 * q1 / r),
    )
}

/// `powerz = (√(|n1|·d²) − zα·√((1+1/r)·p̄·q̄)) / denom` with
/// `d = p1 − p2` — the Fleiss §3.18 z-argument; `n1` is the
/// (possibly cc-adjusted) group-1 size.
pub(crate) fn powerz(n1: f64, p1: f64, p2: f64, r: f64, za: f64, pbar: f64, denom: f64) -> f64 {
    let d = p1 - p2;
    (libm::sqrt(n1.abs() * d * d) - za * libm::sqrt((1.0 + 1.0 / r) * pbar * (1.0 - pbar))) / denom
}

/// `p1·q2/(p2·q1)` — `power_unmatched_cc`'s implied odds ratio,
/// whose direction signs the continuity correction.
pub(crate) fn or_cc(p1: f64, p2: f64) -> f64 {
    (p1 * (1.0 - p2)) / (p2 * (1.0 - p1))
}

/// `p1/p2` — `power_cohort`'s implied risk ratio, whose direction
/// signs the continuity correction.
pub(crate) fn rr_cohort(p1: f64, p2: f64) -> f64 {
    p1 / p2
}

/// `p1 − p2` — `power_cohort`'s implied risk difference.
pub(crate) fn rd_cohort(p1: f64, p2: f64) -> f64 {
    p1 - p2
}

/// `n1 ∓ (r+1)/(r·d)` with `d = p1 − p2` — the Fleiss-CC `n1`
/// adjustment; `gt1` is whether the implied measure exceeds 1.
pub(crate) fn cc_n1(n1: f64, r: f64, p1: f64, p2: f64, gt1: bool) -> f64 {
    let d = p1 - p2;
    if gt1 {
        n1 - (r + 1.0) / (r * d)
    } else {
        n1 + (r + 1.0) / (r * d)
    }
}

/// `power_mean_difference`'s `−zα + √n1·d/√(s1² + s2²/r)`.
pub(crate) fn powerz_mean(n1: f64, r: f64, d: f64, s1: f64, s2: f64, za: f64) -> f64 {
    -za + libm::sqrt(n1) * d / libm::sqrt(s1 * s1 + s2 * s2 / r)
}

/// The `(p1, p2, n1, n2)` inputs of `power_unmatched_cc`, decoded
/// as the run does.
pub(crate) fn cc_inputs(inputs: &Value) -> (f64, f64, f64, f64) {
    (
        num(inputs, "percent_cases_exposed") / 100.0,
        num(inputs, "percent_controls_exposed") / 100.0,
        cnt(inputs, "n_cases") as f64,
        cnt(inputs, "n_controls") as f64,
    )
}

/// The `(p1, p2, n1, n2)` inputs of `power_cohort`, decoded as the
/// run does.
pub(crate) fn cohort_inputs(inputs: &Value) -> (f64, f64, f64, f64) {
    (
        num(inputs, "percent_exposed_outcome") / 100.0,
        num(inputs, "percent_unexposed_outcome") / 100.0,
        cnt(inputs, "n_exposed") as f64,
        cnt(inputs, "n_unexposed") as f64,
    )
}

/// `n2/n1` — the group-size ratio `r` the power formulas use. Shared
/// by the run paths and the explain trace (v1.0 Task 10, spec §6).
pub(crate) fn group_ratio(n1: f64, n2: f64) -> f64 {
    n2 / n1
}
