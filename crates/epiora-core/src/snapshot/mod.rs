//! Columnar snapshot codec (v0.3 spec §5.1, v1.1 spec §4.3): the
//! derived cache a store keeps per dataset version. Format 1 (v1.rs)
//! is the v0.3 layout; format 2 (v2.rs) adds `Time`/`DateTime` type
//! tags, a per-column meta block and a trailing label-set section.
//! `encode_snapshot` writes v1 when the dataset has no v2 features, so
//! stored snapshots and parity bytes are unchanged; `decode_snapshot`
//! reads both formats.
//!
//! Shared layout, all little-endian: magic `EPSN`, format u8, column
//! count u32, row count u64; then per column: name (u32 length +
//! UTF-8), type tag u8 (the §3.3 hash tags 1–7), a presence bitmap of
//! `ceil(rows/8)` bytes (bit `i % 8` of byte `i / 8` set = present),
//! then values for every row — missing rows hold zero. Integer i64;
//! Real f64 bits with `-0.0` written as `0.0`; Boolean u8 (0/1); Date
//! i32; Time i32; DateTime i64; Text: `rows + 1` u32 offsets then the
//! concatenated UTF-8 bytes.
//!
//! Decoding is strict about the canonical form — unused bitmap bits,
//! non-zero value slots under missing rows, `-0.0` real bits and
//! non-empty text spans under missing rows are all `SnapshotDecode` —
//! so every accepted input re-encodes byte-for-byte (the property the
//! `snapshot_decode` fuzz target checks). `Dataset::new` runs last, so
//! its rules apply (`InvalidColumn`, `EmptyDataset`).

mod v1;
mod v2;

use crate::dataset::{Column, ColumnData, Dataset};
use crate::EngineError;

/// Snapshot magic bytes.
pub const SNAPSHOT_MAGIC: [u8; 4] = *b"EPSN";
/// Snapshot format 1's version byte.
pub const SNAPSHOT_FORMAT: u8 = 1;

/// Encodes a dataset into its canonical snapshot bytes: format 1 when
/// the dataset has no v2 features (`Time`/`DateTime` columns, meta or
/// label sets), else format 2.
pub fn encode_snapshot(ds: &Dataset) -> Vec<u8> {
    if ds.has_v2_features() {
        v2::encode(ds)
    } else {
        v1::encode(ds)
    }
}

/// Decodes snapshot bytes of format 1 or 2 into a validated `Dataset`
/// (strict canonical form; `Dataset::new` rules apply).
pub fn decode_snapshot(
    id: impl Into<String>,
    version: u64,
    bytes: &[u8],
) -> Result<Dataset, EngineError> {
    let mut r = Reader { bytes, pos: 0 };
    if r.take(SNAPSHOT_MAGIC.len())? != SNAPSHOT_MAGIC {
        return Err(snap("bad magic"));
    }
    let format = r.u8()?;
    match format {
        1 => v1::decode_body(id, version, &mut r),
        2 => v2::decode_body(id, version, &mut r),
        other => Err(snap(format!("unknown format {other}"))),
    }
}

fn is_present(data: &ColumnData, row: usize) -> bool {
    match data {
        ColumnData::Integer(v) => v[row].is_some(),
        ColumnData::Real(v) => v[row].is_some(),
        ColumnData::Text(v) => v[row].is_some(),
        ColumnData::Boolean(v) => v[row].is_some(),
        ColumnData::Date(v) => v[row].is_some(),
        ColumnData::Time(v) => v[row].is_some(),
        ColumnData::DateTime(v) => v[row].is_some(),
    }
}

fn write_bitmap(out: &mut Vec<u8>, rows: usize, present: impl Fn(usize) -> bool) {
    let mut bitmap = vec![0u8; rows.div_ceil(8)];
    for i in 0..rows {
        if present(i) {
            bitmap[i / 8] |= 1 << (i % 8);
        }
    }
    out.extend_from_slice(&bitmap);
}

fn snap(reason: impl Into<String>) -> EngineError {
    EngineError::SnapshotDecode {
        reason: reason.into(),
    }
}

fn nonzero_missing(name: &str, row: usize) -> EngineError {
    snap(format!(
        "column {name}: non-zero value bytes under missing row {row}"
    ))
}

/// Writes one column's shared body (both formats): name, type tag,
/// presence bitmap and values for every row.
fn write_column(out: &mut Vec<u8>, column: &Column) {
    out.extend_from_slice(&(column.name.len() as u32).to_le_bytes());
    out.extend_from_slice(column.name.as_bytes());
    out.push(column.data.value_type().tag());
    write_bitmap(out, column.data.len(), |i| is_present(&column.data, i));
    match &column.data {
        ColumnData::Integer(values) => {
            for v in values {
                out.extend_from_slice(&v.unwrap_or(0).to_le_bytes());
            }
        }
        ColumnData::Real(values) => {
            for v in values {
                // -0.0 normalises to 0.0, matching canonical_bytes.
                let x = match v {
                    Some(x) if *x != 0.0 => *x,
                    _ => 0.0,
                };
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
        ColumnData::Boolean(values) => {
            for v in values {
                out.push(u8::from(v.unwrap_or(false)));
            }
        }
        ColumnData::Date(values) => {
            for v in values {
                out.extend_from_slice(&v.unwrap_or(0).to_le_bytes());
            }
        }
        ColumnData::Time(values) => {
            for v in values {
                out.extend_from_slice(&v.unwrap_or(0).to_le_bytes());
            }
        }
        ColumnData::DateTime(values) => {
            for v in values {
                out.extend_from_slice(&v.unwrap_or(0).to_le_bytes());
            }
        }
        ColumnData::Text(values) => {
            let mut offsets = Vec::with_capacity(values.len() + 1);
            let mut blob = Vec::new();
            offsets.push(0u32);
            for v in values {
                if let Some(s) = v {
                    blob.extend_from_slice(s.as_bytes());
                }
                offsets.push(blob.len() as u32);
            }
            for offset in offsets {
                out.extend_from_slice(&offset.to_le_bytes());
            }
            out.extend_from_slice(&blob);
        }
    }
}

/// Reads a column's name and value data under the bitmap; the meta
/// block (format 2) is the caller's to read next. `v2_types` admits
/// the `Time`/`DateTime` tags — a format-1 stream rejects them so an
/// accepted v1 input always re-encodes byte-identically.
fn read_data_column(
    r: &mut Reader<'_>,
    rows: usize,
    v2_types: bool,
) -> Result<(String, ColumnData), EngineError> {
    let name_len = r.u32()? as usize;
    let name = std::str::from_utf8(r.take(name_len)?)
        .map_err(|_| snap("column name is not UTF-8"))?
        .to_string();
    let tag = r.u8()?;
    let bitmap = r.take(rows.div_ceil(8))?;
    if !rows.is_multiple_of(8) && bitmap[bitmap.len() - 1] >> (rows % 8) != 0 {
        return Err(snap(format!("column {name}: non-zero padding bitmap bits")));
    }
    let present = |i: usize| bitmap[i / 8] & (1 << (i % 8)) != 0;
    let data = match tag {
        1 => ColumnData::Integer(read_values(r, rows, present, &name, |r| {
            r.take(8).map(i64::from_le_bytes_array)
        })?),
        2 => {
            let mut values = Vec::new();
            for i in 0..rows {
                let bits = r.take(8).map(u64::from_le_bytes_array)?;
                if !present(i) {
                    if bits != 0 {
                        return Err(nonzero_missing(&name, i));
                    }
                    values.push(None);
                } else {
                    if bits == (-0.0f64).to_bits() {
                        return Err(snap(format!(
                            "column {name}: -0.0 bits at row {i} are not canonical"
                        )));
                    }
                    values.push(Some(f64::from_bits(bits)));
                }
            }
            ColumnData::Real(values)
        }
        3 => {
            let count = rows
                .checked_add(1)
                .ok_or_else(|| snap("row count overflows"))?;
            let mut offsets = Vec::with_capacity(count.min(1024));
            for _ in 0..count {
                offsets.push(r.u32()?);
            }
            if offsets[0] != 0 {
                return Err(snap(format!("column {name}: first offset is not 0")));
            }
            if offsets.windows(2).any(|w| w[0] > w[1]) {
                return Err(snap(format!("column {name}: offsets are not monotonic")));
            }
            let blob = r.take(*offsets.last().unwrap_or(&0) as usize)?;
            let mut values = Vec::with_capacity(rows.min(1024));
            for i in 0..rows {
                let span = &blob[offsets[i] as usize..offsets[i + 1] as usize];
                if !present(i) {
                    if !span.is_empty() {
                        return Err(nonzero_missing(&name, i));
                    }
                    values.push(None);
                } else {
                    let s = std::str::from_utf8(span)
                        .map_err(|_| snap(format!("column {name}: invalid UTF-8 at row {i}")))?;
                    values.push(Some(s.to_string()));
                }
            }
            ColumnData::Text(values)
        }
        4 => {
            let mut values = Vec::with_capacity(rows.min(1024));
            for i in 0..rows {
                let byte = r.u8()?;
                if byte > 1 || (!present(i) && byte != 0) {
                    return Err(snap(format!(
                        "column {name}: boolean byte {byte} at row {i} is not canonical"
                    )));
                }
                values.push(present(i).then_some(byte == 1));
            }
            ColumnData::Boolean(values)
        }
        5 => ColumnData::Date(read_values(r, rows, present, &name, |r| {
            r.take(4).map(i32::from_le_bytes_array)
        })?),
        6 if v2_types => ColumnData::Time(read_values(r, rows, present, &name, |r| {
            r.take(4).map(i32::from_le_bytes_array)
        })?),
        7 if v2_types => ColumnData::DateTime(read_values(r, rows, present, &name, |r| {
            r.take(8).map(i64::from_le_bytes_array)
        })?),
        other => return Err(snap(format!("unknown type tag {other}"))),
    };
    Ok((name, data))
}

/// Reads a fixed-width value per row; a missing row must hold zero.
fn read_values<T: Default + PartialEq + Copy>(
    r: &mut Reader<'_>,
    rows: usize,
    present: impl Fn(usize) -> bool,
    name: &str,
    read: impl Fn(&mut Reader<'_>) -> Result<T, EngineError>,
) -> Result<Vec<Option<T>>, EngineError> {
    let mut values = Vec::with_capacity(rows.min(1024));
    for i in 0..rows {
        let v = read(r)?;
        if !present(i) {
            if v != T::default() {
                return Err(nonzero_missing(name, i));
            }
            values.push(None);
        } else {
            values.push(Some(v));
        }
    }
    Ok(values)
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], EngineError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| snap("length overflows usize"))?;
        if end > self.bytes.len() {
            return Err(snap("truncated"));
        }
        let out = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, EngineError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, EngineError> {
        self.take(4).map(u32::from_le_bytes_array)
    }

    fn u64(&mut self) -> Result<u64, EngineError> {
        self.take(8).map(u64::from_le_bytes_array)
    }

    /// A `u32` length + UTF-8 string.
    fn string(&mut self) -> Result<String, EngineError> {
        let len = self.u32()? as usize;
        Ok(std::str::from_utf8(self.take(len)?)
            .map_err(|_| snap("string is not UTF-8"))?
            .to_string())
    }
}

fn u64_le(bytes: &[u8]) -> u64 {
    u64::from_le_bytes_array(bytes)
}

fn i64_le(bytes: &[u8]) -> i64 {
    i64::from_le_bytes_array(bytes)
}

fn i32_le(bytes: &[u8]) -> i32 {
    i32::from_le_bytes_array(bytes)
}

trait FromLeBytes: Sized {
    fn from_le_bytes_array(bytes: &[u8]) -> Self;
}

impl FromLeBytes for u32 {
    fn from_le_bytes_array(bytes: &[u8]) -> Self {
        let mut b = [0u8; 4];
        b.copy_from_slice(bytes);
        u32::from_le_bytes(b)
    }
}

impl FromLeBytes for u64 {
    fn from_le_bytes_array(bytes: &[u8]) -> Self {
        let mut b = [0u8; 8];
        b.copy_from_slice(bytes);
        u64::from_le_bytes(b)
    }
}

impl FromLeBytes for i32 {
    fn from_le_bytes_array(bytes: &[u8]) -> Self {
        let mut b = [0u8; 4];
        b.copy_from_slice(bytes);
        i32::from_le_bytes(b)
    }
}

impl FromLeBytes for i64 {
    fn from_le_bytes_array(bytes: &[u8]) -> Self {
        let mut b = [0u8; 8];
        b.copy_from_slice(bytes);
        i64::from_le_bytes(b)
    }
}
