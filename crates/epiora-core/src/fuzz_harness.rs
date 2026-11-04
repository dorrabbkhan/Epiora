//! Fuzz harnesses (spec §6.5). Each public function takes the raw bytes
//! a fuzzer supplies, decodes them with `arbitrary`, and runs one
//! engine entry point. A harness returns normally on `Ok`, on `Err`
//! (`EngineError` is the expected outcome for bad input) and on an
//! `arbitrary` decode failure. The only permitted panic is the single
//! invariant assertion in [`assert_no_null`].
//!
//! The cargo-fuzz targets in `fuzz/fuzz_targets/` call these functions;
//! `tests/fuzz_regressions.rs` replays committed crash inputs through
//! the same code.

use crate::corpus::load_dataset;
use crate::dataset::{Column, ColumnData, Dataset, Value};
use crate::procedures::{
    freq, tables, FreqOptions, ProportionCi, RunContext, TablesOptions, VarSpec,
};
use crate::EngineError;
use arbitrary::{Arbitrary, Result, Unstructured};
use serde::Serialize;
use serde_json::Value as Json;

// Bounds keep one fuzz unit small enough for libFuzzer to stay fast;
// they are truncation caps on decoded data, not rejection rules.
const MAX_COLUMNS: usize = 8;
const MAX_ROWS: usize = 64;
const MAX_VARS: usize = 8;
const MAX_STRATA: usize = 3;
const MAX_NAME: usize = 24;
const MAX_TEXT: usize = 32;
/// One selector in ten names an unknown variable (spec §6.5).
const UNKNOWN_EVERY: u8 = 10;

fn ctx() -> RunContext {
    RunContext {
        timestamp: "fuzz".into(),
    }
}

/// A column's data as decoded bytes describe it; `f64` may be NaN or
/// infinite — `Dataset::new` must reject it (`InvalidColumn`).
enum FuzzData {
    Integer(Vec<Option<i64>>),
    Real(Vec<Option<f64>>),
    Text(Vec<Option<String>>),
    Boolean(Vec<Option<bool>>),
    Date(Vec<Option<i32>>),
}

impl<'a> Arbitrary<'a> for FuzzData {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(match u.arbitrary::<u8>()? % 5 {
            0 => Self::Integer(u.arbitrary()?),
            1 => Self::Real(u.arbitrary()?),
            2 => Self::Text(u.arbitrary()?),
            3 => Self::Boolean(u.arbitrary()?),
            _ => Self::Date(u.arbitrary()?),
        })
    }
}

struct FuzzColumn {
    name: String,
    data: FuzzData,
}

impl<'a> Arbitrary<'a> for FuzzColumn {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Self {
            name: u.arbitrary()?,
            data: u.arbitrary()?,
        })
    }
}

fn to_column(raw: FuzzColumn) -> Column {
    let name: String = raw.name.chars().take(MAX_NAME).collect();
    let data = match raw.data {
        FuzzData::Integer(mut v) => {
            v.truncate(MAX_ROWS);
            ColumnData::Integer(v)
        }
        FuzzData::Real(mut v) => {
            v.truncate(MAX_ROWS);
            ColumnData::Real(v)
        }
        FuzzData::Text(mut v) => {
            v.truncate(MAX_ROWS);
            ColumnData::Text(
                v.into_iter()
                    .map(|s| s.map(|t| t.chars().take(MAX_TEXT).collect()))
                    .collect(),
            )
        }
        FuzzData::Boolean(mut v) => {
            v.truncate(MAX_ROWS);
            ColumnData::Boolean(v)
        }
        FuzzData::Date(mut v) => {
            v.truncate(MAX_ROWS);
            ColumnData::Date(v)
        }
    };
    Column { name, data }
}

struct FuzzDataset {
    id: String,
    version: u64,
    columns: Vec<FuzzColumn>,
}

impl<'a> Arbitrary<'a> for FuzzDataset {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Self {
            id: u.arbitrary()?,
            version: u.arbitrary()?,
            columns: u.arbitrary()?,
        })
    }
}

fn build_dataset(raw: FuzzDataset) -> std::result::Result<Dataset, EngineError> {
    let id: String = raw.id.chars().take(MAX_NAME).collect();
    let columns = raw
        .columns
        .into_iter()
        .take(MAX_COLUMNS)
        .map(to_column)
        .collect();
    Dataset::new(id, raw.version, columns)
}

/// A scalar for `VarSpec::levels`; deliberately not type-matched to the
/// column so `TypeMismatch` is exercised too.
enum FuzzValue {
    Integer(i64),
    Real(f64),
    Text(String),
    Boolean(bool),
    Date(i32),
}

impl<'a> Arbitrary<'a> for FuzzValue {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(match u.arbitrary::<u8>()? % 5 {
            0 => Self::Integer(u.arbitrary()?),
            1 => Self::Real(u.arbitrary()?),
            2 => Self::Text(u.arbitrary()?),
            3 => Self::Boolean(u.arbitrary()?),
            _ => Self::Date(u.arbitrary()?),
        })
    }
}

impl From<FuzzValue> for Value {
    fn from(v: FuzzValue) -> Self {
        match v {
            FuzzValue::Integer(x) => Self::Integer(x),
            FuzzValue::Real(x) => Self::Real(x),
            FuzzValue::Text(x) => Self::Text(x.chars().take(MAX_TEXT).collect()),
            FuzzValue::Boolean(x) => Self::Boolean(x),
            FuzzValue::Date(x) => Self::Date(x),
        }
    }
}

fn levels(raw: Option<(FuzzValue, FuzzValue)>) -> Option<[Value; 2]> {
    raw.map(|(a, b)| [a.into(), b.into()])
}

/// `sel` picks a generated column by index; every tenth selector (and
/// an empty name list) names an unknown variable instead, exercising
/// `UnknownVariable`. Deterministic: the choice comes only from the
/// decoded bytes — no RNG, no clock.
fn pick(sel: u8, names: &[String]) -> String {
    if names.is_empty() || sel.is_multiple_of(UNKNOWN_EVERY) {
        format!("__fuzz_unknown_{sel}")
    } else {
        names[usize::from(sel) % names.len()].clone()
    }
}

fn names_of(ds: &Dataset) -> Vec<String> {
    ds.columns().iter().map(|c| c.name.clone()).collect()
}

/// The plan's one assertion (spec §6.5): a serialized output contains
/// no `null`. serde_json writes non-finite `f64` as `null`, and Task 8
/// guarantees results never serialize a `null` — under cargo-fuzz this
/// panic is the crash signal; anywhere else it means that guarantee
/// broke. The scoped allow lifts the crate's `clippy::panic` deny for
/// exactly this assertion; no other code in the crate may panic.
#[allow(clippy::panic)]
fn assert_no_null(value: &Json, path: &str) {
    match value {
        Json::Null => panic!("{path}: null in serialized output"),
        Json::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                assert_no_null(item, &format!("{path}[{i}]"));
            }
        }
        Json::Object(map) => {
            for (key, item) in map {
                assert_no_null(item, &format!("{path}.{key}"));
            }
        }
        _ => {}
    }
}

fn check_result(result: &impl Serialize) {
    if let Ok(json) = serde_json::to_value(result) {
        assert_no_null(&json, "result");
    }
}

/// `Dataset::new` rejects non-finite reals; if one ever slipped
/// through, serializing the real cells would emit `null`.
fn assert_no_nonfinite(ds: &Dataset) {
    for column in ds.columns() {
        if let ColumnData::Real(values) = &column.data {
            let cells: Vec<f64> = values.iter().flatten().copied().collect();
            if let Ok(json) = serde_json::to_value(&cells) {
                assert_no_null(&json, &column.name);
            }
        }
    }
}

/// Corpus loader: decoded bytes as JSON into `load_dataset`.
pub fn dataset_json(data: &[u8]) {
    let mut u = Unstructured::new(data);
    let Ok(json) = u.arbitrary::<String>() else {
        return;
    };
    if let Ok(ds) = load_dataset(&json) {
        assert_no_nonfinite(&ds);
    }
}

/// Arbitrary typed columns into `Dataset::new`.
pub fn dataset_columns(data: &[u8]) {
    let mut u = Unstructured::new(data);
    let Ok(input) = FuzzDataset::arbitrary(&mut u) else {
        return;
    };
    if let Ok(ds) = build_dataset(input) {
        assert_no_nonfinite(&ds);
    }
}

struct FuzzFreq {
    dataset: FuzzDataset,
    variables: Vec<u8>,
    stratify_by: Option<u8>,
    include_missing: bool,
    wilson: bool,
}

impl<'a> Arbitrary<'a> for FuzzFreq {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Self {
            dataset: u.arbitrary()?,
            variables: u.arbitrary()?,
            stratify_by: u.arbitrary()?,
            include_missing: u.arbitrary()?,
            wilson: u.arbitrary()?,
        })
    }
}

/// Arbitrary dataset and `FreqOptions` into `freq`.
pub fn freq_any(data: &[u8]) {
    let mut u = Unstructured::new(data);
    let Ok(input) = FuzzFreq::arbitrary(&mut u) else {
        return;
    };
    let Ok(ds) = build_dataset(input.dataset) else {
        return;
    };
    let names = names_of(&ds);
    let opts = FreqOptions {
        variables: input
            .variables
            .iter()
            .take(MAX_VARS)
            .map(|&sel| pick(sel, &names))
            .collect(),
        stratify_by: input.stratify_by.map(|sel| pick(sel, &names)),
        include_missing: input.include_missing,
        ci_method: if input.wilson {
            ProportionCi::Wilson
        } else {
            ProportionCi::ClopperPearson
        },
    };
    if let Ok(out) = freq(&ds, &opts, &ctx()) {
        check_result(&out.result);
    }
}

struct FuzzTables {
    dataset: FuzzDataset,
    exposure: u8,
    outcome: u8,
    exposure_levels: Option<(FuzzValue, FuzzValue)>,
    outcome_levels: Option<(FuzzValue, FuzzValue)>,
    stratify_by: Vec<u8>,
    include_missing: bool,
}

impl<'a> Arbitrary<'a> for FuzzTables {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        Ok(Self {
            dataset: u.arbitrary()?,
            exposure: u.arbitrary()?,
            outcome: u.arbitrary()?,
            exposure_levels: u.arbitrary()?,
            outcome_levels: u.arbitrary()?,
            stratify_by: u.arbitrary()?,
            include_missing: u.arbitrary()?,
        })
    }
}

/// Arbitrary dataset and `TablesOptions` into `tables`.
pub fn tables_any(data: &[u8]) {
    let mut u = Unstructured::new(data);
    let Ok(input) = FuzzTables::arbitrary(&mut u) else {
        return;
    };
    let Ok(ds) = build_dataset(input.dataset) else {
        return;
    };
    let names = names_of(&ds);
    let opts = TablesOptions {
        exposure: VarSpec {
            name: pick(input.exposure, &names),
            levels: levels(input.exposure_levels),
        },
        outcome: VarSpec {
            name: pick(input.outcome, &names),
            levels: levels(input.outcome_levels),
        },
        stratify_by: input
            .stratify_by
            .iter()
            .take(MAX_STRATA)
            .map(|&sel| pick(sel, &names))
            .collect(),
        include_missing: input.include_missing,
    };
    if let Ok(out) = tables(&ds, &opts, &ctx()) {
        check_result(&out.result);
    }
}
