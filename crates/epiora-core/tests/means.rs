#![cfg(feature = "corpus")]

use epiora_core::corpus::compare::compare;
use epiora_core::corpus::{load_dataset_dir, run_case};
use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_core::procedures::{means, MeansOptions, RunContext};
use epiora_core::stats::{Estimate, UndefinedReason};
use epiora_core::validation::statistic_ids;
use proptest::prelude::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".to_string(),
        project: None,
    }
}

fn datasets() -> BTreeMap<String, Dataset> {
    load_dataset_dir(&fixtures_root().join("data")).expect("corpus datasets")
}

fn load_fixtures(dir: &str) -> Vec<(String, Value)> {
    let mut out: Vec<(String, Value)> = std::fs::read_dir(fixtures_root().join(dir))
        .unwrap_or_else(|e| panic!("read fixtures/{dir}: {e}"))
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| {
            let id = format!("{dir}/{}", p.file_stem().expect("stem").to_string_lossy());
            let fixture = serde_json::from_str(&std::fs::read_to_string(&p).expect("read fixture"))
                .expect("parse fixture");
            (id, fixture)
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn make_dataset(columns: Vec<Column>) -> Dataset {
    Dataset::new("t".to_string(), 1, columns).expect("dataset")
}

fn col(name: &str, data: ColumnData) -> Column {
    Column {
        name: name.to_string(),
        data,
    }
}

fn undefined(e: &Estimate) -> Option<UndefinedReason> {
    match e {
        Estimate::Undefined { reason } => Some(*reason),
        Estimate::Value(v) => panic!("expected Undefined, got {v}"),
    }
}

#[test]
fn every_means_fixture_matches() {
    let sets = datasets();
    let mut failures = Vec::new();
    let mut cases = 0;
    for (id, fixture) in load_fixtures("means") {
        cases += 1;
        match run_case(&fixture, &sets) {
            Ok(output) => match compare(&fixture, &output.actual) {
                Ok(mismatches) => {
                    for m in mismatches {
                        failures.push(format!("{id}: {m}"));
                    }
                }
                Err(e) => failures.push(format!("{id}: {e}")),
            },
            Err(e) => failures.push(format!("{id}: run_case failed: {e}")),
        }
    }
    assert!(cases > 0, "no means fixtures found");
    assert!(
        failures.is_empty(),
        "{} failures in {cases} cases:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Review Focus 2: all-missing, constant and single-observation data
/// give `Undefined` statistics — never an error, NaN or a panic.
#[test]
fn degenerate_columns_are_undefined_not_errors() {
    let sets = datasets();
    let all_missing = sets.get("means_all_missing").expect("dataset");
    let out = means(
        all_missing,
        &MeansOptions {
            variable: "value".to_string(),
            group_by: Some("grp".to_string()),
            include_missing: false,
        },
        &ctx(),
    )
    .expect("all-missing run must not error");
    assert_eq!(out.result.overall.observations, 0);
    for e in [
        &out.result.overall.mean,
        &out.result.overall.variance,
        &out.result.overall.std_dev,
        &out.result.overall.min,
        &out.result.overall.p25,
        &out.result.overall.median,
        &out.result.overall.p75,
        &out.result.overall.max,
        &out.result.overall.mode,
    ] {
        assert_eq!(undefined(e), Some(UndefinedReason::TooFew), "{e:?}");
    }
    assert!(out.result.t_test.is_none());
    assert!(out.result.anova.is_none());
    assert!(out.result.bartlett.is_none());
    assert!(out.result.kruskal_wallis.is_none());

    let constant = sets.get("means_constant").expect("dataset");
    let out = means(
        constant,
        &MeansOptions {
            variable: "value".to_string(),
            group_by: Some("grp".to_string()),
            include_missing: false,
        },
        &ctx(),
    )
    .expect("constant run must not error");
    for g in &out.result.groups {
        assert_eq!(g.variance, Estimate::Value(0.0));
        assert_eq!(g.std_dev, Estimate::Value(0.0));
    }
    let anova = out.result.anova.as_ref().expect("anova present");
    assert_eq!(undefined(&anova.f), Some(UndefinedReason::ZeroVariance));
    assert_eq!(
        undefined(&anova.p_value),
        Some(UndefinedReason::ZeroVariance)
    );
    let bartlett = out.result.bartlett.as_ref().expect("bartlett present");
    assert_eq!(
        undefined(&bartlett.statistic),
        Some(UndefinedReason::ZeroVariance)
    );
    let kw = out
        .result
        .kruskal_wallis
        .as_ref()
        .expect("kruskal_wallis present");
    assert_eq!(undefined(&kw.h), Some(UndefinedReason::ZeroVariance));

    let one_obs = sets.get("means_one_obs_group").expect("dataset");
    let out = means(
        one_obs,
        &MeansOptions {
            variable: "value".to_string(),
            group_by: Some("grp".to_string()),
            include_missing: false,
        },
        &ctx(),
    )
    .expect("one-obs-group run must not error");
    let singleton = out
        .result
        .groups
        .iter()
        .find(|g| g.observations == 1)
        .expect("a 1-observation group");
    assert_eq!(
        undefined(&singleton.variance),
        Some(UndefinedReason::TooFew)
    );
    assert_eq!(undefined(&singleton.std_dev), Some(UndefinedReason::TooFew));
    let anova = out.result.anova.as_ref().expect("anova present");
    assert_eq!(undefined(&anova.ss_within), Some(UndefinedReason::TooFew));
    assert!(out.result.bartlett.is_none());
}

#[test]
fn text_variable_is_not_numeric() {
    let ds = make_dataset(vec![
        col(
            "t",
            ColumnData::Text(vec![Some("a".into()), Some("b".into())]),
        ),
        col("v", ColumnData::Real(vec![Some(1.0), Some(2.0)])),
        col("b", ColumnData::Boolean(vec![Some(true), Some(false)])),
    ]);
    for variable in ["t", "b"] {
        let err = means(
            &ds,
            &MeansOptions {
                variable: variable.to_string(),
                group_by: None,
                include_missing: false,
            },
            &ctx(),
        )
        .expect_err("non-numeric variable must error");
        assert_eq!(err.code(), "not_numeric_variable", "{variable}");
    }
    let err = means(
        &ds,
        &MeansOptions {
            variable: "nope".to_string(),
            group_by: None,
            include_missing: false,
        },
        &ctx(),
    )
    .expect_err("unknown variable must error");
    assert_eq!(err.code(), "unknown_variable");
}

#[test]
fn group_by_101_levels_is_too_many_groups() {
    let n: i64 = 101;
    let ds = make_dataset(vec![
        col(
            "v",
            ColumnData::Real((0..n).map(|i| Some(i as f64)).collect()),
        ),
        col(
            "g",
            ColumnData::Text((0..n).map(|i| Some(format!("g{i:03}"))).collect()),
        ),
    ]);
    let err = means(
        &ds,
        &MeansOptions {
            variable: "v".to_string(),
            group_by: Some("g".to_string()),
            include_missing: false,
        },
        &ctx(),
    )
    .expect_err("101 levels must error");
    assert_eq!(err.code(), "too_many_groups");
}

#[test]
fn exclusions_counted() {
    let ds = make_dataset(vec![
        col(
            "value",
            ColumnData::Real(vec![Some(1.0), None, Some(3.0), Some(4.0), None]),
        ),
        col(
            "grp",
            ColumnData::Text(vec![
                Some("A".into()),
                Some("A".into()),
                None,
                Some("B".into()),
                Some("A".into()),
            ]),
        ),
    ]);
    let out = means(
        &ds,
        &MeansOptions {
            variable: "value".to_string(),
            group_by: Some("grp".to_string()),
            include_missing: false,
        },
        &ctx(),
    )
    .expect("means");
    assert_eq!(out.result.exclusions.get("value"), Some(&2));
    assert_eq!(out.result.exclusions.get("grp"), Some(&1));
    assert_eq!(out.receipt.exclusions, out.result.exclusions);

    let out = means(
        &ds,
        &MeansOptions {
            variable: "value".to_string(),
            group_by: Some("grp".to_string()),
            include_missing: true,
        },
        &ctx(),
    )
    .expect("means include_missing");
    assert_eq!(out.result.exclusions.get("value"), Some(&2));
    assert_eq!(out.result.exclusions.get("grp"), Some(&0));
    let missing_group = out
        .result
        .groups
        .iter()
        .find(|g| g.label == "(missing)")
        .expect("(missing) group listed");
    assert_eq!(missing_group.observations, 1);
    assert_eq!(missing_group.mean, Estimate::Value(3.0));
}

#[test]
fn dates_mean_is_days() {
    let ds = make_dataset(vec![col(
        "onset",
        ColumnData::Date(vec![Some(-2), Some(1), Some(4), Some(9)]),
    )]);
    let out = means(
        &ds,
        &MeansOptions {
            variable: "onset".to_string(),
            group_by: None,
            include_missing: false,
        },
        &ctx(),
    )
    .expect("means on dates");
    assert_eq!(out.result.overall.mean, Estimate::Value(3.0));
    assert_eq!(out.result.overall.min, Estimate::Value(-2.0));
}

/// `neumaier_sum` over 1,000,000 seeded reals equals the sum of the
/// same values sorted ascending within 1e-12 relative (the plan's
/// `sum_is_order_stable_at_1m`; seed 0xE910A fixed here).
#[test]
fn sum_is_order_stable_at_1m() {
    let mut state = 0xE910A_u64;
    let mut rng = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let m = (state >> 11) as f64 / 9007199254740992.0;
        m * 2.0e6 - 1.0e6
    };
    let values: Vec<f64> = (0..1_000_000).map(|_| rng()).collect();
    let mut sorted = values.clone();
    sorted.sort_by(f64::total_cmp);
    let data_order = epiora_core::stats::neumaier_sum(values.iter().copied());
    let ascending = epiora_core::stats::neumaier_sum(sorted.iter().copied());
    assert!(
        (data_order - ascending).abs() <= 1e-12 * ascending.abs(),
        "data-order sum {data_order} vs ascending {ascending}"
    );
}

#[test]
fn receipt_lists_every_means_statistic() {
    let sets = datasets();
    let ds = sets.get("means_ties").expect("dataset");
    let out = means(
        ds,
        &MeansOptions {
            variable: "value".to_string(),
            group_by: Some("grp".to_string()),
            include_missing: false,
        },
        &ctx(),
    )
    .expect("means");
    let mut v = serde_json::to_value(&out.result).expect("serialize result");
    let map = v.as_object_mut().expect("result object");
    map.remove("exclusions");
    map.insert(
        "total".to_string(),
        serde_json::Value::from(ds.row_count() as u64),
    );
    map.insert(
        "missing".to_string(),
        serde_json::Value::from(ds.row_count() as u64 - out.result.overall.observations),
    );
    let emitted: BTreeSet<String> = statistic_ids(&v)
        .into_iter()
        .map(|p| format!("means.{p}"))
        .collect();
    let listed: BTreeSet<String> = out
        .receipt
        .validation
        .iter()
        .map(|e| e.statistic.clone())
        .collect();
    assert_eq!(emitted, listed, "receipt validation vs emitted ids");
}

fn arb_dataset() -> impl Strategy<Value = Dataset> {
    (
        proptest::collection::vec((-1e3f64..1e3f64).prop_map(Some), 0..60usize),
        proptest::collection::vec(0usize..4, 0..60usize),
    )
        .prop_map(|(values, groups)| {
            let n = values.len();
            let grp: Vec<Option<String>> = groups
                .into_iter()
                .chain(std::iter::repeat(0))
                .take(n)
                .map(|g| Some(format!("g{g}")))
                .collect();
            make_dataset(vec![
                col("v", ColumnData::Real(values)),
                col("g", ColumnData::Text(grp)),
            ])
        })
}

proptest! {
    /// Every p-value a grouped MEANS emits lies in [0, 1]
    /// (`p_values_in_unit_interval`).
    #[test]
    fn p_values_in_unit_interval(ds in arb_dataset()) {
        let out = means(
            &ds,
            &MeansOptions {
                variable: "v".to_string(),
                group_by: Some("g".to_string()),
                include_missing: false,
            },
            &ctx(),
        );
        prop_assume!(out.is_ok());
        let out = out.expect("means");
        let mut ps = Vec::new();
        if let Some(t) = &out.result.t_test {
            ps.push(&t.pooled.p_value);
            ps.push(&t.welch.p_value);
        }
        if let Some(a) = &out.result.anova {
            ps.push(&a.p_value);
        }
        if let Some(b) = &out.result.bartlett {
            ps.push(&b.p_value);
        }
        if let Some(k) = &out.result.kruskal_wallis {
            ps.push(&k.p_value);
        }
        for p in ps {
            if let Estimate::Value(v) = p {
                prop_assert!((0.0..=1.0).contains(v), "p = {v}");
            }
        }
    }

    /// A t-test's 95% CI brackets its reported difference
    /// (`ci_contains_diff`).
    #[test]
    fn ci_contains_diff(ds in arb_dataset()) {
        let out = means(
            &ds,
            &MeansOptions {
                variable: "v".to_string(),
                group_by: Some("g".to_string()),
                include_missing: false,
            },
            &ctx(),
        );
        prop_assume!(out.is_ok());
        let out = out.expect("means");
        if let Some(t) = &out.result.t_test {
            for tt in [&t.pooled, &t.welch] {
                if let (Estimate::Value(lo), Estimate::Value(d), Estimate::Value(hi)) = (
                    &tt.diff_ci_95.lower,
                    &tt.diff,
                    &tt.diff_ci_95.upper,
                ) {
                    prop_assert!(lo <= d && d <= hi, "CI [{lo}, {hi}] does not contain {d}");
                }
            }
        }
    }

    /// `ss_total = ss_between + ss_within` within 1e-9 relative
    /// (`anova_ss_total_equals_between_plus_within`).
    #[test]
    fn anova_ss_total_equals_between_plus_within(ds in arb_dataset()) {
        let out = means(
            &ds,
            &MeansOptions {
                variable: "v".to_string(),
                group_by: Some("g".to_string()),
                include_missing: false,
            },
            &ctx(),
        );
        prop_assume!(out.is_ok());
        let out = out.expect("means");
        if let Some(a) = &out.result.anova {
            if let (Estimate::Value(t), Estimate::Value(b), Estimate::Value(w)) =
                (&a.ss_total, &a.ss_between, &a.ss_within)
            {
                prop_assert!(
                    (t - (b + w)).abs() <= 1e-9 * (b + w).abs() || (t - (b + w)).abs() <= 1e-9,
                    "ss_total {t} != ss_between {b} + ss_within {w}"
                );
            }
        }
    }
}
