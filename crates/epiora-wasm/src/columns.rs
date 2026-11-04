//! Column transfer decoding (spec §5.1): each JS column descriptor
//! becomes a `dataset::Column`. This is the only place typed arrays are
//! read; it is type conversion, never statistics.
//!
//! Encoding: `integer` → `BigInt64Array` + `Uint8Array` presence mask,
//! `real` → `Float64Array` + mask, `date` → `Int32Array` + mask,
//! `boolean` → `Uint8Array` (0, 1, 255 = missing), `text` → dictionary
//! `string[]` + `Uint32Array` codes (`0xFFFFFFFF` = missing). A presence
//! byte of 0 marks a missing entry.

use epiora_core::dataset::{Column, ColumnData};
use epiora_core::EngineError;
use js_sys::{Array, BigInt64Array, Float64Array, Int32Array, Reflect, Uint32Array, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};

const MISSING_TEXT_CODE: u32 = u32::MAX;

fn invalid(column: &str, reason: impl Into<String>) -> EngineError {
    EngineError::InvalidColumn {
        column: column.to_string(),
        reason: reason.into(),
    }
}

fn field(object: &JsValue, name: &str, column: &str) -> Result<JsValue, EngineError> {
    Reflect::get(object, &JsValue::from_str(name))
        .map_err(|_| invalid(column, format!("cannot read field {name:?}")))
}

fn string_field(object: &JsValue, name: &str) -> Result<String, EngineError> {
    let value = Reflect::get(object, &JsValue::from_str(name))
        .map_err(|_| invalid("?", format!("cannot read field {name:?}")))?;
    value
        .as_string()
        .ok_or_else(|| invalid("?", format!("field {name:?} is not a string")))
}

fn typed<T: JsCast>(object: &JsValue, name: &str, column: &str) -> Result<T, EngineError> {
    field(object, name, column)?.dyn_into::<T>().map_err(|_| {
        invalid(
            column,
            format!("field {name:?} has the wrong typed-array kind"),
        )
    })
}

/// Reads the `presence` mask and checks it covers every element.
fn presence(object: &JsValue, column: &str, len: usize) -> Result<Vec<u8>, EngineError> {
    let mask = typed::<Uint8Array>(object, "presence", column)?.to_vec();
    if mask.len() != len {
        return Err(invalid(
            column,
            format!("presence mask has {} entries, expected {len}", mask.len()),
        ));
    }
    Ok(mask)
}

fn masked<T: Copy>(values: Vec<T>, mask: Vec<u8>) -> Vec<Option<T>> {
    values
        .into_iter()
        .zip(mask)
        .map(|(v, p)| (p != 0).then_some(v))
        .collect()
}

fn integer(object: &JsValue, column: &str) -> Result<ColumnData, EngineError> {
    let values = typed::<BigInt64Array>(object, "values", column)?.to_vec();
    let mask = presence(object, column, values.len())?;
    Ok(ColumnData::Integer(masked(values, mask)))
}

fn real(object: &JsValue, column: &str) -> Result<ColumnData, EngineError> {
    let values = typed::<Float64Array>(object, "values", column)?.to_vec();
    let mask = presence(object, column, values.len())?;
    Ok(ColumnData::Real(masked(values, mask)))
}

fn date(object: &JsValue, column: &str) -> Result<ColumnData, EngineError> {
    let values = typed::<Int32Array>(object, "values", column)?.to_vec();
    let mask = presence(object, column, values.len())?;
    Ok(ColumnData::Date(masked(values, mask)))
}

fn boolean(object: &JsValue, column: &str) -> Result<ColumnData, EngineError> {
    let raw = typed::<Uint8Array>(object, "values", column)?.to_vec();
    let mut values = Vec::with_capacity(raw.len());
    for (i, code) in raw.iter().enumerate() {
        values.push(match code {
            0 => Some(false),
            1 => Some(true),
            255 => None,
            other => {
                return Err(invalid(
                    column,
                    format!("boolean code {other} at index {i} is not 0, 1 or 255"),
                ));
            }
        });
    }
    Ok(ColumnData::Boolean(values))
}

fn text(object: &JsValue, column: &str) -> Result<ColumnData, EngineError> {
    let dictionary = field(object, "dictionary", column)?;
    let dictionary = dictionary
        .dyn_into::<Array>()
        .map_err(|_| invalid(column, "field \"dictionary\" is not an array"))?;
    let mut strings = Vec::with_capacity(dictionary.length() as usize);
    for i in 0..dictionary.length() {
        strings.push(
            dictionary
                .get(i)
                .as_string()
                .ok_or_else(|| invalid(column, format!("dictionary entry {i} is not a string")))?,
        );
    }
    let codes = typed::<Uint32Array>(object, "codes", column)?.to_vec();
    let mut values = Vec::with_capacity(codes.len());
    for (i, code) in codes.iter().enumerate() {
        values.push(match code {
            &MISSING_TEXT_CODE => None,
            &code if (code as usize) < strings.len() => Some(strings[code as usize].clone()),
            other => {
                return Err(invalid(
                    column,
                    format!(
                        "text code {other} at index {i} is outside the dictionary ({} entries)",
                        strings.len()
                    ),
                ));
            }
        });
    }
    Ok(ColumnData::Text(values))
}

fn column(object: &JsValue, index: u32) -> Result<Column, EngineError> {
    let name = string_field(object, "name").map_err(|_| {
        invalid(
            "?",
            format!("column {index}: 'name' is missing or not a string"),
        )
    })?;
    let kind = string_field(object, "type")
        .map_err(|_| invalid(&name, "field 'type' is missing or not a string"))?;
    let data = match kind.as_str() {
        "integer" => integer(object, &name)?,
        "real" => real(object, &name)?,
        "text" => text(object, &name)?,
        "boolean" => boolean(object, &name)?,
        "date" => date(object, &name)?,
        other => return Err(invalid(&name, format!("unknown column type {other:?}"))),
    };
    Ok(Column { name, data })
}

/// Converts the JS `ColumnInput[]` argument of `datasetFromColumns`.
pub fn convert(columns: &Array) -> Result<Vec<Column>, EngineError> {
    let mut out = Vec::with_capacity(columns.length() as usize);
    for i in 0..columns.length() {
        out.push(column(&columns.get(i), i)?);
    }
    Ok(out)
}
