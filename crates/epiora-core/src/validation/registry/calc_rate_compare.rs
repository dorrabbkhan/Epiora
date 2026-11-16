//! `calc` registry coverage for `rate_compare` (v1.0 Task 8): every
//! leaf the calculator emits, mapped to the Task 2 fixtures that
//! cover it. `adjusted.*` exists only in multi-stratum fixtures —
//! `stratified` is the one that contains it.

use super::super::ToleranceClass::{ClosedForm as CF, Iterative as IT};
use super::super::{cov, Coverage};

const ALL: &[&str] = &[
    "calc/rate_compare/boundary",
    "calc/rate_compare/large",
    "calc/rate_compare/large_counts",
    "calc/rate_compare/one_inside_boundary",
    "calc/rate_compare/stratified",
    "calc/rate_compare/support_max",
    "calc/rate_compare/textbook",
    "calc/rate_compare/zero_event",
];

const ADJ: &[&str] = &["calc/rate_compare/stratified"];

/// One `rate_compare_table` block's coverage, prefixed `crude.` or
/// `strata[].`.
macro_rules! table_rows {
    ($p:expr) => {
        [
            cov(concat!($p, ".zscore.value"), ALL, CF),
            cov(concat!($p, ".zscore.p_1t"), ALL, CF),
            cov(concat!($p, ".zscore.p_2t"), ALL, CF),
            cov(concat!($p, ".rate.exposed.est"), ALL, CF),
            cov(concat!($p, ".rate.exposed.lower"), ALL, CF),
            cov(concat!($p, ".rate.exposed.upper"), ALL, CF),
            cov(concat!($p, ".rate.unexposed.est"), ALL, CF),
            cov(concat!($p, ".rate.unexposed.lower"), ALL, CF),
            cov(concat!($p, ".rate.unexposed.upper"), ALL, CF),
            cov(concat!($p, ".rate.overall.est"), ALL, CF),
            cov(concat!($p, ".rate.overall.lower"), ALL, CF),
            cov(concat!($p, ".rate.overall.upper"), ALL, CF),
            cov(concat!($p, ".rate_ratio.est"), ALL, CF),
            cov(concat!($p, ".rate_ratio.lower"), ALL, CF),
            cov(concat!($p, ".rate_ratio.upper"), ALL, CF),
            cov(concat!($p, ".rate_difference.est"), ALL, CF),
            cov(concat!($p, ".rate_difference.lower"), ALL, CF),
            cov(concat!($p, ".rate_difference.upper"), ALL, CF),
            cov(concat!($p, ".cmle_rr.est"), ALL, IT),
            cov(concat!($p, ".cmle_rr.midp_lower"), ALL, IT),
            cov(concat!($p, ".cmle_rr.midp_upper"), ALL, IT),
            cov(concat!($p, ".cmle_rr.fisher_lower"), ALL, IT),
            cov(concat!($p, ".cmle_rr.fisher_upper"), ALL, IT),
            cov(concat!($p, ".etiologic.efp.est"), ALL, CF),
            cov(concat!($p, ".etiologic.efp.lower"), ALL, CF),
            cov(concat!($p, ".etiologic.efp.upper"), ALL, CF),
            cov(concat!($p, ".etiologic.efe.est"), ALL, CF),
            cov(concat!($p, ".etiologic.efe.lower"), ALL, CF),
            cov(concat!($p, ".etiologic.efe.upper"), ALL, CF),
            cov(concat!($p, ".etiologic.pfp.est"), ALL, CF),
            cov(concat!($p, ".etiologic.pfp.lower"), ALL, CF),
            cov(concat!($p, ".etiologic.pfp.upper"), ALL, CF),
            cov(concat!($p, ".etiologic.pfe.est"), ALL, CF),
            cov(concat!($p, ".etiologic.pfe.lower"), ALL, CF),
            cov(concat!($p, ".etiologic.pfe.upper"), ALL, CF),
        ]
    };
}

const CRUDE: &[Coverage] = &table_rows!("calc.rate_compare.crude");
const STRATA: &[Coverage] = &table_rows!("calc.rate_compare.strata[]");

const ADJUSTED: &[Coverage] = &[
    cov("calc.rate_compare.adjusted.zscore.value", ADJ, CF),
    cov("calc.rate_compare.adjusted.zscore.p", ADJ, CF),
    cov("calc.rate_compare.adjusted.direct_rr.est", ADJ, CF),
    cov("calc.rate_compare.adjusted.direct_rr.lower", ADJ, CF),
    cov("calc.rate_compare.adjusted.direct_rr.upper", ADJ, CF),
    cov("calc.rate_compare.adjusted.mh_rr.est", ADJ, CF),
    cov("calc.rate_compare.adjusted.mh_rr.lower", ADJ, CF),
    cov("calc.rate_compare.adjusted.mh_rr.upper", ADJ, CF),
    cov("calc.rate_compare.adjusted.direct_rd.est", ADJ, CF),
    cov("calc.rate_compare.adjusted.direct_rd.lower", ADJ, CF),
    cov("calc.rate_compare.adjusted.direct_rd.upper", ADJ, CF),
    cov("calc.rate_compare.adjusted.breslow_day.rr_chisq", ADJ, CF),
    cov("calc.rate_compare.adjusted.breslow_day.rr_p", ADJ, CF),
    cov("calc.rate_compare.adjusted.breslow_day.rd_chisq", ADJ, CF),
    cov("calc.rate_compare.adjusted.breslow_day.rd_p", ADJ, CF),
];

const LEN: usize = CRUDE.len() + STRATA.len() + ADJUSTED.len();
const fn join<const N: usize>(parts: &[&'static [Coverage]]) -> [Coverage; N] {
    let mut out = [Coverage {
        statistic: "",
        fixtures: &[],
        class: CF,
    }; N];
    let (mut i, mut f) = (0, 0);
    while f < parts.len() {
        let mut j = 0;
        while j < parts[f].len() {
            out[i] = parts[f][j];
            i += 1;
            j += 1;
        }
        f += 1;
    }
    out
}

pub(super) static ENTRIES: &[Coverage] = &join::<LEN>(&[CRUDE, STRATA, ADJUSTED]);
