//! Snapshot format 1 (v0.3 spec §5.1): magic `EPSN`, format byte `1`,
//! column count u32, row count u64, then the shared per-column body of
//! `super::write_column` / `super::read_data_column`. Format 1 knows
//! only type tags 1–5 — `encode_snapshot` picks it only for datasets
//! without v2 features, so its byte stream is unchanged from v0.3.

use super::{read_data_column, snap, write_column, Reader, SNAPSHOT_MAGIC};
use crate::dataset::{Column, Dataset};
use crate::EngineError;

pub(super) fn encode(ds: &Dataset) -> Vec<u8> {
    let rows = ds.row_count();
    let mut out = Vec::new();
    out.extend_from_slice(&SNAPSHOT_MAGIC);
    out.push(1);
    out.extend_from_slice(&(ds.columns().len() as u32).to_le_bytes());
    out.extend_from_slice(&(rows as u64).to_le_bytes());
    for column in ds.columns() {
        write_column(&mut out, column);
    }
    out
}

/// Reads the bytes after `EPSN` + format `1`: column count, row count
/// and every column body.
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
    for _ in 0..column_count {
        let (name, data) = read_data_column(r, rows, false)?;
        columns.push(Column { name, data });
    }
    if r.pos != r.bytes.len() {
        return Err(snap(format!("{} trailing bytes", r.bytes.len() - r.pos)));
    }
    Dataset::new(id, version, columns)
}
