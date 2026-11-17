//! `parity_norm` — the corpus parity line and its build-normalized
//! form (v1.0 Task 10, owner decision D1). The parity line is the
//! canonical JSON of `{id, actual, receipt}` with every f64
//! replaced by its 16-hex-digit bit pattern (spec §5.2) — the line
//! `corpus-runner` prints and the wasm parity run must reproduce.
//! The normalized form pins the receipt's `engine.git_sha` to
//! `"unknown"` and reseals `receipt_sha256` — exactly what a
//! `GIT_SHA`-less build emits. `git_sha` is build metadata fixed at
//! compile time (spec §4.3), not a result, so `results_unchanged`
//! and `--hash-golden` compare receipts this way.

use super::super::receipt::{canonical_json, canonical_sha256_hex};
use serde_json::{json, Value};

/// The parity line of one case — see the module doc.
pub(crate) fn parity_line(id: &str, actual: &Value, receipt: Option<String>) -> String {
    line(id, actual, receipt)
}

/// `parity_line` with the receipt build-normalized — see the
/// module doc (v1.0 Task 10, owner decision D1).
pub(crate) fn parity_line_normalized(id: &str, actual: &Value, receipt: Option<&str>) -> String {
    line(id, actual, receipt.map(normalize_receipt))
}

fn line(id: &str, actual: &Value, receipt: Option<String>) -> String {
    canonical_json(&json!({
        "id": id,
        "actual": encode_bits(actual),
        "receipt": receipt,
    }))
}

/// The canonical receipt a `GIT_SHA`-less build seals: parse the
/// receipt JSON, set `engine.git_sha` to `"unknown"` and reseal
/// `receipt_sha256` over the normalized receipt as
/// `Receipt::sha256_hex` does — structural normalization only; every
/// other byte is untouched.
fn normalize_receipt(receipt: &str) -> String {
    let Ok(mut v) = serde_json::from_str::<Value>(receipt) else {
        return receipt.to_string();
    };
    if !v.is_object() {
        return canonical_json(&v);
    }
    if let Some(engine) = v.get_mut("engine").and_then(Value::as_object_mut) {
        engine.insert("git_sha".to_string(), json!("unknown"));
    }
    if v.get("receipt_sha256").is_some() {
        let mut unsigned = v.clone();
        if let Value::Object(m) = &mut unsigned {
            m.remove("receipt_sha256");
        }
        let seal = canonical_sha256_hex(&unsigned);
        if let Value::Object(m) = &mut v {
            m.insert("receipt_sha256".to_string(), Value::String(seal));
        }
    }
    canonical_json(&v)
}

/// Replaces every f64 in `v` by its 16-hex-digit bit pattern — the
/// parity encoding of `actual` (spec §5.2), shared by the explain
/// parity lines of v1.0 Task 11.
pub(crate) fn encode_bits(v: &Value) -> Value {
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
