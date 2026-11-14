//! `run_case` — runs one corpus fixture through the matching `stats`
//! function or `procedures` call and returns the serialized result for
//! `compare` (spec §6.3). Fixtures with `inputs.dataset` run the real
//! procedure on the corpus dataset and carry the receipt; a declared
//! `inputs.table`/`inputs.strata` is checked against the tabulation.
//! `parity_line` renders the case for the native/wasm bit-parity diff
//! (spec §5.2).

use super::super::procedures::{
    freq, tables, FreqOptions, ProcOutput, ProportionCi, RunContext, TablesOptions, TablesResult,
    VarSpec,
};
use super::super::receipt::canonical_json;
use super::super::stats::{rxc, stratified, two_by_two, Table2x2};
use super::super::tabulate::cross;
use super::run_special::run_special;
use crate::dataset::Dataset;
use crate::EngineError;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// The fixed `RunContext` timestamp for dataset-backed corpus runs —
/// corpus outputs (and receipts) must be reproducible bit for bit.
const CASE_TIMESTAMP: &str = "2026-10-30T12:00:00Z";

/// The output of one corpus case.
pub struct CaseOutput {
    /// The fixture id.
    pub id: String,
    /// The serialized stats object the fixture's `expected` describes.
    pub actual: Value,
    /// The canonical receipt JSON when the case ran a procedure on a
    /// dataset.
    pub receipt: Option<String>,
}

impl CaseOutput {
    /// Canonical JSON of `{id, actual, receipt}` with every f64
    /// replaced by its 16-hex-digit bit pattern (spec §5.2) — the line
    /// `corpus-runner` prints and the wasm parity run must reproduce.
    pub fn parity_line(&self) -> String {
        canonical_json(&json!({
            "id": self.id,
            "actual": encode_bits(&self.actual),
            "receipt": self.receipt,
        }))
    }
}

fn encode_bits(v: &Value) -> Value {
    match v {
        Value::Number(n) => match n.as_f64() {
            Some(f) if n.is_f64() => Value::String(format!("{:016x}", f.to_bits())),
            _ => v.clone(),
        },
        Value::Array(items) => Value::Array(items.iter().map(encode_bits).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, c)| (k.clone(), encode_bits(c)))
                .collect(),
        ),
        _ => v.clone(),
    }
}

fn corpus(reason: impl Into<String>) -> EngineError {
    EngineError::CorpusFormat {
        reason: reason.into(),
    }
}

fn ctx() -> RunContext {
    RunContext {
        timestamp: CASE_TIMESTAMP.to_string(),
        project: None,
    }
}

fn spec(name: &Value) -> Result<VarSpec, EngineError> {
    match name.as_str() {
        Some(n) => Ok(VarSpec {
            name: n.to_string(),
            levels: None,
        }),
        None => Err(corpus("variable name is not a string")),
    }
}

fn dataset<'a>(
    fixture: &Value,
    datasets: &'a BTreeMap<String, Dataset>,
) -> Result<Option<&'a Dataset>, EngineError> {
    match fixture["inputs"].get("dataset") {
        None => Ok(None),
        Some(v) => {
            let id = v
                .as_str()
                .ok_or_else(|| corpus("inputs.dataset is not a string"))?;
            datasets
                .get(id)
                .map(Some)
                .ok_or_else(|| corpus(format!("unknown dataset {id:?}")))
        }
    }
}

fn table2x2(fixture: &Value) -> Result<Table2x2, EngineError> {
    let t = &fixture["inputs"]["table"];
    let cell = |r: usize, c: usize| -> Result<u64, EngineError> {
        t.get(r)
            .and_then(|row| row.get(c))
            .and_then(Value::as_u64)
            .ok_or_else(|| corpus(format!("inputs.table[{r}][{c}] is missing or not u64")))
    };
    Ok(Table2x2 {
        a: cell(0, 0)?,
        b: cell(0, 1)?,
        c: cell(1, 0)?,
        d: cell(1, 1)?,
    })
}

fn strata(fixture: &Value) -> Result<Vec<Table2x2>, EngineError> {
    let items = fixture["inputs"]["strata"]
        .as_array()
        .ok_or_else(|| corpus("inputs.strata is not an array"))?;
    items
        .iter()
        .map(|t| table2x2(&json!({"inputs": {"table": t}})))
        .collect()
}

fn receipt_of<T: serde::Serialize>(out: &ProcOutput<T>) -> Result<String, EngineError> {
    serde_json::to_value(&out.receipt)
        .map(|v| canonical_json(&v))
        .map_err(|e| corpus(format!("receipt does not serialize: {e}")))
}

/// Checks a declared `inputs.table`/`inputs.strata` against what the
/// tabulation produced — a disagreement is a corpus error, not a
/// comparison mismatch.
fn check_counts(
    fixture: &Value,
    ds: &Dataset,
    opts: &TablesOptions,
    expect_strata: bool,
) -> Result<(), EngineError> {
    let xt = cross(
        ds,
        &opts.exposure,
        &opts.outcome,
        &opts.stratify_by,
        opts.include_missing,
    )?;
    let declared: Vec<Vec<u64>> = if expect_strata {
        let mut out = Vec::new();
        for t in strata(fixture)? {
            out.push(vec![t.a, t.b, t.c, t.d]);
        }
        out
    } else {
        let t = table2x2(fixture)?;
        vec![vec![t.a, t.b, t.c, t.d]]
    };
    let actual: Vec<Vec<u64>> = xt
        .counts
        .iter()
        .filter(|t| t.len() >= 2)
        .map(|t| {
            t.iter()
                .take(2)
                .flat_map(|row| row.iter().take(2).copied())
                .collect()
        })
        .collect();
    if actual != declared {
        return Err(corpus(format!(
            "{}: dataset tabulation {actual:?} differs from declared counts {declared:?}",
            fixture["id"]
        )));
    }
    Ok(())
}

/// Runs one fixture (spec §6.3). `actual` is the stats object —
/// `crude` for a `tables` 2×2, the stratified stats for
/// `stratified_two_by_two`, `tables[0]` for `freq`, the raw `RxcStats`
/// for `rxc`, `{"values": [...]}` for `special.*`.
pub fn run_case(
    fixture: &Value,
    datasets: &BTreeMap<String, Dataset>,
) -> Result<CaseOutput, EngineError> {
    let id = fixture["id"]
        .as_str()
        .ok_or_else(|| corpus("fixture has no string id"))?
        .to_string();
    let procedure = fixture["procedure"]
        .as_str()
        .ok_or_else(|| corpus(format!("{id}: procedure is not a string")))?;
    let out = |actual: Value, receipt: Option<String>| {
        Ok(CaseOutput {
            id: id.clone(),
            actual,
            receipt,
        })
    };

    match procedure {
        "two_by_two" => match dataset(fixture, datasets)? {
            Some(ds) => {
                let opts = TablesOptions {
                    exposure: spec(&fixture["inputs"]["exposure"])?,
                    outcome: spec(&fixture["inputs"]["outcome"])?,
                    stratify_by: vec![],
                    include_missing: false,
                };
                check_counts(fixture, ds, &opts, false)?;
                let output = tables(ds, &opts, &ctx())?;
                let TablesResult::TwoByTwo { crude, .. } = &output.result else {
                    return Err(corpus(format!("{id}: dataset run is not a 2×2")));
                };
                out(
                    serde_json::to_value(crude).map_err(|e| corpus(e.to_string()))?,
                    Some(receipt_of(&output)?),
                )
            }
            None => out(
                serde_json::to_value(two_by_two(&table2x2(fixture)?))
                    .map_err(|e| corpus(e.to_string()))?,
                None,
            ),
        },
        "stratified_two_by_two" => match dataset(fixture, datasets)? {
            Some(ds) => {
                let stratify_by = match fixture["inputs"]["stratify_by"].as_str() {
                    Some(s) => vec![s.to_string()],
                    None => vec![],
                };
                let opts = TablesOptions {
                    exposure: spec(&fixture["inputs"]["exposure"])?,
                    outcome: spec(&fixture["inputs"]["outcome"])?,
                    stratify_by,
                    include_missing: false,
                };
                check_counts(fixture, ds, &opts, true)?;
                let output = tables(ds, &opts, &ctx())?;
                let TablesResult::TwoByTwo {
                    stratified: Some(s),
                    ..
                } = &output.result
                else {
                    return Err(corpus(format!("{id}: dataset run produced no strata")));
                };
                out(
                    serde_json::to_value(&s.stats).map_err(|e| corpus(e.to_string()))?,
                    Some(receipt_of(&output)?),
                )
            }
            None => out(
                serde_json::to_value(stratified(&strata(fixture)?))
                    .map_err(|e| corpus(e.to_string()))?,
                None,
            ),
        },
        "rxc" => {
            let table: Vec<Vec<u64>> = fixture["inputs"]["table"]
                .as_array()
                .ok_or_else(|| corpus("inputs.table is not an array"))?
                .iter()
                .map(|row| {
                    row.as_array()
                        .map(|r| r.iter().filter_map(Value::as_u64).collect())
                        .ok_or_else(|| corpus("inputs.table row is not an array"))
                })
                .collect::<Result<_, _>>()?;
            out(
                serde_json::to_value(rxc(&table)).map_err(|e| corpus(e.to_string()))?,
                None,
            )
        }
        "freq" => {
            let ds = dataset(fixture, datasets)?
                .ok_or_else(|| corpus(format!("{id}: freq fixture needs inputs.dataset")))?;
            let inputs = &fixture["inputs"];
            let opts = FreqOptions {
                variables: vec![inputs["variable"]
                    .as_str()
                    .ok_or_else(|| corpus(format!("{id}: inputs.variable")))?
                    .to_string()],
                stratify_by: inputs
                    .get("stratify_by")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                include_missing: inputs["include_missing"].as_bool().unwrap_or(false),
                ci_method: ProportionCi::ClopperPearson,
            };
            let output = freq(ds, &opts, &ctx())?;
            let first = output
                .result
                .tables
                .first()
                .ok_or_else(|| corpus(format!("{id}: freq produced no tables")))?;
            out(
                serde_json::to_value(first).map_err(|e| corpus(e.to_string()))?,
                Some(receipt_of(&output)?),
            )
        }
        name if name.starts_with("special.") => run_special(fixture).and_then(|v| out(v, None)),
        other => Err(corpus(format!("{id}: unknown procedure {other:?}"))),
    }
}
