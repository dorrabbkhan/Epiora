#![cfg(feature = "corpus")]

//! `random_numbers` calculator tests (v1.0 Task 9). There is no R
//! fixture for this calculator (spec §5.2: R cannot reproduce a
//! ChaCha stream) — the oracle is the golden file
//! `tests/golden/random_numbers.json` plus the distribution property
//! tests here.

use epiora_core::calc::other::random_numbers::draw_values;
use epiora_core::calc::run;
use epiora_core::procedures::RunContext;
use epiora_core::special::chisq_sf;
use proptest::prelude::*;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".into(),
        project: None,
        working_view: None,
    }
}

/// Seeds 0, 1 and 20261112 with the golden file's fixed options
/// produce its recorded draws exactly — the only per-value oracle
/// this calculator has (spec §5.2).
#[test]
fn random_numbers_golden() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/random_numbers.json");
    let golden: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read golden file"))
            .expect("parse golden file");
    let cases = golden["cases"].as_array().expect("golden cases array");
    assert_eq!(cases.len(), 3, "golden file must hold the three seeds");
    for case in cases {
        let out = run("random_numbers", &case["inputs"], &ctx())
            .unwrap_or_else(|e| panic!("random_numbers {}: {e}", case["seed"]));
        assert_eq!(
            out.result["values"],
            case["values"].clone(),
            "seed {}",
            case["seed"]
        );
    }
}

/// `unique = yes` redraws duplicates: the emitted values are pairwise
/// distinct.
#[test]
fn random_unique_has_no_duplicates() {
    for seed in [0u64, 1, 20261112, 42] {
        let out = run(
            "random_numbers",
            &json!({"lower": 1, "upper": 10, "count": 10, "seed": seed,
                    "unique": "yes"}),
            &ctx(),
        )
        .expect("unique run");
        let values: Vec<i64> = out.result["values"]
            .as_array()
            .expect("values array")
            .iter()
            .map(|v| v.as_i64().expect("integer value"))
            .collect();
        let distinct: BTreeSet<i64> = values.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            values.len(),
            "seed {seed}: duplicates in {values:?}"
        );
    }
}

/// Every emitted value lands inside `[lower, upper]` — arbitrary
/// seeds, ranges and counts through the full `calc::run` path.
#[test]
fn random_values_in_range() {
    proptest!(|(
        seed in 0u64..(1u64 << 53),
        lower in -1_000_000i64..1_000_000,
        span in 1u64..1_000,
        count in 1u64..200,
    )| {
        let upper = lower + span as i64 - 1;
        let out = run(
            "random_numbers",
            &json!({"lower": lower, "upper": upper, "count": count,
                    "seed": seed}),
            &ctx(),
        )
        .expect("in-range run");
        let values = out.result["values"].as_array().expect("values array");
        prop_assert_eq!(values.len() as u64, count);
        for v in values {
            let x = v.as_i64().expect("integer value");
            prop_assert!(x >= lower && x <= upper, "{x} outside [{lower},{upper}]");
        }
    });
}

/// Uniformity of the fixed stream: 100,000 draws over 10 values is
/// χ²-tested against equal expectation (df = 9) for each of the
/// three golden seeds; p must exceed 1e-6 (plan Task 9).
#[test]
fn random_uniform_chi_square() {
    for seed in [0u64, 1, 20261112] {
        let values = draw_values(seed, 1, 10, 100_000);
        let mut bins = [0u64; 10];
        for &x in &values {
            assert!((1..=10).contains(&x), "draw {x} outside [1,10]");
            bins[(x - 1) as usize] += 1;
        }
        let expected = 10_000.0;
        let chi2: f64 = bins
            .iter()
            .map(|&o| (o as f64 - expected) * (o as f64 - expected) / expected)
            .sum();
        let p = chisq_sf(chi2, 9.0);
        assert!(p > 1e-6, "seed {seed}: χ² p = {p} (χ² = {chi2})");
    }
}

/// `unique = yes` with `count` above the span cannot terminate — a
/// typed input error, not a hang. `lower > upper` and a span past
/// 2^32 are rejected the same way.
#[test]
fn random_unique_impossible_rejected() {
    let cases = [
        // unique count exceeds the span.
        json!({"lower": 1, "upper": 10, "count": 11, "seed": 0, "unique": "yes"}),
        // lower above upper.
        json!({"lower": 10, "upper": 1, "count": 5, "seed": 0}),
        // span above 2^32.
        json!({"lower": 0, "upper": 4294967296u64, "count": 5, "seed": 0}),
    ];
    for inputs in cases {
        let err = run("random_numbers", &inputs, &ctx())
            .expect_err(&format!("{inputs} must be rejected"));
        assert_eq!(err.code(), "calc_input_out_of_range", "{inputs}");
    }
}
