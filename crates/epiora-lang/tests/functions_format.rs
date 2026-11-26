//! Exact-value tests for `FORMAT` numeric digit pictures (v1.1 Task 14,
//! item 11): Visual Basic `Format` honours `0`, `#` and `.` placeholders
//! and rounds half away from zero.

use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_lang::{eval_column, parse_expr, EvalEnv};

fn dataset() -> Dataset {
    Dataset::new(
        "t",
        1,
        vec![Column {
            name: "x".into(),
            data: ColumnData::Real(vec![None, Some(2.0)]),
        }],
    )
    .expect("dataset builds")
}

fn text_values(source: &str, ds: &Dataset) -> Vec<Option<String>> {
    let expr = parse_expr(source).unwrap_or_else(|e| panic!("{source}: {e}"));
    match eval_column(&expr, ds, &EvalEnv::default()).unwrap_or_else(|e| panic!("{source}: {e}")) {
        ColumnData::Text(v) => v,
        other => panic!("{source} yielded {other:?}"),
    }
}

fn first(source: &str) -> Option<String> {
    text_values(source, &dataset()).into_iter().next().flatten()
}

/// The `functions_text_math.html` line 109 values: integer and real
/// inputs both honour the two-decimal picture.
#[test]
fn format_zero_padded_two_decimal_picture() {
    assert_eq!(first("FORMAT(54, \"0.00\")").as_deref(), Some("54.00"));
    assert_eq!(first("FORMAT(46.1, \"0.00\")").as_deref(), Some("46.10"));
    assert_eq!(first("FORMAT(53.3, \"0.00\")").as_deref(), Some("53.30"));
}

/// VB's numeric pictures round half away from zero, positive and negative.
#[test]
fn format_rounds_half_away_from_zero() {
    assert_eq!(first("FORMAT(2.5, \"0\")").as_deref(), Some("3"));
    assert_eq!(first("FORMAT(-2.5, \"0\")").as_deref(), Some("-3"));
    assert_eq!(first("FORMAT(0.125, \"0.00\")").as_deref(), Some("0.13"));
}

/// .NET Framework first rounds a `double` to 15 significant digits, so
/// binary-rounding tails never decide the picture rounding.
#[test]
fn format_reduces_reals_to_15_significant_digits_first() {
    assert_eq!(
        first("FORMAT(2.674999999999999, \"0.00\")").as_deref(),
        Some("2.68")
    );
    assert_eq!(first("FORMAT(2.675, \"0.00\")").as_deref(), Some("2.68"));
    assert_eq!(first("FORMAT(1.005, \"0.00\")").as_deref(), Some("1.01"));
    assert_eq!(first("FORMAT(0.1 + 0.2, \"0.0\")").as_deref(), Some("0.3"));
}

/// `#` is optional: fractional trailing zeroes drop, and a zero integer
/// part has no leading digit.
#[test]
fn format_hash_picture_drops_non_significant_zeroes() {
    assert_eq!(first("FORMAT(0.456, \"#.##\")").as_deref(), Some(".46"));
    assert_eq!(first("FORMAT(1.2, \"#.##\")").as_deref(), Some("1.2"));
    assert_eq!(first("FORMAT(1.25, \"0.0\")").as_deref(), Some("1.3"));
}

/// A missing operand stays missing; a present operand formats.
#[test]
fn format_missing_input_is_missing() {
    let values = text_values("FORMAT(x, \"0.00\")", &dataset());
    assert_eq!(values, vec![None, Some("2.00".into())]);
}
