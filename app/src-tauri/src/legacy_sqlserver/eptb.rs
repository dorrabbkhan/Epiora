//! The EPTB v1 encoder for the SQL Server reader (v1.1 Task 9) —
//! column accumulation, TDS→EPTB kind mapping and chunk
//! serialisation. Same wire format as
//! `crates/epiora-mdb/c/eptb.c` and the test encoder in
//! `crates/epiora-legacy/tests/common/encode.rs`; kept product-side
//! because the command returns batches over IPC and the worker/test
//! decodes them.

use tiberius::numeric::Numeric;
use tiberius::time::{Date, DateTime, DateTime2, Time};
use tiberius::{Column, ColumnData, ColumnType, Row};

/// A value destined for one EPTB cell.
enum CellVal {
    I(i64),
    F(f64),
    B(bool),
    S(String),
}

/// One output column: the batch metadata plus per-row state.
pub(crate) struct ColAcc {
    name: String,
    source_type: u8,
    kind: u8,
    /// Per-row missing flags, appended every row (bit set = missing).
    miss: Vec<bool>,
    vals: Vec<CellVal>,
}

/// TDS `ColumnType` → the EPTB kind byte. `source_type` carries the
/// TDS discriminant (the mapper compares it only for cross-chunk
/// consistency, never for mapping decisions).
pub(crate) fn col_acc(c: &Column) -> ColAcc {
    use ColumnType as T;
    let kind = match c.column_type() {
        T::Int1 | T::Int2 | T::Int4 | T::Int8 | T::Intn => 0,
        T::Bit | T::Bitn => 3,
        T::Float4 | T::Float8 | T::Floatn | T::Money | T::Money4 | T::Decimaln | T::Numericn => 1,
        T::Datetime4
        | T::Datetime
        | T::Datetimen
        | T::Daten
        | T::Timen
        | T::Datetime2
        | T::DatetimeOffsetn => 4,
        T::Guid
        | T::NVarchar
        | T::NChar
        | T::BigVarChar
        | T::BigChar
        | T::Text
        | T::NText
        | T::Xml => 2,
        // BigVarBin, BigBinary, Image, Udt, SSVariant, Null and
        // anything new: the mdb reader's "binary/unknown" arm.
        _ => 5,
    };
    ColAcc {
        name: c.name().to_string(),
        source_type: c.column_type() as u8,
        kind,
        miss: Vec::new(),
        vals: Vec::new(),
    }
}

/// Access serial days (OLE epoch 1899-12-30) for one TDS datetime
/// value: T-SQL `datetime`/`smalldatetime` days count from 1900-01-01
/// (+2 serial), T-SQL `date`/`datetime2` days count from 0001-01-01
/// (−693593 serial). `smalldatetime`'s wire time part is *minutes*
/// since midnight (the field tiberius names `seconds_fragments`).
/// `datetimeoffset` is normalised to UTC.
fn tds_serial(data: &ColumnData) -> Option<f64> {
    match data {
        ColumnData::DateTime(Some(dt)) => Some(datetime_serial(*dt)),
        ColumnData::SmallDateTime(Some(dt)) => {
            Some(f64::from(dt.days()) + 2.0 + f64::from(dt.seconds_fragments()) / 1440.0)
        }
        ColumnData::Date(Some(d)) => Some(date_serial(*d)),
        ColumnData::Time(Some(t)) => Some(time_serial(*t)),
        ColumnData::DateTime2(Some(dt)) => Some(datetime2_serial(*dt)),
        ColumnData::DateTimeOffset(Some(dt)) => {
            Some(datetime2_serial(dt.datetime2()) - f64::from(dt.offset()) / 1440.0)
        }
        _ => None,
    }
}

fn datetime_serial(dt: DateTime) -> f64 {
    f64::from(dt.days()) + 2.0 + f64::from(dt.seconds_fragments()) / 25_920_000.0
}

fn date_serial(d: Date) -> f64 {
    f64::from(d.days()) - 693_593.0
}

fn time_serial(t: Time) -> f64 {
    t.increments() as f64 * 10f64.powi(-i32::from(t.scale())) / 86_400.0
}

fn datetime2_serial(dt: DateTime2) -> f64 {
    date_serial(dt.date()) + time_serial(dt.time())
}

/// One cell's EPTB value for the column's declared kind; `None` means
/// missing (NULL) or unreadable-as-declared (counted elsewhere).
fn cell_val(kind: u8, data: &ColumnData) -> Option<CellVal> {
    match (kind, data) {
        (0, ColumnData::U8(Some(v))) => Some(CellVal::I(i64::from(*v))),
        (0, ColumnData::I16(Some(v))) => Some(CellVal::I(i64::from(*v))),
        (0, ColumnData::I32(Some(v))) => Some(CellVal::I(i64::from(*v))),
        (0, ColumnData::I64(Some(v))) => Some(CellVal::I(*v)),
        (1, ColumnData::F32(Some(v))) => Some(CellVal::F(f64::from(*v))),
        (1, ColumnData::F64(Some(v))) => Some(CellVal::F(*v)),
        (1, ColumnData::Numeric(Some(v))) => Some(CellVal::F(numeric_f64(*v))),
        (3, ColumnData::Bit(Some(v))) => Some(CellVal::B(*v)),
        (2, ColumnData::String(Some(v))) => Some(CellVal::S(v.to_string())),
        (2, ColumnData::Guid(Some(v))) => Some(CellVal::S(format!("{{{v:X}}}"))),
        (2, ColumnData::Xml(Some(v))) => Some(CellVal::S(v.as_ref().to_string())),
        (4, d) => tds_serial(d).map(CellVal::F),
        _ => None,
    }
}

fn numeric_f64(n: Numeric) -> f64 {
    n.value() as f64 / 10f64.powi(i32::from(n.scale()))
}

fn data_is_null(data: &ColumnData) -> bool {
    match data {
        ColumnData::U8(v) => v.is_none(),
        ColumnData::I16(v) => v.is_none(),
        ColumnData::I32(v) => v.is_none(),
        ColumnData::I64(v) => v.is_none(),
        ColumnData::F32(v) => v.is_none(),
        ColumnData::F64(v) => v.is_none(),
        ColumnData::Bit(v) => v.is_none(),
        ColumnData::String(v) => v.is_none(),
        ColumnData::Guid(v) => v.is_none(),
        ColumnData::Binary(v) => v.is_none(),
        ColumnData::Numeric(v) => v.is_none(),
        ColumnData::Xml(v) => v.is_none(),
        ColumnData::DateTime(v) => v.is_none(),
        ColumnData::SmallDateTime(v) => v.is_none(),
        ColumnData::Time(v) => v.is_none(),
        ColumnData::Date(v) => v.is_none(),
        ColumnData::DateTime2(v) => v.is_none(),
        ColumnData::DateTimeOffset(v) => v.is_none(),
    }
}

/// Accumulates rows column-wise and emits one EPTB chunk per
/// `chunk_rows` rows.
pub(crate) struct ChunkEncoder {
    table: String,
    accs: Vec<ColAcc>,
    /// Rows buffered since the last flush.
    pub(crate) row_count: usize,
    decode_errors: u32,
    /// Finished chunks in emission order.
    pub(crate) chunks: Vec<Vec<u8>>,
}

impl ChunkEncoder {
    pub(crate) fn new(table: &str, accs: Vec<ColAcc>) -> Self {
        Self {
            table: table.to_string(),
            accs,
            row_count: 0,
            decode_errors: 0,
            chunks: Vec::new(),
        }
    }

    pub(crate) fn push_row(&mut self, row: &Row) {
        for (acc, (_, data)) in self.accs.iter_mut().zip(row.cells()) {
            if acc.kind == 5 {
                continue;
            }
            match cell_val(acc.kind, data) {
                Some(v) => {
                    acc.miss.push(false);
                    acc.vals.push(v);
                }
                None => {
                    // A NULL cell is missing, not an error; a cell the
                    // declared kind can't hold is a decode error too
                    // (the mdb reader counts, never aborts).
                    acc.miss.push(true);
                    if !data_is_null(data) {
                        self.decode_errors += 1;
                    }
                }
            }
        }
        self.row_count += 1;
    }

    /// Emit the buffered rows as one EPTB chunk and reset the
    /// accumulators. Called once unconditionally at the end so an
    /// empty table still yields a header-only chunk.
    pub(crate) fn flush(&mut self) {
        if self.row_count == 0 && !self.chunks.is_empty() {
            return;
        }
        let chunk = encode_chunk(&self.table, &self.accs, self.row_count, self.decode_errors);
        self.chunks.push(chunk);
        self.row_count = 0;
        self.decode_errors = 0;
        for acc in &mut self.accs {
            acc.miss.clear();
            acc.vals.clear();
        }
    }
}

fn encode_chunk(table: &str, accs: &[ColAcc], row_count: usize, decode_errors: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut ext = Vec::new();
    ext.push(1u8);
    ext.extend_from_slice(&(table.len() as u16).to_le_bytes());
    ext.extend_from_slice(table.as_bytes());
    ext.push(3u8);
    ext.extend_from_slice(&4u16.to_le_bytes());
    ext.extend_from_slice(&decode_errors.to_le_bytes());

    out.extend_from_slice(b"EPTB");
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(row_count as u32).to_le_bytes());
    out.extend_from_slice(&(accs.len() as u16).to_le_bytes());
    out.extend_from_slice(&(ext.len() as u16).to_le_bytes());
    out.extend_from_slice(&ext);

    let bitmap_len = row_count.div_ceil(8);
    for acc in accs {
        out.extend_from_slice(&(acc.name.len() as u32).to_le_bytes());
        out.extend_from_slice(acc.name.as_bytes());
        out.push(acc.source_type);
        out.push(acc.kind);
        if acc.kind == 5 {
            continue;
        }
        let mut bitmap = vec![0u8; bitmap_len];
        for (r, m) in acc.miss.iter().enumerate() {
            if *m {
                bitmap[r / 8] |= 1 << (r % 8);
            }
        }
        out.extend_from_slice(&bitmap);
        match acc.kind {
            0 => {
                for v in acc.vals.iter() {
                    if let CellVal::I(x) = v {
                        out.extend_from_slice(&x.to_le_bytes());
                    }
                }
            }
            1 | 4 => {
                for v in acc.vals.iter() {
                    if let CellVal::F(x) = v {
                        out.extend_from_slice(&x.to_le_bytes());
                    }
                }
            }
            3 => {
                for v in acc.vals.iter() {
                    if let CellVal::B(b) = v {
                        out.push(u8::from(*b));
                    }
                }
            }
            _ => {
                let mut blob = Vec::new();
                let mut offs = Vec::new();
                for v in acc.vals.iter() {
                    if let CellVal::S(s) = v {
                        offs.push(blob.len() as u32);
                        blob.extend_from_slice(s.as_bytes());
                        blob.push(0);
                    }
                }
                out.extend_from_slice(&(blob.len() as u32).to_le_bytes());
                out.extend_from_slice(&blob);
                for o in offs {
                    out.extend_from_slice(&o.to_le_bytes());
                }
            }
        }
    }
    out
}
