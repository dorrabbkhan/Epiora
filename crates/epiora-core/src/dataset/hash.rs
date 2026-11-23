//! The canonical byte encoding behind the content hash (spec §3.3):
//! format byte `0x01`, column count (u32 LE), then per column the name
//! (u32 LE length + UTF-8), type tag (u8), row count (u64 LE), then a
//! presence byte (0/1) per value followed, if present, by the value
//! (i64 LE; f64 bit pattern LE with `-0.0` normalised to `0.0`; u32 LE
//! length + UTF-8; u8 0/1; i32 LE). `id` and `version` are not hashed.
//!
//! Hash rule (v1.1 spec §4.3, recorded in `docs/decisions.md`): a
//! dataset with no `Time`/`DateTime` column and no stored label set
//! hashes exactly those v1 bytes. Otherwise a `LBLS` section is
//! appended — the magic, the label-set count (u32 LE), then each set
//! in id order: id string, entry count (u32 LE), and per entry the
//! value (tag u8 + the same value bytes a present cell writes) and
//! its label (u32 LE length + UTF-8). Prompts, legacy types and
//! legacy names never enter the hash.

use super::{ColumnData, Dataset, Value};

const FORMAT: u8 = 0x01;

pub(super) fn canonical_bytes(dataset: &Dataset) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(FORMAT);
    u32(&mut out, dataset.columns().len() as u32);
    for column in dataset.columns() {
        string(&mut out, &column.name);
        out.push(column.data.value_type().tag());
        u64(&mut out, column.data.len() as u64);
        match &column.data {
            ColumnData::Integer(values) => {
                for v in values {
                    presence(&mut out, v, |out, x| {
                        out.extend_from_slice(&x.to_le_bytes())
                    });
                }
            }
            ColumnData::Real(values) => {
                for v in values {
                    presence(&mut out, v, |out, x| {
                        // -0.0 normalises to 0.0 so both hash identically.
                        let normalised = if *x == 0.0 { 0.0 } else { *x };
                        out.extend_from_slice(&normalised.to_le_bytes());
                    });
                }
            }
            ColumnData::Text(values) => {
                for v in values {
                    presence(&mut out, v, |out, s| string(out, s));
                }
            }
            ColumnData::Boolean(values) => {
                for v in values {
                    presence(&mut out, v, |out, b| out.push(u8::from(*b)));
                }
            }
            ColumnData::Date(values) => {
                for v in values {
                    presence(&mut out, v, |out, d| {
                        out.extend_from_slice(&d.to_le_bytes())
                    });
                }
            }
            ColumnData::Time(values) => {
                for v in values {
                    presence(&mut out, v, |out, t| {
                        out.extend_from_slice(&t.to_le_bytes())
                    });
                }
            }
            ColumnData::DateTime(values) => {
                for v in values {
                    presence(&mut out, v, |out, t| {
                        out.extend_from_slice(&t.to_le_bytes())
                    });
                }
            }
        }
    }
    if dataset.hash_needs_lbls() {
        out.extend_from_slice(b"LBLS");
        u32(&mut out, dataset.label_sets().len() as u32);
        for (id, set) in dataset.label_sets() {
            string(&mut out, id);
            u32(&mut out, set.entries.len() as u32);
            for (value, label) in &set.entries {
                write_value(&mut out, value);
                string(&mut out, label);
            }
        }
    }
    out
}

/// The canonical bytes of one present value: its type tag (u8) then
/// the same bytes a present cell of that type writes in the column
/// encoding (v1.1 `LBLS` entries and snapshot v2 label sets).
pub(super) fn write_value(out: &mut Vec<u8>, value: &Value) {
    out.push(match value {
        Value::Integer(_) => 1,
        Value::Real(_) => 2,
        Value::Text(_) => 3,
        Value::Boolean(_) => 4,
        Value::Date(_) => 5,
        Value::Time(_) => 6,
        Value::DateTime(_) => 7,
    });
    match value {
        Value::Integer(x) | Value::DateTime(x) => out.extend_from_slice(&x.to_le_bytes()),
        Value::Real(x) => {
            let normalised = if *x == 0.0 { 0.0 } else { *x };
            out.extend_from_slice(&normalised.to_le_bytes());
        }
        Value::Text(s) => string(out, s),
        Value::Boolean(b) => out.push(u8::from(*b)),
        Value::Date(d) | Value::Time(d) => out.extend_from_slice(&d.to_le_bytes()),
    }
}

fn presence<T>(out: &mut Vec<u8>, value: &Option<T>, write: impl Fn(&mut Vec<u8>, &T)) {
    match value {
        Some(v) => {
            out.push(1);
            write(out, v);
        }
        None => out.push(0),
    }
}

fn string(out: &mut Vec<u8>, s: &str) {
    u32(out, s.len() as u32);
    out.extend_from_slice(s.as_bytes());
}

fn u32(out: &mut Vec<u8>, n: u32) {
    out.extend_from_slice(&n.to_le_bytes());
}

fn u64(out: &mut Vec<u8>, n: u64) {
    out.extend_from_slice(&n.to_le_bytes());
}
