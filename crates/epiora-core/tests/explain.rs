//! Explain-trace tests (v1.0 Task 10, spec §6): registry coverage,
//! bit-identical final steps, declared formula ids, method-kind
//! classification, the `results_unchanged` corpus parity golden and
//! `ExplainUnknown`. Every corpus case runs once and each emitted
//! statistic traces off that output — the sweep is shared through
//! `explain_support` (fix round 3: previously every statistic
//! re-ran the whole procedure or calculator).
#![cfg(feature = "corpus")]

mod explain_support;

use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_core::explain::{
    explain_calculator, explain_procedure, ExplainKind, Explanation, FORMULAS,
};
use epiora_core::procedures::{means, FreqOptions, MeansOptions, ProportionCi, RunContext};
use epiora_core::stats::{Estimate, UndefinedReason};
use epiora_core::validation::{ToleranceClass, REGISTRY};
use epiora_core::EngineError;
use explain_support::{
    assert_same_explanation, case_for, concretize, datasets, explain_for, fixtures_root,
    registry_prefix, registry_rel, sha256_hex, sweep, CaseRun,
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Every REGISTRY row with at least one covering fixture explains —
/// the task's `every_registry_statistic_explains` (spec §6). The
/// row's first covering fixture also exercises the public
/// `explain_procedure`/`explain_calculator` API, asserted identical
/// (bit-for-bit values) to the run-once trace so the two paths can
/// never diverge.
#[test]
fn every_registry_statistic_explains() {
    let root = fixtures_root();
    let mut owned: BTreeMap<String, CaseRun> = BTreeMap::new();
    let mut problems = Vec::new();
    let mut covered_rows = 0usize;
    let mut covered_stats = 0usize;
    let mut public_checked = 0usize;
    for coverage in REGISTRY {
        if coverage.fixtures.is_empty() {
            problems.push(format!(
                "{}: registry row lists no covering fixtures",
                coverage.statistic
            ));
            continue;
        }
        let rel = registry_rel(coverage.statistic);
        let prefix = registry_prefix(coverage.statistic);
        let mut row_explained = 0usize;
        let mut public_done = false;
        for fixture_id in coverage.fixtures {
            let path = root.join(format!("{fixture_id}.json"));
            if !path.exists() {
                problems.push(format!(
                    "{}: covering fixture {fixture_id}.json missing",
                    coverage.statistic
                ));
                continue;
            }
            let case = match case_for(fixture_id, &mut owned, &root, datasets()) {
                Ok(c) => c,
                Err(e) => {
                    problems.push(format!(
                        "{}: covering fixture {fixture_id}: {e}",
                        coverage.statistic
                    ));
                    continue;
                }
            };
            // The row's first covering fixture explains through the
            // public API as well — asserted identical to the trace.
            let check_public = !public_done;
            let mut did_public = false;
            for concrete in concretize(&case.line.output.actual, rel) {
                let stat = format!("{prefix}.{concrete}");
                let exp = match case.exp(&stat) {
                    Some(e) => e,
                    None => {
                        problems.push(format!("{stat} ({fixture_id}): no trace"));
                        continue;
                    }
                };
                assert_eq!(exp.statistic, stat, "explanation carries the asked path");
                row_explained += 1;
                if check_public {
                    match explain_for(&case.line.fixture, datasets(), &stat) {
                        Ok(pub_exp) => {
                            assert_same_explanation(&pub_exp, exp, &stat);
                            public_checked += 1;
                            did_public = true;
                        }
                        Err(e) => problems.push(format!("{stat} ({fixture_id}): {e:?}")),
                    }
                }
            }
            public_done |= did_public;
        }
        if row_explained == 0 {
            problems.push(format!(
                "{}: no concrete instance explained by any covering fixture",
                coverage.statistic
            ));
        } else {
            covered_rows += 1;
            covered_stats += row_explained;
        }
    }
    assert!(
        problems.is_empty(),
        "registry explain coverage failures:\n{}",
        problems.join("\n")
    );
    assert!(covered_rows > 0 && covered_stats > 0);
    eprintln!(
        "every_registry_statistic_explains: {covered_rows} rows, {covered_stats} concrete stats, {public_checked} public-API checks"
    );
}

/// `last_step_equals_result_bitwise` — the last step's `result` is
/// the run's own leaf, bit for bit (spec §6).
#[test]
fn last_step_equals_result_bitwise() {
    let mut compared = 0usize;
    let mut undefined = 0usize;
    for (stat, exp, want) in sweep().iter().flat_map(|c| c.exps.iter()) {
        let last = exp.steps.last().expect("at least one step");
        match (&last.result, want) {
            (Estimate::Value(a), Estimate::Value(b)) => assert_eq!(
                a.to_bits(),
                b.to_bits(),
                "{stat}: trace result is not bit-identical"
            ),
            (a, b) => {
                assert_eq!(a, b, "{stat}: trace result is not identical");
                undefined += 1;
            }
        }
        compared += 1;
    }
    assert!(compared > 0);
    eprintln!("last_step_equals_result_bitwise: {compared} leaves, {undefined} undefined");
}

/// `formula_ids_are_declared` — every emitted step names a `FORMULAS`
/// id.
#[test]
fn formula_ids_are_declared() {
    let declared: BTreeSet<&str> = FORMULAS.iter().copied().collect();
    let mut problems = Vec::new();
    let mut stats = 0usize;
    let mut steps = 0usize;
    for (stat, exp, _) in sweep().iter().flat_map(|c| c.exps.iter()) {
        stats += 1;
        for step in &exp.steps {
            steps += 1;
            if !declared.contains(step.formula.as_str()) {
                problems.push(format!("{stat}: undeclared formula id {}", step.formula));
            }
            for (k, v) in &step.values {
                assert!(v.is_finite(), "{stat}: non-finite value {k}={v}");
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    eprintln!("formula_ids_are_declared: {stats} stats, {steps} steps");
}

/// `iterative_statistics_are_method_kind` — every iterative-class
/// registry statistic is `ExplainKind::Method` with one step.
#[test]
fn iterative_statistics_are_method_kind() {
    let root = fixtures_root();
    let mut owned: BTreeMap<String, CaseRun> = BTreeMap::new();
    let mut problems = Vec::new();
    let mut checked = 0usize;
    for coverage in REGISTRY {
        if coverage.class != ToleranceClass::Iterative {
            continue;
        }
        let rel = registry_rel(coverage.statistic);
        let prefix = registry_prefix(coverage.statistic);
        if coverage.fixtures.is_empty() {
            problems.push(format!(
                "{}: iterative registry row lists no covering fixtures",
                coverage.statistic
            ));
            continue;
        }
        let mut row_checked = 0usize;
        for fixture_id in coverage.fixtures {
            let path = root.join(format!("{fixture_id}.json"));
            if !path.exists() {
                problems.push(format!(
                    "{}: covering fixture {fixture_id}.json missing",
                    coverage.statistic
                ));
                continue;
            }
            let case = match case_for(fixture_id, &mut owned, &root, datasets()) {
                Ok(c) => c,
                Err(e) => {
                    problems.push(format!(
                        "{}: covering fixture {fixture_id}: {e}",
                        coverage.statistic
                    ));
                    continue;
                }
            };
            for concrete in concretize(&case.line.output.actual, rel) {
                let stat = format!("{prefix}.{concrete}");
                match case.exp(&stat) {
                    Some(ex) => {
                        if ex.kind != ExplainKind::Method || ex.steps.len() != 1 {
                            problems.push(format!(
                                "{stat}: kind {:?}, {} steps",
                                ex.kind,
                                ex.steps.len()
                            ));
                        }
                        checked += 1;
                        row_checked += 1;
                    }
                    None => problems.push(format!("{stat}: no trace")),
                }
            }
        }
        if row_checked == 0 {
            problems.push(format!(
                "{}: iterative row exercised by no fixture",
                coverage.statistic
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "iterative statistics not Method traces:\n{}",
        problems.join("\n")
    );
    assert!(checked > 0, "no iterative statistic was exercised");
    eprintln!("iterative_statistics_are_method_kind: {checked} concrete stats");
}

/// `results_unchanged` — the corpus parity hash golden generated on
/// main (owner decision D1, v1.0 Task 10). A row whose fixture file
/// (or dataset file) hash changed is reported, not compared; a row
/// whose fixture vanished from the run is reported; every unchanged
/// row's parity hash must match, and at least one row compares. The
/// hash is over `parity_line_normalized`: the receipt's
/// `engine.git_sha` and `engine.version` are build metadata fixed
/// at compile time (spec §4.3), not results, so they are pinned to
/// `"unknown"` and `receipt_sha256` resealed before hashing — the
/// comparison is build-environment independent.
#[test]
fn results_unchanged() {
    let root = fixtures_root();
    let golden_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/corpus_parity_main.tsv");
    let golden = std::fs::read_to_string(&golden_path).expect("read golden");
    let lines: BTreeMap<(String, String), &epiora_core::corpus::CorpusLine> = sweep()
        .iter()
        .map(|c| ((c.line.output.id.clone(), c.line.rel_path.clone()), &c.line))
        .collect();

    let mut compared = 0usize;
    let mut notes = Vec::new();
    let mut failures = Vec::new();
    for (n, row) in golden.lines().enumerate() {
        let cols: Vec<&str> = row.split('\t').collect();
        let (id, fixture_path, fixture_sha, dataset_path, dataset_sha, parity_sha) =
            match cols.as_slice() {
                [a, b, c, d, e, f] => (*a, *b, *c, *d, *e, *f),
                other => panic!("golden row {n} has {} columns", other.len()),
            };
        let rel = fixture_path
            .strip_prefix("fixtures/")
            .expect("golden fixture path is fixtures/<rel>");
        let Some(line) = lines.get(&(id.to_string(), rel.to_string())) else {
            // D1: a missing row fails when its fixture file (and
            // dataset file, if any) still exists unchanged — only a
            // genuinely changed or removed file is a note.
            let fpath = root.join(rel);
            let fixture_same = fpath.exists()
                && sha256_hex(&std::fs::read(&fpath).expect("read fixture")) == fixture_sha;
            let dataset_same = if dataset_path == "-" {
                true
            } else {
                let drel = dataset_path
                    .strip_prefix("fixtures/")
                    .expect("golden dataset path is fixtures/<rel>");
                let dpath = root.join(drel);
                dpath.exists()
                    && sha256_hex(&std::fs::read(&dpath).expect("read dataset")) == dataset_sha
            };
            if fixture_same && dataset_same {
                failures.push(format!(
                    "row {n} ({id}): fixture absent from corpus run                      but its fixture/dataset files are unchanged"
                ));
            } else {
                notes.push(format!(
                    "row {n} ({id}): fixture absent from corpus run;                      source file changed or removed"
                ));
            }
            continue;
        };
        let current_sha =
            sha256_hex(&std::fs::read(root.join(&line.rel_path)).expect("read fixture"));
        if current_sha != fixture_sha {
            notes.push(format!(
                "row {n} ({id}): fixture file changed — not compared"
            ));
            continue;
        }
        if dataset_path != "-" {
            let drel = dataset_path
                .strip_prefix("fixtures/")
                .expect("golden dataset path is fixtures/<rel>");
            let dsha = sha256_hex(&std::fs::read(root.join(drel)).expect("read dataset"));
            if dsha != dataset_sha {
                notes.push(format!(
                    "row {n} ({id}): dataset file changed — not compared"
                ));
                continue;
            }
        }
        let got = sha256_hex(line.output.parity_line_normalized().as_bytes());
        if got != parity_sha {
            failures.push(format!(
                "row {n} ({id}): parity hash changed\n  want {parity_sha}\n  got  {got}"
            ));
        }
        compared += 1;
    }
    for note in &notes {
        eprintln!("results_unchanged note: {note}");
    }
    assert!(
        failures.is_empty(),
        "parity results changed:\n{}",
        failures.join("\n")
    );
    assert!(compared >= 1, "no golden row was compared");
    eprintln!(
        "results_unchanged: {compared} rows compared, {} notes",
        notes.len()
    );
}

/// `zero_observation_group_explains_without_panic` — a MEANS run
/// whose groups hold zero non-missing values explains `Undefined`
/// leaves, never a panic: `GroupSummary::ssq` computed `n − 1` on
/// `u64` and panicked in debug builds on the all-missing `overall`
/// block (v1.0 Task 10 fix round 2; release wrapped silently).
#[test]
fn zero_observation_group_explains_without_panic() {
    let ctx = RunContext {
        timestamp: "2026-11-17T00:00:00Z".to_string(),
        project: None,
    };
    let too_few = Estimate::Undefined {
        reason: UndefinedReason::TooFew,
    };
    let last_result = |exp: &Explanation, stat: &str| {
        for step in &exp.steps {
            for (k, v) in &step.values {
                assert!(v.is_finite(), "{stat}: non-finite value {k}={v}");
            }
        }
        exp.steps.last().expect("at least one step").result.clone()
    };
    let mut explained = 0usize;

    // Every `V` missing: `group_numeric` keeps no group and `overall`
    // is the zero-observation summary `ssq` underflowed on.
    let empty = Dataset::new(
        "empty",
        1,
        vec![
            Column {
                name: "G".to_string(),
                data: ColumnData::Text(vec![
                    Some("a".to_string()),
                    Some("a".to_string()),
                    Some("b".to_string()),
                ]),
            },
            Column {
                name: "V".to_string(),
                data: ColumnData::Real(vec![None, None, None]),
            },
        ],
    )
    .expect("empty dataset");
    let opts = json!(MeansOptions {
        variable: "V".to_string(),
        group_by: Some("G".to_string()),
        include_missing: false,
    });
    let run = means(
        &empty,
        &serde_json::from_value::<MeansOptions>(opts.clone()).expect("options"),
        &ctx,
    )
    .expect("means run");
    assert_eq!(run.result.overall.observations, 0);
    assert_eq!(run.result.overall.mean, too_few);
    assert_eq!(run.result.overall.variance, too_few);
    assert!(run.result.anova.is_none());
    for stat in [
        "means.overall.observations",
        "means.overall.total",
        "means.overall.mean",
        "means.overall.variance",
        "means.overall.std_dev",
        "means.overall.min",
        "means.overall.p25",
        "means.overall.median",
        "means.overall.p75",
        "means.overall.max",
        "means.overall.mode",
    ] {
        let exp = explain_procedure(&empty, "means", &opts, stat).expect(stat);
        let result = last_result(&exp, stat);
        match stat {
            "means.overall.observations" | "means.overall.total" => {
                assert_eq!(result, Estimate::Value(0.0), "{stat}")
            }
            _ => assert_eq!(result, too_few, "{stat}"),
        }
        explained += 1;
    }

    // Group `c` has zero non-missing values (`group_numeric` drops
    // it); group `b` keeps one observation — ANOVA stays in the
    // result, its within-group statistics `Undefined { too_few }`.
    let sparse = Dataset::new(
        "sparse",
        1,
        vec![
            Column {
                name: "G".to_string(),
                data: ColumnData::Text(vec![
                    Some("a".to_string()),
                    Some("a".to_string()),
                    Some("a".to_string()),
                    Some("b".to_string()),
                    Some("c".to_string()),
                    Some("c".to_string()),
                ]),
            },
            Column {
                name: "V".to_string(),
                data: ColumnData::Real(vec![
                    Some(1.0),
                    Some(2.0),
                    Some(3.0),
                    Some(9.0),
                    None,
                    None,
                ]),
            },
        ],
    )
    .expect("sparse dataset");
    let run = means(
        &sparse,
        &serde_json::from_value::<MeansOptions>(opts.clone()).expect("options"),
        &ctx,
    )
    .expect("means run");
    let anova = run.result.anova.as_ref().expect("anova present");
    for est in [
        &anova.ss_within,
        &anova.ms_within,
        &anova.f,
        &anova.p_value,
        &anova.ss_total,
    ] {
        assert_eq!(*est, too_few);
    }
    assert_eq!(run.result.groups[1].variance, too_few);
    assert_eq!(run.result.groups[1].mean, Estimate::Value(9.0));
    for stat in [
        "means.groups[1].variance",
        "means.groups[1].std_dev",
        "means.anova.ss_within",
        "means.anova.ms_within",
        "means.anova.f",
        "means.anova.p_value",
        "means.anova.ss_total",
    ] {
        let exp = explain_procedure(&sparse, "means", &opts, stat).expect(stat);
        assert_eq!(last_result(&exp, stat), too_few, "{stat}");
        explained += 1;
    }
    eprintln!("zero_observation_group_explains_without_panic: {explained} stats explained");
}

/// `explain_unknown` — a statistic that resolves to no leaf is a
/// typed `ExplainUnknown`, never a panic (spec §6).
#[test]
fn explain_unknown() {
    let ds = datasets().values().next().expect("a dataset");
    let variable = ds.columns()[0].name.clone();
    let opts = json!(FreqOptions {
        variables: vec![variable],
        stratify_by: None,
        include_missing: false,
        ci_method: ProportionCi::ClopperPearson,
    });
    match explain_procedure(ds, "freq", &opts, "freq.no_such_leaf") {
        Err(EngineError::ExplainUnknown { statistic }) => {
            assert_eq!(statistic, "freq.no_such_leaf");
        }
        other => panic!("expected ExplainUnknown, got {other:?}"),
    }
    let err = explain_calculator(
        "proportion",
        &json!({
            "numerator": 10,
            "denominator": 45,
            "conf_level": 95,
            "multiplier": 100,
            "population_size": 1000000,
            "compare_with": 50
        }),
        "calc.proportion.no_such_leaf",
    )
    .expect_err("unknown calc statistic");
    assert_eq!(err.code(), "explain_unknown");
    eprintln!("explain_unknown: 2 unknown-statistic errors");
}
