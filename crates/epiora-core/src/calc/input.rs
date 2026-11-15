//! Calculator input validation (v1.0 spec §5.3). `validate` checks a
//! raw JSON inputs object against a `CalculatorDef`'s declared kinds
//! and returns canonical inputs — defaults filled, optional inputs
//! left out. Every failure is `CalcInputMissing` or
//! `CalcInputOutOfRange`; unknown keys are `unknown_input`.

use super::{CalculatorDef, CellKind, DefaultValue, InputKind};
use crate::error::EngineError;
use serde_json::{Map, Value};

/// `2^53` — counts are integers strictly below it (Review Focus 3).
pub(crate) const MAX_COUNT: u64 = 1u64 << 53;

fn out_of_range(input: &str, reason: &str) -> EngineError {
    EngineError::CalcInputOutOfRange {
        input: input.into(),
        reason: reason.into(),
    }
}

fn count(name: &str, v: &Value) -> Result<Value, EngineError> {
    let n = v
        .as_u64()
        .filter(|&n| n < MAX_COUNT)
        .ok_or_else(|| out_of_range(name, "not_a_count"))?;
    Ok(Value::from(n))
}

fn integer(name: &str, v: &Value, min: i64, max: i64) -> Result<Value, EngineError> {
    let n = v
        .as_i64()
        .ok_or_else(|| out_of_range(name, "not_an_integer"))?;
    if n < min {
        return Err(out_of_range(name, "below_min"));
    }
    if n > max {
        return Err(out_of_range(name, "above_max"));
    }
    Ok(Value::from(n))
}

fn real(name: &str, v: &Value, min: f64, max: f64) -> Result<Value, EngineError> {
    let x = v
        .as_f64()
        .filter(|x| x.is_finite())
        .ok_or_else(|| out_of_range(name, "not_finite"))?;
    if x < min {
        return Err(out_of_range(name, "below_min"));
    }
    if x > max {
        return Err(out_of_range(name, "above_max"));
    }
    Ok(Value::from(x))
}

fn cell(name: &str, kind: CellKind, v: &Value) -> Result<Value, EngineError> {
    match kind {
        CellKind::Count => count(name, v),
        CellKind::Real => real(name, v, f64::MIN, f64::MAX),
    }
}

/// A 2-D table of `cols`-wide rows (or calculator-defined columns when
/// `cols == 0`), `min_rows..=max_rows` rows. When every element is
/// itself a 2-D array the input is a list of tables (dose-response or
/// 2×2 strata): each element is validated on the same spec and the
/// outer list is capped at `max_rows`, the inventory's strata bound —
/// a Task 7 gap that left 10+ nested strata accepted.
fn table(
    name: &str,
    v: &Value,
    cols: u8,
    min_rows: u8,
    max_rows: u8,
    kind: CellKind,
) -> Result<Value, EngineError> {
    let items = v
        .as_array()
        .ok_or_else(|| out_of_range(name, "bad_table_shape"))?;
    let nested = !items.is_empty()
        && items
            .iter()
            .all(|e| e.as_array().is_some_and(|r| r.iter().all(Value::is_array)));
    if nested {
        if items.len() > usize::from(max_rows) {
            return Err(out_of_range(name, "bad_table_shape"));
        }
        for e in items {
            table(name, e, cols, min_rows, max_rows, kind)?;
        }
        return Ok(v.clone());
    }
    let rows = items.len();
    if (rows as u64) < u64::from(min_rows) || (rows as u64) > u64::from(max_rows) {
        return Err(out_of_range(name, "bad_table_shape"));
    }
    for row in items {
        let row = row
            .as_array()
            .ok_or_else(|| out_of_range(name, "bad_table_shape"))?;
        let width_ok = if cols == 0 {
            (2..=10).contains(&row.len())
        } else {
            row.len() == usize::from(cols)
        };
        if !width_ok {
            return Err(out_of_range(name, "bad_table_shape"));
        }
        for c in row {
            cell(name, kind, c)?;
        }
    }
    Ok(v.clone())
}

fn validate_kind(name: &str, kind: &InputKind, v: &Value) -> Result<Value, EngineError> {
    match kind {
        InputKind::Count => count(name, v),
        InputKind::Integer { min, max } => integer(name, v, *min, *max),
        InputKind::Real { min, max } => real(name, v, *min, *max),
        InputKind::Proportion => real(name, v, 0.0, 1.0).and_then(|x| {
            let f = x.as_f64().unwrap_or(0.0);
            if f > 0.0 && f < 1.0 {
                Ok(x)
            } else {
                Err(out_of_range(name, "out_of_range"))
            }
        }),
        InputKind::Percent => real(name, v, 0.0, 100.0),
        InputKind::ConfLevel => {
            let n = v
                .as_u64()
                .filter(|n| matches!(*n, 90 | 95 | 99))
                .ok_or_else(|| out_of_range(name, "bad_conf_level"))?;
            Ok(Value::from(n))
        }
        InputKind::Choice { options } => {
            let s = v
                .as_str()
                .filter(|s| options.contains(s))
                .ok_or_else(|| out_of_range(name, "bad_choice"))?;
            Ok(Value::from(s))
        }
        InputKind::Table {
            cols,
            min_rows,
            max_rows,
            cell,
        } => table(name, v, *cols, *min_rows, *max_rows, *cell),
    }
}

/// Validates `inputs` against `def`: rejects unknown keys, fills
/// defaults, type-checks every declared input. The canonical object is
/// what the calculator sees and what the receipt seals.
pub fn validate(def: &CalculatorDef, inputs: &Value) -> Result<Value, EngineError> {
    let map = match inputs.as_object() {
        Some(m) => m,
        None => {
            return Err(EngineError::CalcInputOutOfRange {
                input: String::new(),
                reason: "not_an_object".into(),
            })
        }
    };
    for key in map.keys() {
        if !def.inputs.iter().any(|i| i.name == key) {
            return Err(out_of_range(key, "unknown_input"));
        }
    }
    let mut out = Map::with_capacity(def.inputs.len());
    for input in def.inputs {
        match map.get(input.name) {
            Some(v) => {
                out.insert(
                    input.name.into(),
                    validate_kind(input.name, &input.kind, v)?,
                );
            }
            None => match &input.default {
                Some(DefaultValue::Number(n)) => {
                    // Canonical JSON: integral defaults stay integers.
                    let v = if n.fract() == 0.0 && n.abs() < 9.007_199_254_740_992e15 {
                        Value::from(*n as i64)
                    } else {
                        Value::from(*n)
                    };
                    out.insert(input.name.into(), v);
                }
                Some(DefaultValue::Choice(s)) => {
                    out.insert(input.name.into(), Value::from(*s));
                }
                Some(DefaultValue::Omit) => {}
                None => {
                    return Err(EngineError::CalcInputMissing {
                        input: input.name.into(),
                    })
                }
            },
        }
    }
    Ok(Value::Object(out))
}
