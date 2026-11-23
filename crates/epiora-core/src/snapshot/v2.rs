//! Snapshot format 2 (v1.1 spec §4.3): the format-1 header with the
//! version byte `2`, the shared per-column body (type tags 1–7) plus
//! a meta block per column, then a trailing label-set section.
//!
//! Meta block, after a column's values: a flag byte — bit 0 prompt,
//! bit 1 legacy type, bit 2 legacy name, bit 3 label-set id; the high
//! nibble is reserved and must be zero — then each flagged field as a
//! `u32` length + UTF-8, in bit order.
//!
//! Label sets: a `u32` count, then each set in strictly increasing id
//! order (the canonical `BTreeMap` order — an unsorted or duplicated
//! id is `SnapshotDecode`, never silently re-ordered): id string,
//! entry count `u32`, then per entry the `Value` canonical bytes
//! (`tag + value bytes`) and the label as `u32` length + UTF-8.

use super::{i32_le, i64_le, read_data_column, snap, u64_le, write_column, Reader, SNAPSHOT_MAGIC};
use crate::dataset::{Column, ColumnMeta, Dataset, LabelSet, Value};
use crate::EngineError;
use std::collections::BTreeMap;

const PROMPT: u8 = 1;
const LEGACY_TYPE: u8 = 2;
const LEGACY_NAME: u8 = 4;
const LABEL_SET: u8 = 8;
const RESERVED: u8 = !0x0f;

pub(super) fn encode(ds: &Dataset) -> Vec<u8> {
    let rows = ds.row_count();
    let mut out = Vec::new();
    out.extend_from_slice(&SNAPSHOT_MAGIC);
    out.push(2);
    out.extend_from_slice(&(ds.columns().len() as u32).to_le_bytes());
    out.extend_from_slice(&(rows as u64).to_le_bytes());
    for (i, column) in ds.columns().iter().enumerate() {
        write_column(&mut out, column);
        let meta = ds.meta().get(i).cloned().unwrap_or_default();
        write_meta(&mut out, &meta);
    }
    out.extend_from_slice(&(ds.label_sets().len() as u32).to_le_bytes());
    for (id, set) in ds.label_sets() {
        write_string(&mut out, id);
        out.extend_from_slice(&(set.entries.len() as u32).to_le_bytes());
        for (value, label) in &set.entries {
            out.extend_from_slice(&value.canonical_bytes());
            write_string(&mut out, label);
        }
    }
    out
}

fn write_meta(out: &mut Vec<u8>, meta: &ColumnMeta) {
    let mut flags = 0u8;
    flags |= u8::from(meta.prompt.is_some()) * PROMPT;
    flags |= u8::from(meta.legacy_type.is_some()) * LEGACY_TYPE;
    flags |= u8::from(meta.legacy_name.is_some()) * LEGACY_NAME;
    flags |= u8::from(meta.label_set.is_some()) * LABEL_SET;
    out.push(flags);
    for s in [
        &meta.prompt,
        &meta.legacy_type,
        &meta.legacy_name,
        &meta.label_set,
    ]
    .into_iter()
    .flatten()
    {
        write_string(out, s);
    }
}

fn read_meta(r: &mut Reader<'_>) -> Result<ColumnMeta, EngineError> {
    let flags = r.u8()?;
    if flags & RESERVED != 0 {
        return Err(snap(format!("meta flags {flags:#04x} are not canonical")));
    }
    Ok(ColumnMeta {
        prompt: (flags & PROMPT != 0).then(|| r.string()).transpose()?,
        legacy_type: (flags & LEGACY_TYPE != 0).then(|| r.string()).transpose()?,
        legacy_name: (flags & LEGACY_NAME != 0).then(|| r.string()).transpose()?,
        label_set: (flags & LABEL_SET != 0).then(|| r.string()).transpose()?,
    })
}

fn write_string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// One `LBLS`-encoding `Value`: tag u8 then the column value bytes.
fn read_value(r: &mut Reader<'_>) -> Result<Value, EngineError> {
    match r.u8()? {
        1 => Ok(Value::Integer(r.take(8).map(i64_le)?)),
        2 => {
            let bits = r.take(8).map(u64_le)?;
            if bits == (-0.0f64).to_bits() {
                return Err(snap("label value: -0.0 bits are not canonical"));
            }
            Ok(Value::Real(f64::from_bits(bits)))
        }
        3 => Ok(Value::Text(r.string()?)),
        4 => {
            let b = r.u8()?;
            if b > 1 {
                return Err(snap("label value: boolean byte is not canonical"));
            }
            Ok(Value::Boolean(b == 1))
        }
        5 => Ok(Value::Date(r.take(4).map(i32_le)?)),
        6 => Ok(Value::Time(r.take(4).map(i32_le)?)),
        7 => Ok(Value::DateTime(r.take(8).map(i64_le)?)),
        other => Err(snap(format!("label value: unknown type tag {other}"))),
    }
}

/// Reads the bytes after `EPSN` + format `2`: column count, row
/// count, column bodies each followed by a meta block, then the
/// label-set section. `with_meta` validates label-set references and
/// `Time` ranges last.
pub(super) fn decode_body(
    id: impl Into<String>,
    version: u64,
    r: &mut Reader<'_>,
) -> Result<Dataset, EngineError> {
    let column_count = r.u32()? as usize;
    let rows_wide = r.u64()?;
    let rows = usize::try_from(rows_wide)
        .map_err(|_| snap(format!("row count {rows_wide} overflows usize")))?;
    let mut columns = Vec::with_capacity(column_count.min(1024));
    let mut metas = Vec::with_capacity(column_count.min(1024));
    for _ in 0..column_count {
        let (name, data) = read_data_column(r, rows, true)?;
        metas.push(read_meta(r)?);
        columns.push(Column { name, data });
    }
    let set_count = r.u32()? as usize;
    let mut label_sets = BTreeMap::new();
    let mut previous: Option<String> = None;
    for _ in 0..set_count {
        let id = r.string()?;
        if previous.as_deref() >= Some(id.as_str()) {
            return Err(snap("label set ids are not in canonical order"));
        }
        previous = Some(id.clone());
        let entry_count = r.u32()? as usize;
        let mut entries = Vec::with_capacity(entry_count.min(1024));
        for _ in 0..entry_count {
            entries.push((read_value(r)?, r.string()?));
        }
        label_sets.insert(id.clone(), LabelSet { id, entries });
    }
    if r.pos != r.bytes.len() {
        return Err(snap(format!("{} trailing bytes", r.bytes.len() - r.pos)));
    }
    Dataset::new(id, version, columns)?.with_meta(metas, label_sets)
}
