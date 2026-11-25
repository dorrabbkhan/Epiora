#![cfg(feature = "corpus")]

//! Calculator registry and input-validation tests (v1.0 Task 7).

use epiora_core::calc::{definition, input::validate, run, REGISTRY};
use epiora_core::error::EngineError;
use epiora_core::procedures::RunContext;
use epiora_core::validation::REGISTRY as COVERAGE;
use proptest::prelude::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;

/// The calculator ids of `docs/v1.0-calculators.md` `## ` headings, in
/// inventory order. Non-calculator sections are filtered out because
/// their first word is not all-lowercase snake_case.
fn inventory_ids() -> Vec<String> {
    let doc = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/v1.0-calculators.md"),
    )
    .expect("inventory doc");
    doc.lines()
        .filter_map(|line| {
            let heading = line.strip_prefix("## ")?;
            let id = heading.split_whitespace().next()?;
            (id.chars().all(|c| c.is_ascii_lowercase() || c == '_') && !id.is_empty())
                .then(|| id.to_string())
        })
        .collect()
}

#[test]
fn registry_ids_match_inventory() {
    let ids: Vec<&str> = REGISTRY.iter().map(|d| d.id).collect();
    let inventory = inventory_ids();
    assert_eq!(ids.len(), 24, "REGISTRY must list all 24 calculators");
    assert_eq!(ids, inventory);
}

#[test]
fn every_output_has_registry_coverage() {
    let registered: BTreeSet<&str> = COVERAGE.iter().map(|c| c.statistic).collect();
    let mut missing = Vec::new();
    // All six families emit as of Task 9 — every declared output must
    // carry a validation REGISTRY row.
    for def in REGISTRY.iter() {
        for out in def.outputs {
            if !registered.contains(out.statistic) {
                missing.push(format!("{}: {}", def.id, out.statistic));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "registry outputs missing from validation REGISTRY:\n{}",
        missing.join("\n")
    );
}

/// `CalculatorDef::groups` is the registry's "one of" construct
/// (v1.0 Task 8 follow-up): every member must be a declared input
/// and `Omit`-defaulted — a filled or required member would make the
/// group's cardinality check meaningless.
#[test]
fn group_members_are_declared_optional_inputs() {
    for def in REGISTRY {
        for group in def.groups {
            for member in group.members {
                let input = def
                    .inputs
                    .iter()
                    .find(|i| i.name == *member)
                    .unwrap_or_else(|| panic!("{}: group member {member} undeclared", def.id));
                assert!(
                    matches!(input.default, Some(epiora_core::calc::DefaultValue::Omit)),
                    "{}: group member {member} must be Omit-defaulted",
                    def.id
                );
            }
        }
    }
}

#[test]
fn defaults_fill_canonically() {
    let def = definition("proportion").expect("proportion def");
    let canonical =
        validate(def, &json!({"numerator": 3, "denominator": 5})).expect("valid inputs");
    assert_eq!(canonical["conf_level"], 95);
    assert_eq!(canonical["multiplier"], 100);
    assert_eq!(canonical["population_size"], 1_000_000);
    // `compare_with` is optional and stays absent when not supplied.
    assert!(canonical.get("compare_with").is_none());
}

#[test]
fn unknown_input_rejected() {
    let def = definition("proportion").expect("proportion def");
    let err = validate(def, &json!({"bogus": 1})).expect_err("unknown key");
    assert_eq!(err.code(), "calc_input_out_of_range");
    match err {
        EngineError::CalcInputOutOfRange { input, reason } => {
            assert_eq!(input, "bogus");
            assert_eq!(reason, "unknown_input");
        }
        other => panic!("expected CalcInputOutOfRange, got {other:?}"),
    }
}

#[test]
fn counts_above_2_pow_53_rejected() {
    let def = definition("proportion").expect("proportion def");
    let err = validate(
        def,
        &json!({"numerator": 9007199254740992.0, "denominator": 100}),
    )
    .expect_err("2^53 must be rejected");
    assert_eq!(err.code(), "calc_input_out_of_range");
}

#[test]
fn conf_level_only_90_95_99() {
    let def = definition("proportion").expect("proportion def");
    for ok in [90, 95, 99] {
        validate(
            def,
            &json!({"numerator": 3, "denominator": 5, "conf_level": ok}),
        )
        .unwrap_or_else(|e| panic!("conf_level {ok}: {e}"));
    }
    for bad in [80, 97, 0, 100] {
        let err = validate(
            def,
            &json!({"numerator": 3, "denominator": 5, "conf_level": bad}),
        )
        .expect_err("conf_level must be rejected");
        assert_eq!(err.code(), "calc_input_out_of_range");
    }
}

#[test]
fn table_shape_enforced() {
    let def = definition("two_by_two").expect("two_by_two def");
    // A 3-column stratum is rejected.
    let err = validate(def, &json!({"strata": [[1, 2, 3]]})).expect_err("3 cols must be rejected");
    assert_eq!(err.code(), "calc_input_out_of_range");
    // Zero strata is below min_rows.
    let err = validate(def, &json!({"strata": []})).expect_err("empty strata");
    assert_eq!(err.code(), "calc_input_out_of_range");
    // A non-integer cell is rejected.
    let err = validate(def, &json!({"strata": [[1, 2], [3, 4.5]]})).expect_err("fractional cell");
    assert_eq!(err.code(), "calc_input_out_of_range");
    // Ten strata exceeds the 9-stratum cap.
    let err = validate(
        def,
        &json!({"strata": [[1,2],[3,4],[1,2],[3,4],[1,2],[3,4],[1,2],[3,4],[1,2],[3,4]]}),
    )
    .expect_err("10 strata must be rejected");
    assert_eq!(err.code(), "calc_input_out_of_range");
    // A negative cell is rejected.
    let err = validate(def, &json!({"strata": [[1, 2], [3, -1]]})).expect_err("negative cell");
    assert_eq!(err.code(), "calc_input_out_of_range");
}

#[test]
fn nested_strata_count_enforced() {
    let def = definition("two_by_two").expect("two_by_two def");
    let stratum = json!([[1, 2], [3, 4]]);
    // Nine nested strata is the inventory cap.
    validate(def, &json!({"strata": vec![stratum.clone(); 9]})).expect("9 strata");
    // Ten nested strata exceeds it — the outer count is bounded too,
    // not just a flat table's rows.
    let err = validate(def, &json!({"strata": vec![stratum; 10]}))
        .expect_err("10 nested strata must be rejected");
    assert_eq!(err.code(), "calc_input_out_of_range");
}

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".into(),
        project: None,
        working_view: None,
    }
}

/// The `CalcInputOutOfRange` `(input, reason)` of an error, for tests
/// that assert the typed failure rather than `is_err`.
fn out_of_range_parts(e: &EngineError) -> (&str, &str) {
    match e {
        EngineError::CalcInputOutOfRange { input, reason } => (input, reason),
        other => panic!("expected CalcInputOutOfRange, got {other:?}"),
    }
}

/// Review round 2: a list-of-tables is a strata list — calculators
/// whose inventory input is a plain table (`strata: None`) reject it
/// as `bad_table_shape` instead of panicking mid-compute.
#[test]
fn nested_input_rejected_for_non_strata_tables() {
    let nested = json!([[[1, 2], [3, 4]], [[5, 6], [7, 8]]]);
    for (id, key) in [
        ("screening", "table"),
        ("r_by_c", "table"),
        ("matched_case_control", "matched_sets"),
        ("anova_summary", "table"),
    ] {
        let err = run(id, &json!({ key: nested.clone() }), &ctx())
            .expect_err(&format!("{id} must reject a nested table"));
        assert_eq!(err.code(), "calc_input_out_of_range", "{id}");
        assert_eq!(out_of_range_parts(&err).1, "bad_table_shape", "{id}");
    }
}

/// Review round 2: the nested outer bound is each calculator's
/// inventory strata cap, not the stratum's own `max_rows` — and
/// `dose_response`'s `strata` count must agree with the list length.
#[test]
fn strata_count_bound_per_calculator() {
    let level = json!([[0, 10, 20], [1, 15, 25]]);
    // dose_response: 9 nested strata is the cap even though one
    // stratum may hold 10 level rows; `strata` must match the list.
    let nine = json!({"table": vec![level.clone(); 9], "strata": 9});
    run("dose_response", &nine, &ctx()).expect("9 strata, declared 9");
    let ten = json!({"table": vec![level.clone(); 10], "strata": 10});
    // 10 fails twice over: above the integer bound and the list cap.
    let err = run("dose_response", &ten, &ctx()).expect_err("10 strata must be rejected");
    assert_eq!(err.code(), "calc_input_out_of_range");
    // A list shorter or longer than the declared count is a typed
    // `bad_table_shape` on `strata`.
    for (len, declared) in [(2, 3), (3, 2)] {
        let err = run(
            "dose_response",
            &json!({"table": vec![level.clone(); len], "strata": declared}),
            &ctx(),
        )
        .expect_err("strata/table mismatch must be rejected");
        assert_eq!(
            out_of_range_parts(&err),
            ("strata", "bad_table_shape"),
            "len {len} vs declared {declared}"
        );
    }
    // The flat single-stratum form is the strata = 1 case.
    run(
        "dose_response",
        &json!({"table": [[0, 10, 20], [1, 15, 25]], "strata": 1}),
        &ctx(),
    )
    .expect("flat table is one stratum");
    let err = run(
        "dose_response",
        &json!({"table": [[0, 10, 20], [1, 15, 25]], "strata": 2}),
        &ctx(),
    )
    .expect_err("flat table with strata 2 must be rejected");
    assert_eq!(out_of_range_parts(&err), ("strata", "bad_table_shape"));
    // Strata whose level counts differ cannot be pooled by position.
    let ragged = json!({
        "table": [level.clone(), json!([[0, 10, 20], [1, 15, 25], [2, 5, 5]])],
        "strata": 2,
    });
    let err =
        run("dose_response", &ragged, &ctx()).expect_err("unequal level counts must be rejected");
    assert_eq!(err.code(), "calc_input_out_of_range");
}

/// Review round 2: a 2×2 stratum is exactly two rows of two counts —
/// a 3-row stratum was silently truncated and a 1-row stratum
/// silently padded; both are now `bad_table_shape`, flat or nested.
#[test]
fn malformed_strata_rejected() {
    for id in ["two_by_two", "two_by_n"] {
        for (label, strata) in [
            ("flat 3-row", json!([[1, 2], [3, 4], [5, 6]])),
            ("flat 1-row", json!([[1, 2]])),
            ("nested 3-row", json!([[[1, 2], [3, 4], [5, 6]]])),
            ("nested 1-row", json!([[[1, 2]]])),
        ] {
            let def = definition(id).expect("def");
            let err = validate(def, &json!({"strata": strata}))
                .expect_err(&format!("{id} {label} must be rejected"));
            assert_eq!(err.code(), "calc_input_out_of_range", "{id} {label}");
            assert_eq!(
                out_of_range_parts(&err).1,
                "bad_table_shape",
                "{id} {label}"
            );
        }
        // A well-formed 2×2 stratum still validates, flat and nested.
        let def = definition(id).expect("def");
        validate(def, &json!({"strata": [[1, 2], [3, 4]]})).expect("flat 2x2");
        validate(def, &json!({"strata": [[[1, 2], [3, 4]]]})).expect("nested 2x2");
    }
}

/// Review round 2 (the `anova_summary` row contract): each row is
/// `[n integer ≥ 2 and < 2^53, mean finite, sd finite > 0]` — the
/// untyped `CellKind::Real` table cannot say so, so the calculator
/// checks per row; the listed failures used to underflow `n − 1`,
/// overflow `n_tot` or compute NaN silently.
#[test]
fn anova_summary_rejects_bad_rows() {
    let good = json!([5, 10.0, 2.0]);
    let cases: [(&str, Value, &str); 7] = [
        ("n = 0", json!([0, 10.0, 2.0]), "below_min"),
        ("n = 1", json!([1, 10.0, 2.0]), "below_min"),
        ("negative n", json!([-3, 10.0, 2.0]), "below_min"),
        ("fractional n", json!([2.5, 10.0, 2.0]), "not_an_integer"),
        (
            "n = 2^53",
            json!([9_007_199_254_740_992.0, 10.0, 2.0]),
            "above_max",
        ),
        ("sd = 0", json!([5, 10.0, 0.0]), "below_min"),
        ("negative sd", json!([5, 10.0, -1.0]), "below_min"),
    ];
    for (label, bad_row, reason) in cases {
        let err = run(
            "anova_summary",
            &json!({"table": [bad_row, good.clone()]}),
            &ctx(),
        )
        .expect_err(&format!("{label} must be rejected"));
        assert_eq!(err.code(), "calc_input_out_of_range", "{label}");
        assert_eq!(out_of_range_parts(&err), ("table", reason), "{label}");
    }
    // Valid rows still compute.
    run(
        "anova_summary",
        &json!({"table": [[5, 10.0, 2.0], [7, 12.0, 3.0]]}),
        &ctx(),
    )
    .expect("valid anova_summary rows");
}

/// Arbitrary JSON trees (arrays nested four deep, numbers across the
/// count boundary) — the shapes that fuzzed `calc::run` into silent
/// truncation, padding and panics in review round 2.
fn arb_json() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        any::<i64>().prop_map(Value::from),
        any::<f64>().prop_map(Value::from),
        any::<bool>().prop_map(Value::from),
        Just(Value::Null),
        "[a-z]{0,4}".prop_map(Value::from),
    ];
    leaf.prop_recursive(4, 32, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(Value::Array),
            prop::collection::btree_map("[a-z]{0,4}", inner, 0..4)
                .prop_map(|m| Value::Object(m.into_iter().collect())),
        ]
    })
}

/// Review round 2: `calc::run` never panics — valid inputs compute,
/// everything else is a typed `EngineError` — for the calculators
/// whose table/strata contracts this fix round tightened.
#[test]
fn calc_run_never_panics_on_table_inputs() {
    proptest!(|(v in arb_json(), w in arb_json(), m in 1u64..=4)| {
        let cases: [(String, Value); 10] = [
            ("two_by_two".into(), json!({"strata": v})),
            ("two_by_n".into(), json!({"strata": v})),
            ("dose_response".into(), json!({"table": v, "strata": m})),
            ("screening".into(), json!({"table": v})),
            ("r_by_c".into(), json!({"table": v})),
            ("matched_case_control".into(), json!({"matched_sets": v, "controls_per_case": m})),
            ("anova_summary".into(), json!({"table": v})),
            ("rate_compare".into(), json!({"strata": v, "per_unit": 1000})),
            ("mean_ci".into(), json!({"mean": v, "sd_or": "sd", "sd_value": w,
                                      "n": 25, "population_size": v})),
            ("t_test_summary".into(), json!({"n_1": v, "mean_1": w, "sd_1": v,
                                            "n_2": 30, "mean_2": 0, "sd_2": 1})),
        ];
        for (id, inputs) in cases {
            // `Ok` or `Err` — a panic fails the property.
            let _ = run(&id, &inputs, &ctx());
        }
    });
}

/// v1.0 Task 13: every `calc.*` statistic the `### v1.0 Task 13 —
/// method choices` table in `docs/decisions.md` lists must be an
/// `OutputDef` whose `method` names the tool showing the differing
/// method (`"OpenEpi shows …"`, `"EI7 shows …"` or `Epi Info`), and
/// every output whose `method` carries an `"OpenEpi shows"` /
/// `"EI7 shows"` marker must appear in the table. The markers also
/// classify the OpenEpi corpus leg's documented method differences.
#[test]
fn every_differing_output_is_labelled() {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/decisions.md"),
    )
    .expect("docs/decisions.md");
    let section = text
        .split("### v1.0 Task 13 — method choices")
        .nth(1)
        .expect("the method-choices section must exist")
        .split("\n### ")
        .next()
        .unwrap();
    let listed: BTreeSet<&str> = section
        .lines()
        .filter(|l| l.starts_with("| `calc."))
        .map(|l| l.trim_start_matches("| `").split('`').next().unwrap())
        .collect();
    assert!(
        !listed.is_empty(),
        "the method-choices table must list statistics"
    );
    for stat in &listed {
        let output = REGISTRY
            .iter()
            .flat_map(|d| d.outputs.iter())
            .find(|o| o.statistic == *stat)
            .unwrap_or_else(|| panic!("{stat}: listed in decisions.md but not an OutputDef"));
        assert!(
            output.method.contains("OpenEpi shows")
                || output.method.contains("EI7 shows")
                || output.method.contains("Epi Info"),
            "{stat}: `method` does not name the tool showing the differing method: {}",
            output.method
        );
    }
    let marked: BTreeSet<&str> = REGISTRY
        .iter()
        .flat_map(|d| d.outputs.iter())
        .filter(|o| o.method.contains("OpenEpi shows") || o.method.contains("EI7 shows"))
        .map(|o| o.statistic)
        .collect();
    let unlisted: Vec<_> = marked.difference(&listed).copied().collect();
    assert!(
        unlisted.is_empty(),
        "outputs marked as method differences but missing from the table: {unlisted:?}"
    );
}
