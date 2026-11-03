#![cfg(feature = "corpus")]

use epiora_core::stats::{rxc, two_by_two, Ci, Estimate, Table2x2};
use proptest::prelude::*;
use serde_json::Value;

fn val(e: &Estimate) -> Option<f64> {
    match e {
        Estimate::Value(v) => Some(*v),
        Estimate::Undefined { .. } => None,
    }
}

fn table() -> impl Strategy<Value = Table2x2> {
    (0u64..=500, 0u64..=500, 0u64..=500, 0u64..=500).prop_map(|(a, b, c, d)| Table2x2 {
        a,
        b,
        c,
        d,
    })
}

/// True when every leaf of the serialized result is a finite number or
/// a `{"undefined": "<string>"}` leaf — never NaN, null or bare junk.
fn clean_leaves(v: &Value) -> bool {
    match v {
        Value::Object(m) => {
            if let Some(u) = m.get("undefined") {
                m.len() == 1 && u.as_str().is_some_and(|s| !s.is_empty())
            } else {
                m.values().all(clean_leaves)
            }
        }
        Value::Array(items) => items.iter().all(clean_leaves),
        Value::Number(n) => n.as_f64().is_some_and(f64::is_finite),
        _ => false,
    }
}

/// `est` lies inside `ci` when all three are defined (1e-12 slack for
/// floating-point end effects).
fn ci_contains(ci: &Ci, est: &Estimate) -> bool {
    match (val(&ci.lower), val(est), val(&ci.upper)) {
        (Some(lo), Some(e), Some(hi)) => {
            let slack = 1e-12 * e.abs().max(1.0);
            lo - slack <= e && e <= hi + slack
        }
        _ => true,
    }
}

proptest! {
    #[test]
    fn or_reciprocal(t in table()) {
        let fwd = two_by_two(&t);
        let rev = two_by_two(&Table2x2 { a: t.b, b: t.a, c: t.d, d: t.c });
        if let (Some(x), Some(y)) = (val(&fwd.odds_ratio.estimate), val(&rev.odds_ratio.estimate)) {
            prop_assert!(
                (x * y - 1.0).abs() <= 1e-9,
                "OR {} x inverted {} = {} != 1",
                x, y, x * y
            );
        }
    }

    #[test]
    fn defined_cis_contain_their_estimates(t in table()) {
        let s = two_by_two(&t);
        prop_assert!(ci_contains(&s.risk.difference_ci_95_wald, &s.risk.difference));
        prop_assert!(ci_contains(&s.risk_ratio.ci_95_wald, &s.risk_ratio.estimate));
        let ss_ci = Ci {
            lower: s.risk_ratio.small_sample.lower.clone(),
            upper: s.risk_ratio.small_sample.upper.clone(),
        };
        prop_assert!(ci_contains(&ss_ci, &s.risk_ratio.small_sample.estimate));
        prop_assert!(ci_contains(&s.odds_ratio.ci_95_wald, &s.odds_ratio.estimate));
        prop_assert!(ci_contains(&s.odds_ratio.ci_95_fisher_exact, &s.odds_ratio.cmle));
        prop_assert!(ci_contains(&s.odds_ratio.ci_95_midp_exact, &s.odds_ratio.cmle));
    }

    #[test]
    fn p_values_in_unit_interval(t in table()) {
        let s = two_by_two(&t);
        for p in [
            &s.fisher_exact.p_lower,
            &s.fisher_exact.p_upper,
            &s.fisher_exact.p_two_sided,
            &s.midp_exact.p_two_sided,
            &s.chi_square.uncorrected.p_value,
            &s.chi_square.yates.p_value,
            &s.chi_square.mantel_haenszel.p_value,
        ] {
            if let Some(v) = val(p) {
                prop_assert!((0.0..=1.0).contains(&v), "p-value {v} outside [0, 1]");
            }
        }
    }

    #[test]
    fn chi_square_statistics_nonnegative(t in table()) {
        let s = two_by_two(&t);
        for stat in [
            &s.chi_square.uncorrected.statistic,
            &s.chi_square.yates.statistic,
            &s.chi_square.mantel_haenszel.statistic,
        ] {
            if let Some(v) = val(stat) {
                prop_assert!(v >= 0.0, "chi-square {v} < 0");
            }
        }
    }

    #[test]
    fn fisher_two_sided_ge_min_one_sided(t in table()) {
        let s = two_by_two(&t);
        if let (Some(lo), Some(hi), Some(two)) = (
            val(&s.fisher_exact.p_lower),
            val(&s.fisher_exact.p_upper),
            val(&s.fisher_exact.p_two_sided),
        ) {
            prop_assert!(
                two + 1e-12 >= lo.min(hi),
                "two-sided {two} < min({lo}, {hi})"
            );
        }
    }

    #[test]
    fn serialized_output_has_no_nan(t in table()) {
        let s = two_by_two(&t);
        let v = serde_json::to_value(&s).expect("serialize TwoByTwoStats");
        prop_assert!(clean_leaves(&v), "non-finite or non-leaf value in {v}");
    }
}

#[test]
fn rxc_low_expected_flag() {
    // e_00 = 11*11/121 = 1 < 5
    assert!(rxc(&[vec![1, 10], vec![10, 100]]).low_expected);
    // every expected count is 10
    assert!(!rxc(&[vec![10, 10], vec![10, 10]]).low_expected);
}
