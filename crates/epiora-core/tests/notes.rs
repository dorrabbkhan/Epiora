#![cfg(feature = "corpus")]

//! Assumption-note tests (v1.0 Task 22, spec §6.1): the five engine
//! rules fire on the values the run itself computed, never change a
//! result or a receipt, emit only declared codes, and are
//! deterministic under arbitrary valid inputs.

use epiora_core::calc;
use epiora_core::corpus::{corpus_lines, load_dataset_dir};
use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_core::notes::{Note, NOTE_CODES};
use epiora_core::procedures::{
    means, tables, MeansOptions, ProcOutput, RunContext, TablesOptions, TablesResult, VarSpec,
};
use epiora_core::receipt::{canonical_json, EngineStamp};
use proptest::prelude::*;
use serde_json::{json, Value};
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".into(),
        project: None,
    }
}

fn spec(name: &str) -> VarSpec {
    VarSpec {
        name: name.into(),
        levels: None,
    }
}

fn col(name: &str, data: ColumnData) -> Column {
    Column {
        name: name.into(),
        data,
    }
}

fn bools(v: &[bool]) -> ColumnData {
    ColumnData::Boolean(v.iter().map(|b| Some(*b)).collect())
}

fn texts(v: &[&str]) -> ColumnData {
    ColumnData::Text(v.iter().map(|s| Some(s.to_string())).collect())
}

fn sha256_hex(bytes: &[u8]) -> String {
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn codes(notes: &[Note]) -> BTreeSet<&str> {
    notes.iter().map(|n| n.code.as_str()).collect()
}

/// Appends one stratum's `(e, i)` records for `cells` = `[a,b,c,d]`.
fn stratum(
    cells: [u64; 4],
    e: &mut Vec<bool>,
    i: &mut Vec<bool>,
    s: &mut Vec<String>,
    label: &str,
) {
    for (n, ev, iv) in [
        (cells[0], true, true),
        (cells[1], true, false),
        (cells[2], false, true),
        (cells[3], false, false),
    ] {
        for _ in 0..n {
            e.push(ev);
            i.push(iv);
            s.push(label.to_string());
        }
    }
}

/// `tables` on a Boolean `E`×`I` dataset — one `cells` block per
/// stratum label; `&[(label, cells)]` of one `""` is unstratified.
fn tables_run(strata: &[(&str, [u64; 4])]) -> ProcOutput<TablesResult> {
    let (mut e, mut i, mut s) = (Vec::new(), Vec::new(), Vec::new());
    for (label, cells) in strata {
        stratum(*cells, &mut e, &mut i, &mut s, label);
    }
    let stratify_by: Vec<String> = if strata.len() > 1 {
        vec!["S".into()]
    } else {
        vec![]
    };
    let mut columns = vec![col("E", bools(&e)), col("I", bools(&i))];
    if !stratify_by.is_empty() {
        columns.push(col(
            "S",
            texts(&s.iter().map(String::as_str).collect::<Vec<_>>()),
        ));
    }
    let ds = Dataset::new("t", 1, columns).expect("dataset");
    tables(
        &ds,
        &TablesOptions {
            exposure: spec("E"),
            outcome: spec("I"),
            stratify_by,
            include_missing: false,
        },
        &ctx(),
    )
    .expect("tables")
}

/// Oswego's vanilla×ill table ([[43,11],[3,18]]) has every expected
/// cell above 5 — the classic non-sparse case must carry no
/// small-count note (and, with no zero cell and no strata, no notes
/// at all).
#[test]
fn oswego_vanilla_has_no_small_count_note() {
    let datasets = load_dataset_dir(&fixtures_root().join("data")).expect("corpus datasets");
    let ds = datasets.get("oswego").expect("oswego dataset");
    let out = tables(
        ds,
        &TablesOptions {
            exposure: spec("VANILLA"),
            outcome: spec("ILL"),
            stratify_by: vec![],
            include_missing: false,
        },
        &ctx(),
    )
    .expect("tables");
    assert!(
        !codes(&out.notes).contains("expected_count_below_5"),
        "oswego vanilla expected counts are all ≥ 5: {:?}",
        out.notes
    );
    assert!(out.notes.is_empty(), "no rule fires: {:?}", out.notes);
}

/// A 2×2 with an expected cell below 5 ([[5,1],[1,8]]: expected
/// minimum 2.4) carries `expected_count_below_5` suggesting Fisher.
#[test]
fn sparse_two_by_two_suggests_fisher() {
    let out = tables_run(&[("", [5, 1, 1, 8])]);
    let note = out
        .notes
        .iter()
        .find(|n| n.code == "expected_count_below_5")
        .expect("expected_count_below_5 note");
    assert!(
        note.statistics
            .iter()
            .any(|s| s == "two_by_two.chi_square.uncorrected.p_value"),
        "chi-square statistics referenced: {:?}",
        note.statistics
    );
    assert_eq!(
        note.suggests.as_deref(),
        Some("two_by_two.fisher_exact.p_two_sided")
    );
    assert!(
        note.values["expected_min"] < 5.0,
        "trigger value: {:?}",
        note.values
    );
}

/// A zero observed cell ([[0,5],[3,7]]) carries `zero_cell` on the
/// Wald OR/RR statistics, suggesting the exact/mid-p path.
#[test]
fn zero_cell_note() {
    let out = tables_run(&[("", [0, 5, 3, 7])]);
    let note = out
        .notes
        .iter()
        .find(|n| n.code == "zero_cell")
        .expect("zero_cell note");
    for stat in [
        "two_by_two.odds_ratio.ci_95_wald.lower",
        "two_by_two.odds_ratio.ci_95_wald.upper",
        "two_by_two.risk_ratio.ci_95_wald.lower",
        "two_by_two.risk_ratio.ci_95_wald.upper",
    ] {
        assert!(
            note.statistics.iter().any(|s| s == stat),
            "{stat} in {:?}",
            note.statistics
        );
    }
    assert_eq!(note.suggests.as_deref(), Some("two_by_two.odds_ratio.cmle"));
    assert_eq!(note.values["zero_cells"], 1.0);
}

/// `x = 2` out of `n = 30` gives n·p = 2 < 5 — the Wald interval on
/// `calc.proportion` carries `normal_approx_small` suggesting Wilson.
#[test]
fn wald_small_np_note() {
    let proportion = |x: u64| {
        calc::run(
            "proportion",
            &json!({ "numerator": x, "denominator": 30 }),
            &ctx(),
        )
        .expect("proportion")
    };
    let note = proportion(2)
        .notes
        .into_iter()
        .find(|n| n.code == "normal_approx_small")
        .expect("normal_approx_small note");
    for stat in [
        "calc.proportion.ci.wald.lower",
        "calc.proportion.ci.wald.upper",
    ] {
        assert!(
            note.statistics.iter().any(|s| s == stat),
            "{stat} in {:?}",
            note.statistics
        );
    }
    assert_eq!(
        note.suggests.as_deref(),
        Some("calc.proportion.ci.wilson.lower")
    );
    assert_eq!(note.values["np"], 2.0);
    // x = 28 of 30 → n·(1−p) = 2 also fires; x = 15 of 30 does not.
    assert!(codes(&proportion(28).notes).contains("normal_approx_small"));
    assert!(!codes(&proportion(15).notes).contains("normal_approx_small"));
}

/// Two groups with wildly different variances make Bartlett's p
/// significant — `unequal_variances` suggests the Welch t-test.
#[test]
fn bartlett_significant_suggests_welch() {
    let v: Vec<Option<f64>> = (0..15)
        .map(|i| Some(if i % 2 == 0 { 9.9 } else { 10.1 }))
        .chain((0..15).map(|i| Some((i as f64 - 7.0) * 10.0)))
        .collect();
    let g = texts(
        &["a"; 15]
            .iter()
            .chain(&["b"; 15])
            .copied()
            .collect::<Vec<_>>(),
    );
    let ds =
        Dataset::new("t", 1, vec![col("v", ColumnData::Real(v)), col("g", g)]).expect("dataset");
    let out = means(
        &ds,
        &MeansOptions {
            variable: "v".into(),
            group_by: Some("g".into()),
            include_missing: false,
        },
        &ctx(),
    )
    .expect("means");
    let note = out
        .notes
        .iter()
        .find(|n| n.code == "unequal_variances")
        .expect("unequal_variances note");
    assert_eq!(note.suggests.as_deref(), Some("means.t_test.welch.t"));
    assert!(note.values["bartlett_p"] < 0.05, "{:?}", note.values);
}

/// Strata with opposite odds ratios (100 vs 0.01) make the
/// Breslow–Day–Tarone p significant — `heterogeneous_strata`.
#[test]
fn breslow_day_significant_note() {
    let out = tables_run(&[
        ("x", [10, 1, 1, 10]), // OR 100
        ("y", [1, 10, 10, 1]), // OR 0.01
    ]);
    let note = out
        .notes
        .iter()
        .find(|n| n.code == "heterogeneous_strata")
        .expect("heterogeneous_strata note");
    for stat in [
        "stratified_two_by_two.breslow_day_tarone.p_value",
        "stratified_two_by_two.mantel_haenszel.odds_ratio.estimate",
    ] {
        assert!(
            note.statistics.iter().any(|s| s == stat),
            "{stat} in {:?}",
            note.statistics
        );
    }
    assert!(note.suggests.is_none());
    assert!(
        note.values["breslow_day_tarone_p"] < 0.05,
        "{:?}",
        note.values
    );
}

/// Notes never change a result: every corpus case's parity line must
/// hash to the value `tests/golden/corpus_parity_main.tsv` recorded on
/// main (same rule `explain.rs::results_unchanged` applies — rows
/// whose fixture or dataset file changed are reported, not compared).
#[test]
fn notes_do_not_change_results() {
    let root = fixtures_root();
    let golden = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/corpus_parity_main.tsv"),
    )
    .expect("golden");
    let lines: BTreeMap<(String, String), _> = corpus_lines(&root)
        .expect("corpus run")
        .into_iter()
        .map(|c| ((c.output.id.clone(), c.rel_path.clone()), c))
        .collect();
    let (mut compared, mut failures) = (0usize, Vec::new());
    for (n, row) in golden.lines().enumerate() {
        let cols: Vec<&str> = row.split('\t').collect();
        let (id, fixture_path, fixture_sha, dataset_path, dataset_sha, parity_sha) =
            match cols.as_slice() {
                [a, b, c, d, e, f] => (*a, *b, *c, *d, *e, *f),
                other => panic!("golden row {n} has {} columns", other.len()),
            };
        let rel = fixture_path
            .strip_prefix("fixtures/")
            .expect("fixtures/<rel>");
        if sha256_hex(&std::fs::read(root.join(rel)).expect("read fixture")) != fixture_sha {
            continue; // Fixture file changed — not comparable (D1).
        }
        if dataset_path != "-"
            && sha256_hex(
                &std::fs::read(
                    root.join(
                        dataset_path
                            .strip_prefix("fixtures/")
                            .expect("fixtures/<rel>"),
                    ),
                )
                .expect("read dataset"),
            ) != dataset_sha
        {
            continue;
        }
        let line = lines
            .get(&(id.to_string(), rel.to_string()))
            .unwrap_or_else(|| panic!("row {n} ({id}): fixture absent from corpus run"));
        let got = sha256_hex(line.output.parity_line_normalized().as_bytes());
        if got != parity_sha {
            failures.push(format!(
                "row {n} ({id}): parity changed\n  want {parity_sha}\n  got  {got}"
            ));
        }
        compared += 1;
    }
    assert!(compared > 0, "nothing compared");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Receipts are byte-identical with notes attached: the v1 golden is
/// regenerated exactly, and a v3 calculator receipt carries no
/// `notes` key and still seals over its own content only.
#[test]
fn receipts_unchanged() {
    // The same dataset `receipt_v3.rs::golden_v1_unchanged` rebuilds —
    // the receipt seals the dataset sha256, so the rows are verbatim.
    let ds = Dataset::new(
        "golden",
        1,
        vec![
            col(
                "E",
                bools(&[true, true, true, true, false, false, false, false]),
            ),
            col(
                "I",
                bools(&[true, false, true, false, true, true, false, false]),
            ),
            col("S", texts(&["x", "x", "y", "y", "x", "x", "y", "y"])),
        ],
    )
    .expect("dataset");
    let opts = TablesOptions {
        exposure: spec("E"),
        outcome: spec("I"),
        stratify_by: vec!["S".into()],
        include_missing: false,
    };
    let mut receipt = tables(&ds, &opts, &ctx()).expect("tables").receipt;
    receipt.engine = EngineStamp {
        name: "epiora-core".into(),
        version: "1.0.0".into(),
        git_sha: "0000000".into(),
    };
    receipt.receipt_sha256 = receipt.sha256_hex();
    let canon = canonical_json(&serde_json::to_value(&receipt).expect("serialize"));
    let golden = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/receipt_v1.json"),
    )
    .expect("golden receipt");
    assert_eq!(canon, golden.trim_end());

    // A note-emitting calculator run: the v3 receipt serializes no
    // `notes` key and seals over itself only.
    let sparse = calc::run("two_by_two", &json!({ "strata": [[0, 5], [3, 7]] }), &ctx())
        .expect("two_by_two");
    assert!(!sparse.notes.is_empty(), "fixture must emit a note");
    let v3 = serde_json::to_value(&sparse.receipt).expect("serialize receipt");
    assert!(v3.get("notes").is_none(), "notes leak into the receipt");
    assert_eq!(sparse.receipt.receipt_sha256, sparse.receipt.sha256_hex());
}

/// Every code any rule emits is declared in `NOTE_CODES` — and this
/// battery reaches all five so a declared-but-dead rule shows too.
#[test]
fn codes_are_declared() {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut collect = |notes: &[Note]| {
        for n in notes {
            assert!(
                NOTE_CODES.contains(&n.code.as_str()),
                "undeclared code {:?}",
                n.code
            );
            seen.insert(n.code.clone());
        }
    };
    collect(&tables_run(&[("", [5, 1, 1, 8])]).notes);
    collect(&tables_run(&[("", [0, 5, 3, 7])]).notes);
    collect(
        &calc::run(
            "two_by_two",
            &json!({ "strata": [[[10, 1], [1, 10]], [[1, 10], [10, 1]]] }),
            &ctx(),
        )
        .expect("two_by_two")
        .notes,
    );
    collect(
        &calc::run("r_by_c", &json!({ "table": [[5, 1], [1, 8]] }), &ctx())
            .expect("r_by_c")
            .notes,
    );
    collect(
        &calc::run(
            "anova_summary",
            &json!({ "table": [[15, 10.0, 0.1], [15, 0.0, 10.0], [15, 5.0, 0.2]] }),
            &ctx(),
        )
        .expect("anova_summary")
        .notes,
    );
    collect(
        &calc::run(
            "proportion",
            &json!({ "numerator": 2, "denominator": 30 }),
            &ctx(),
        )
        .expect("proportion")
        .notes,
    );
    let declared: BTreeSet<String> = NOTE_CODES.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(seen, declared, "the battery must reach every code");
}

proptest! {
    /// Notes are deterministic (identical runs → identical notes) and
    /// never panic on arbitrary valid inputs.
    #[test]
    fn notes_deterministic_no_panic(
        strata in proptest::collection::vec(
            proptest::collection::vec(0u64..40, 4usize),
            1..=3usize,
        ),
        x in 0u64..=80,
        n in 1u64..=80,
    ) {
        let strata_json: Vec<Value> = strata
            .iter()
            .map(|c| json!([[c[0], c[1]], [c[2], c[3]]]))
            .collect();
        let inputs = json!({ "strata": strata_json });
        let a = calc::run("two_by_two", &inputs, &ctx()).expect("two_by_two a");
        let b = calc::run("two_by_two", &inputs, &ctx()).expect("two_by_two b");
        prop_assert_eq!(&a.notes, &b.notes, "notes differ between identical runs");
        for note in &a.notes {
            prop_assert!(NOTE_CODES.contains(&note.code.as_str()));
        }

        let x = x.min(n);
        let pi = json!({ "numerator": x, "denominator": n });
        let a = calc::run("proportion", &pi, &ctx()).expect("proportion a");
        let b = calc::run("proportion", &pi, &ctx()).expect("proportion b");
        prop_assert_eq!(&a.notes, &b.notes);
        for note in &a.notes {
            prop_assert!(NOTE_CODES.contains(&note.code.as_str()));
        }
    }
}
