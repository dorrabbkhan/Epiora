//! `adjusted` — the stratified block of `two_by_two`/
//! `two_by_n`: `mantelhaen.test(correct=FALSE)`'s chi-square and
//! RBG OR interval, the Greenland–Robins MH RR, the three directly
//! adjusted measures, Breslow–Day, and the stratified CMLE. Split
//! from `two_by_two.rs` in v1.0 Task 10 to keep both files under
//! the ~400-line budget. The `*_parts` helpers expose every
//! intermediate the explain traces name (spec §6); the run and
//! the trace share them.

use super::shared::*;
use crate::calc::counts::martin;
use crate::stats::Table2x2;
use serde_json::{json, Value};

/// `(e, w)` — one stratum's Woolf estimate and inverse-variance
/// weight for `measure` (`or`/`rr`/`rd`) — shared by
/// `direct_adjust`, `woolf` and the explain trace.
pub(crate) fn stratum_ew(t: &Table2x2, measure: &str) -> (f64, f64) {
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (n1, n0) = (a + b, c + d);
    match measure {
        "or" => (
            libm::log(a * d / (b * c)),
            1.0 / (1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d),
        ),
        "rr" => (
            libm::log((a / n1) / (c / n0)),
            1.0 / (b / (a * n1) + d / (c * n0)),
        ),
        _ => (
            a / n1 - c / n0,
            1.0 / (a * b / (n1 * n1 * n1) + c * d / (n0 * n0 * n0)),
        ),
    }
}

/// `woolf`'s pooled result — the explain trace's named values
/// (v1.0 Task 10, spec §6).
pub(crate) struct WoolfParts {
    /// `swe / sw` — the directly adjusted estimate (`ln` for
    /// `or`/`rr`).
    pub adj: f64,
    /// `sum(w)`.
    pub sw: f64,
    /// `sum(w·e)` — equals `adj·sw`; kept for the trace's
    /// `sum_we` named value.
    pub swe: f64,
    /// The per-stratum estimates.
    pub ests: Vec<f64>,
    /// The per-stratum weights.
    pub ws: Vec<f64>,
}

/// The per-stratum estimate/weight pairs `direct_adjust` and
/// `breslow_day` share, plus the pooled adjoint `sum(w·e)/sum(w)`.
pub(crate) fn woolf(strata: &[Table2x2], measure: &str) -> WoolfParts {
    let (mut sw, mut swe) = (0.0, 0.0);
    let mut ests = Vec::with_capacity(strata.len());
    let mut ws = Vec::with_capacity(strata.len());
    for t in strata {
        let (e, w) = stratum_ew(t, measure);
        ests.push(e);
        ws.push(w);
        sw += w;
        swe += w * e;
    }
    WoolfParts {
        adj: swe / sw,
        sw,
        swe,
        ests,
        ws,
    }
}

/// `sum(w·(e − adj)²)` — the Breslow–Day Woolf chi-square, shared
/// by `breslow_day` and the explain trace.
pub(crate) fn woolf_chisq(adj: f64, ests: &[f64], ws: &[f64]) -> f64 {
    ests.iter()
        .zip(ws)
        .map(|(e, w)| w * (e - adj) * (e - adj))
        .sum()
}

/// The directly adjusted `or`/`rr`/`rd` estimate and CI — the
/// inverse-variance pooled log scale (RD stays linear).
pub(crate) fn direct_adjust(strata: &[Table2x2], conf: f64, measure: &str) -> [f64; 3] {
    let zz = z(conf);
    let p = woolf(strata, measure);
    let half = zz / libm::sqrt(p.sw);
    if measure == "rd" {
        [p.adj, p.adj - half, p.adj + half]
    } else {
        [
            libm::exp(p.adj),
            libm::exp(p.adj - half),
            libm::exp(p.adj + half),
        ]
    }
}

/// `breslow_day` — Woolf homogeneity chi-squares for OR, RR and RD
/// around their directly adjusted estimates.
fn breslow_day(strata: &[Table2x2]) -> Value {
    let k = strata.len() as f64;
    let mut out = serde_json::Map::new();
    for measure in ["or", "rr", "rd"] {
        let p = woolf(strata, measure);
        let chi = woolf_chisq(p.adj, &p.ests, &p.ws);
        out.insert(format!("{measure}_chisq"), leaf(chi));
        out.insert(format!("{measure}_p"), leaf(chi_p(chi, k - 1.0)));
    }
    Value::Object(out)
}

/// The pooled sums of `adjusted` — the explain trace's named values
/// (v1.0 Task 10, spec §6). The run path calls `adjusted_parts` and
/// the explain path reuses it.
pub(crate) struct AdjustedParts {
    /// `sum(a − N1·M1/n)`.
    pub delta: f64,
    /// `sum(N1·N0·M1·M0 / (n²·(n−1)))`.
    pub var: f64,
    /// `sum(a·d/n)`.
    pub sd_: f64,
    /// `sum(b·c/n)`.
    pub so_: f64,
    /// `sum((a+d)·a·d/n²)`.
    pub rg_a: f64,
    /// `sum(((a+d)·b·c + (b+c)·a·d)/n²)`.
    pub rg_b: f64,
    /// `sum((b+c)·b·c/n²)`.
    pub rg_c: f64,
    /// `sum(a·N0/n)`.
    pub rr_num: f64,
    /// `sum(c·N1/n)`.
    pub rr_den: f64,
    /// `sum(M1·N1·N0/n² − a·c/n)`.
    pub gr_num: f64,
}

impl AdjustedParts {
    /// `delta²/var` — the uncorrected MH chi-square.
    pub(crate) fn mh_chi(&self) -> f64 {
        self.delta * self.delta / self.var
    }

    /// `sd_/so_` — the MH odds ratio.
    pub(crate) fn or(&self) -> f64 {
        self.sd_ / self.so_
    }

    /// The Robins–Breslow–Greenland SD of `log or`.
    pub(crate) fn sd_or(&self) -> f64 {
        libm::sqrt(
            self.rg_a / (2.0 * self.sd_ * self.sd_)
                + self.rg_b / (2.0 * self.sd_ * self.so_)
                + self.rg_c / (2.0 * self.so_ * self.so_),
        )
    }

    /// `rr_num/rr_den` — the Greenland–Robins MH risk ratio.
    pub(crate) fn rr(&self) -> f64 {
        self.rr_num / self.rr_den
    }

    /// `sqrt(gr_num/(rr_num·rr_den))` — the log-RR SE.
    pub(crate) fn gr_se(&self) -> f64 {
        libm::sqrt(self.gr_num / (self.rr_num * self.rr_den))
    }
}

/// The `adjusted` sums over `strata` (spec §6).
pub(crate) fn adjusted_parts(strata: &[Table2x2]) -> AdjustedParts {
    let (mut delta, mut var): (f64, f64) = (0.0, 0.0);
    let (mut sd_, mut so_, mut rg_a, mut rg_b, mut rg_c) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let (mut rr_num, mut rr_den, mut gr_num) = (0.0, 0.0, 0.0);
    for t in strata {
        let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
        let (n1, n0, m1, m0, n) = (a + b, c + d, a + c, b + d, a + b + c + d);
        delta += a - n1 * m1 / n;
        var += n1 * n0 * m1 * m0 / (n * n * (n - 1.0));
        sd_ += a * d / n;
        so_ += b * c / n;
        rg_a += (a + d) * a * d / (n * n);
        rg_b += ((a + d) * b * c + (b + c) * a * d) / (n * n);
        rg_c += (b + c) * b * c / (n * n);
        rr_num += a * n0 / n;
        rr_den += c * n1 / n;
        gr_num += m1 * n1 * n0 / (n * n) - a * c / n;
    }
    AdjustedParts {
        delta,
        var,
        sd_,
        so_,
        rg_a,
        rg_b,
        rg_c,
        rr_num,
        rr_den,
        gr_num,
    }
}

/// The `adjusted` block — `mantelhaen.test(correct=FALSE)`'s
/// chi-square and RBG OR interval, the Greenland–Robins MH RR, the
/// three directly adjusted measures, Breslow–Day, and the
/// stratified CMLE.
pub(crate) fn adjusted(strata: &[Table2x2], conf: f64) -> Value {
    let zz = z(conf);
    let p = adjusted_parts(strata);
    let mh_chi = p.mh_chi();
    let or = p.or();
    let sd = p.sd_or();
    let rr = p.rr();
    let gr_se = p.gr_se();
    // Owner-decision work guard (PR #74): the stratified CMLE leaves
    // go `too_large` before any polynomial is allocated.
    let cmle_or = if martin::too_large(strata) {
        let u = || undefined("too_large");
        json!({
            "est": u(),
            "midp": { "lower": u(), "upper": u() },
            "fisher": { "lower": u(), "upper": u() },
        })
    } else {
        let cmle = martin::stratified_cmle(strata, conf);
        json!({
            "est": leaf(cmle[0]),
            "midp": { "lower": leaf(cmle[1]), "upper": leaf(cmle[2]) },
            "fisher": { "lower": leaf(cmle[3]), "upper": leaf(cmle[4]) },
        })
    };
    json!({
        "mh_chisq": { "value": leaf(mh_chi), "p": leaf(chi_p(mh_chi, 1.0)) },
        "mh_or": stat_ci([or, or * libm::exp(-zz * sd), or * libm::exp(zz * sd)]),
        "mh_rr": stat_ci([rr, libm::exp(libm::log(rr) - zz * gr_se), libm::exp(libm::log(rr) + zz * gr_se)]),
        "direct_or": stat_ci(direct_adjust(strata, conf, "or")),
        "direct_rr": stat_ci(direct_adjust(strata, conf, "rr")),
        "direct_rd": stat_ci(direct_adjust(strata, conf, "rd")),
        "breslow_day": breslow_day(strata),
        "cmle_or": cmle_or,
    })
}
