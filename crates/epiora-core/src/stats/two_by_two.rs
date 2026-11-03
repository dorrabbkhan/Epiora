//! `two_by_two` — the closed-form and exact statistics of one 2×2
//! table `[[a,b],[c,d]]` (rows exposed/unexposed, columns ill/well),
//! serializing to the Task 2 `two_by_two` fixture layout (spec §3.5).
//! Closed-form parts match `epitools`' `riskratio`/`riskratio.small`/
//! `oddsratio(method = "wald")`, `chisq.test` and the `mantelhaen.test`
//! port; the exact parts live in `exact.rs`. No 0.5 correction is ever
//! applied to an estimate; an empty table or zero margin gives
//! `Undefined`, never NaN.

use super::estimate::{Ci, Estimate, UndefinedReason};
use super::exact;
use crate::special::{chisq_sf, normal_quantile, NcHypergeom};
use serde::Serialize;

/// One 2×2 table: rows exposed/unexposed, columns ill/well.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Table2x2 {
    /// Exposed and ill.
    pub a: u64,
    /// Exposed and well.
    pub b: u64,
    /// Unexposed and ill.
    pub c: u64,
    /// Unexposed and well.
    pub d: u64,
}

/// Serialized 2×2 statistics — the `expected` key layout of the
/// `two_by_two` fixtures.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TwoByTwoStats {
    /// Attack rates and the risk difference with its Wald CI.
    pub risk: RiskStats,
    /// Risk ratio with Wald and small-sample confidence intervals.
    pub risk_ratio: RiskRatioStats,
    /// Odds ratio with Wald, Fisher exact and mid-p exact intervals
    /// plus the conditional MLE.
    pub odds_ratio: OddsRatioStats,
    /// Pearson, Yates-corrected and Mantel–Haenszel (N−1) chi-squares.
    pub chi_square: ChiSquareStats,
    /// Fisher exact p-values.
    pub fisher_exact: FisherExactStats,
    /// Mid-p exact two-sided p-value.
    pub midp_exact: MidpExactStats,
}

/// `risk.*` — risks in each row and their difference.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RiskStats {
    /// `a / (a+b)`.
    pub exposed: Estimate,
    /// `c / (c+d)`.
    pub unexposed: Estimate,
    /// `exposed − unexposed`.
    pub difference: Estimate,
    /// Wald 95% CI on the difference.
    pub difference_ci_95_wald: Ci,
}

/// `risk_ratio.*`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RiskRatioStats {
    /// `exposed / unexposed`.
    pub estimate: Estimate,
    /// Wald 95% CI on the log scale.
    pub ci_95_wald: Ci,
    /// `epitools::riskratio.small`: adjusted estimate with the Wald CI
    /// of the unadjusted table.
    pub small_sample: SmallSample,
}

/// `risk_ratio.small_sample` — flat `estimate`/`lower`/`upper` as the
/// fixture spells it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SmallSample {
    /// `(a/(a+b)) / ((c+1)/(c+d+1))`.
    pub estimate: Estimate,
    /// Lower 95% bound.
    pub lower: Estimate,
    /// Upper 95% bound.
    pub upper: Estimate,
}

/// `odds_ratio.*`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OddsRatioStats {
    /// Cross-product `ad / bc`.
    pub estimate: Estimate,
    /// Conditional MLE (`fisher.test`).
    pub cmle: Estimate,
    /// Wald 95% CI on the log scale.
    pub ci_95_wald: Ci,
    /// Fisher exact 95% CI (`fisher.test`).
    pub ci_95_fisher_exact: Ci,
    /// Mid-p exact 95% CI (epitools `or.midp`).
    pub ci_95_midp_exact: Ci,
}

/// `chi_square.*`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChiSquareStats {
    /// Degrees of freedom — a plain number when defined.
    pub df: Estimate,
    /// Pearson, uncorrected.
    pub uncorrected: ChiSquare,
    /// Yates continuity-corrected.
    pub yates: ChiSquare,
    /// Mantel–Haenszel with the N−1 variance.
    pub mantel_haenszel: ChiSquare,
}

/// One chi-square statistic and its upper-tail p-value.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChiSquare {
    /// The statistic.
    pub statistic: Estimate,
    /// `pchisq(statistic, df, lower.tail = FALSE)`.
    pub p_value: Estimate,
}

/// `fisher_exact.*`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FisherExactStats {
    /// `P(X ≤ a | margins, psi = 1)`.
    pub p_lower: Estimate,
    /// `P(X ≥ a | margins, psi = 1)`.
    pub p_upper: Estimate,
    /// Sum of table probabilities ≤ observed × (1 + 1e-7).
    pub p_two_sided: Estimate,
}

/// `midp_exact.*`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MidpExactStats {
    /// `2 * min(p, 1 − p)` on the mid-p tail.
    pub p_two_sided: Estimate,
}

impl TwoByTwoStats {
    /// Every leaf `Undefined { reason }` — the empty-table result.
    fn all_undefined(reason: UndefinedReason) -> TwoByTwoStats {
        let u = || Estimate::Undefined { reason };
        let ci = || Ci {
            lower: u(),
            upper: u(),
        };
        let stat = || ChiSquare {
            statistic: u(),
            p_value: u(),
        };
        TwoByTwoStats {
            risk: RiskStats {
                exposed: u(),
                unexposed: u(),
                difference: u(),
                difference_ci_95_wald: ci(),
            },
            risk_ratio: RiskRatioStats {
                estimate: u(),
                ci_95_wald: ci(),
                small_sample: SmallSample {
                    estimate: u(),
                    lower: u(),
                    upper: u(),
                },
            },
            odds_ratio: OddsRatioStats {
                estimate: u(),
                cmle: u(),
                ci_95_wald: ci(),
                ci_95_fisher_exact: ci(),
                ci_95_midp_exact: ci(),
            },
            chi_square: ChiSquareStats {
                df: u(),
                uncorrected: stat(),
                yates: stat(),
                mantel_haenszel: stat(),
            },
            fisher_exact: FisherExactStats {
                p_lower: u(),
                p_upper: u(),
                p_two_sided: u(),
            },
            midp_exact: MidpExactStats { p_two_sided: u() },
        }
    }
}

/// `qnorm(0.975)`; `normal_quantile` is `Some` for every p in (0, 1),
/// the fallback is the same correctly-rounded constant.
fn z_975() -> f64 {
    normal_quantile(0.975).unwrap_or(1.959963984540054)
}

fn chi(statistic: f64) -> ChiSquare {
    ChiSquare {
        statistic: Estimate::from_f64(statistic, UndefinedReason::ZeroMargin),
        p_value: if statistic.is_finite() {
            Estimate::from_f64(
                chisq_sf(statistic, 1.0).clamp(0.0, 1.0),
                UndefinedReason::ZeroMargin,
            )
        } else {
            Estimate::Undefined {
                reason: UndefinedReason::ZeroMargin,
            }
        },
    }
}

/// All statistics of `t` (spec §3.5). A table of total 0 gives
/// `Undefined { EmptyTable }` everywhere; zero margins give
/// `Undefined` per statistic, never NaN.
pub fn two_by_two(t: &Table2x2) -> TwoByTwoStats {
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let n1 = a + b;
    let n0 = c + d;
    let n = n1 + n0;
    if n == 0.0 {
        return TwoByTwoStats::all_undefined(UndefinedReason::EmptyTable);
    }
    let m1 = a + c;
    let m0 = b + d;
    let z = z_975();
    let zc = UndefinedReason::ZeroCell;
    let zm = UndefinedReason::ZeroMargin;

    let pe = a / n1;
    let pu = c / n0;
    let exposed = Estimate::from_f64(pe, zm);
    let unexposed = Estimate::from_f64(pu, zm);

    let rd = pe - pu;
    let se_rd = libm::sqrt(pe * (1.0 - pe) / n1 + pu * (1.0 - pu) / n0);
    let risk = RiskStats {
        exposed,
        unexposed,
        difference: Estimate::from_f64(rd, zm),
        difference_ci_95_wald: Ci {
            lower: Estimate::from_f64(rd - z * se_rd, zm),
            upper: Estimate::from_f64(rd + z * se_rd, zm),
        },
    };

    let rr = pe / pu;
    let se_rr = libm::sqrt(1.0 / a - 1.0 / n1 + 1.0 / c - 1.0 / n0);
    let log_rr = libm::log(rr);
    // epitools::riskratio.small: the reference-group count and margin
    // get +1; the CI keeps the unadjusted Wald standard error.
    let rr_small = pe / ((c + 1.0) / (n0 + 1.0));
    let log_rr_small = libm::log(rr_small);
    let risk_ratio = RiskRatioStats {
        estimate: Estimate::from_f64(rr, zc),
        ci_95_wald: Ci {
            lower: Estimate::from_f64(libm::exp(log_rr - z * se_rr), zc),
            upper: Estimate::from_f64(libm::exp(log_rr + z * se_rr), zc),
        },
        small_sample: SmallSample {
            estimate: Estimate::from_f64(rr_small, zc),
            lower: Estimate::from_f64(libm::exp(log_rr_small - z * se_rr), zc),
            upper: Estimate::from_f64(libm::exp(log_rr_small + z * se_rr), zc),
        },
    };

    let or = a * d / (b * c);
    let se_or = libm::sqrt(1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d);
    let log_or = libm::log(or);
    // Margins in u64 for the conditional support; saturating so an
    // absurd input can never panic (rule: no input panics the engine).
    let (m1u, m0u, ku) = (
        t.a.saturating_add(t.c),
        t.b.saturating_add(t.d),
        t.a.saturating_add(t.b),
    );
    let h = NcHypergeom::new(m1u, m0u, ku);
    let odds_ratio = OddsRatioStats {
        estimate: Estimate::from_f64(or, zc),
        cmle: exact::cmle(&h, t.a),
        ci_95_wald: Ci {
            lower: Estimate::from_f64(libm::exp(log_or - z * se_or), zc),
            upper: Estimate::from_f64(libm::exp(log_or + z * se_or), zc),
        },
        ci_95_fisher_exact: exact::fisher_ci(&h, t.a),
        ci_95_midp_exact: exact::midp_ci(&h, t.a),
    };

    let delta = (a - n1 * m1 / n).abs();
    let e = [n1 * m1 / n, n1 * m0 / n, n0 * m1 / n, n0 * m0 / n];
    let o = [a, b, c, d];
    let chi_square = if e.iter().all(|&ei| ei > 0.0) {
        let uncorrected: f64 = o
            .iter()
            .zip(&e)
            .map(|(oi, ei)| (oi - ei) * (oi - ei) / ei)
            .sum();
        let yates_adj = 0.5_f64.min(delta);
        let yates: f64 = o
            .iter()
            .zip(&e)
            .map(|(oi, ei)| ((oi - ei).abs() - yates_adj) * ((oi - ei).abs() - yates_adj) / ei)
            .sum();
        // The mantelhaen.test port: DELTA² / (prod(margins) / (n² (n−1))).
        let mh = delta * delta * n * n * (n - 1.0) / (n1 * n0 * m1 * m0);
        ChiSquareStats {
            df: Estimate::Value(1.0),
            uncorrected: chi(uncorrected),
            yates: chi(yates),
            mantel_haenszel: chi(mh),
        }
    } else {
        ChiSquareStats {
            df: Estimate::Value(1.0),
            uncorrected: chi(f64::NAN),
            yates: chi(f64::NAN),
            mantel_haenszel: chi(f64::NAN),
        }
    };

    let p_lower = h.lower_tail(t.a, 1.0).clamp(0.0, 1.0);
    let p_upper = h.upper_tail(t.a, 1.0).clamp(0.0, 1.0);
    let fisher_exact = FisherExactStats {
        p_lower: Estimate::from_f64(p_lower, zm),
        p_upper: Estimate::from_f64(p_upper, zm),
        p_two_sided: Estimate::from_f64(exact::fisher_p_two_sided(&h, m1u, m0u, ku, t.a), zm),
    };

    let midp_exact = MidpExactStats {
        p_two_sided: Estimate::from_f64(exact::midp_p_two_sided(&h, t.a), zm),
    };

    TwoByTwoStats {
        risk,
        risk_ratio,
        odds_ratio,
        chi_square,
        fisher_exact,
        midp_exact,
    }
}
