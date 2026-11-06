//! Inference and coercion unit tests (v0.3 plan Task 3 Step 2; spec
//! §6.2). Candidates are tried Boolean → Integer → Real → Date → Text.

use epiora_core::dataset::{ColumnData, ValueType};
use epiora_import::{import, DateFormat, ImportError, ImportSettings};

fn imported(bytes: &[u8], settings: &ImportSettings) -> epiora_import::Imported {
    import(bytes, "t.csv", settings).expect("import")
}

fn types(bytes: &[u8], settings: &ImportSettings) -> Vec<ValueType> {
    imported(bytes, settings)
        .columns
        .iter()
        .map(|c| c.data.value_type())
        .collect()
}

#[test]
fn priority_boolean_integer_real_date_text() {
    let t = types(
        b"B,I,R,D,T\ntrue,1,1.5,2020-01-01,abc\n",
        &ImportSettings::default(),
    );
    assert_eq!(
        t,
        [
            ValueType::Boolean,
            ValueType::Integer,
            ValueType::Real,
            ValueType::Date,
            ValueType::Text
        ]
    );
}

#[test]
fn all_missing_is_boolean() {
    // Spec §6.2: the first candidate that every non-missing value fits
    // wins; with zero non-missing values that is Boolean (the
    // fixtures/import corpus agrees: ALLMISS, header_only).
    let t = types(b"X\n\nNA\n.\n", &ImportSettings::default());
    assert_eq!(t, [ValueType::Boolean]);
}

#[test]
fn zero_one_is_integer() {
    let t = types(b"Z\n0\n1\n1\n0\n", &ImportSettings::default());
    assert_eq!(t, [ValueType::Integer]);
}

#[test]
fn plus_sign_integer() {
    let t = types(b"P\n+5\n-3\n", &ImportSettings::default());
    assert_eq!(t, [ValueType::Integer]);
}

#[test]
fn inf_and_nan_are_text() {
    let t = types(b"X\ninf\nnan\n-inf\nInfinity\n", &ImportSettings::default());
    assert_eq!(t, [ValueType::Text]);
}

#[test]
fn thousands_separator_is_text() {
    // "1,234" with Dot decimal is not a number.
    let t = types(b"N;V\n1.5;1,234\n2.5;2,345\n", &ImportSettings::default());
    assert_eq!(t, [ValueType::Real, ValueType::Text]);
}

#[test]
fn iso_date_rejects_feb_30() {
    let t = types(b"D\n2021-02-30\n2021-03-01\n", &ImportSettings::default());
    assert_eq!(t, [ValueType::Text]);
}

#[test]
fn dmy_needs_setting() {
    let bytes = b"D\n03/04/2026\n15/06/2020\n";
    assert_eq!(types(bytes, &ImportSettings::default()), [ValueType::Text]);
    let settings = ImportSettings {
        date_format: DateFormat::Dmy,
        ..ImportSettings::default()
    };
    let i = imported(bytes, &settings);
    assert_eq!(i.columns[0].data.value_type(), ValueType::Date);
    let ColumnData::Date(days) = &i.columns[0].data else {
        panic!("expected Date")
    };
    // 03/04/2026 dmy = 2026-04-03; 15/06/2020 = 2020-06-15.
    assert_eq!(days[0], Some(20546));
    assert_eq!(days[1], Some(18428));
}

#[test]
fn missing_codes_match_after_ascii_trim() {
    let i = imported(b"X\n  NA  \n5\n", &ImportSettings::default());
    let ColumnData::Integer(v) = &i.columns[0].data else {
        panic!("expected Integer")
    };
    assert_eq!(v, &[None, Some(5)]);
}

#[test]
fn text_keeps_original_whitespace() {
    let i = imported(b"X\n  abc  \nfoo\n", &ImportSettings::default());
    let ColumnData::Text(v) = &i.columns[0].data else {
        panic!("expected Text")
    };
    assert_eq!(v[0].as_deref(), Some("  abc  "));
}

#[test]
fn limits_are_errors() {
    let header = vec!["x"; 16_385].join(",");
    let err = import(
        format!("{header}\n").as_bytes(),
        "wide.csv",
        &ImportSettings::default(),
    )
    .expect_err("16,385 columns must fail");
    assert!(matches!(err, ImportError::TooManyColumns));
    assert_eq!(err.code(), "too_many_columns");
}
