//! The canonical byte encoding behind the content hash (spec §3.3):
//! format byte `0x01`, column count (u32 LE), then per column the name
//! (u32 LE length + UTF-8), type tag (u8), row count (u64 LE), then a
//! presence byte (0/1) per value followed, if present, by the value
//! (i64 LE; f64 bit pattern LE with `-0.0` normalised to `0.0`; u32 LE
//! length + UTF-8; u8 0/1; i32 LE). `id` and `version` are not hashed.

use super::{ColumnData, Dataset};

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
        }
    }
    out
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
