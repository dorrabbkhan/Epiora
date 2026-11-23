//! EPTB v1 batch decoding (v1.1 plan Task 5). One batch is a chunk of a
//! legacy table emitted by `epiora-mdb` (native or wasm), little-endian:
//!
//! ```text
//! "EPTB" u16 ver=1 | u32 row_count | u16 column_count
//! u16 ext_len, then { u8 tag, u16 len, value }* extension fields:
//!   tag 1 = table name (UTF-8, required)
//!   tag 2 = code page used for JET3 text (UTF-8)
//!   tag 3 = u32 count of values emitted with decode errors
//! per column: u32 name_len + UTF-8 name | u8 source_type (Access type
//!   code) | u8 kind (0 i64, 1 f64, 2 text, 3 bool, 4 Access-days
//!   datetime f64, 5 binary-skipped)
//! kinds 0-4: missing bitmap ceil(row_count/8), bit set = missing,
//!   then present-only values: kind 0 i64, kinds 1/4 f64, kind 3 u8,
//!   kind 2 u32 blob_len + blob + u32[present] offsets into the blob
//!   of NUL-terminated UTF-8 strings
//! ```

use crate::error::LegacyError;

/// A decoded batch: one chunk of a legacy table.
#[derive(Debug, Clone, PartialEq)]
pub struct RawTable {
    /// Source table name (extension tag 1).
    pub name: String,
    /// The code page JET3 text was decoded with (extension tag 2), so
    /// the report can name the page assumed.
    pub code_page: Option<String>,
    /// Values emitted with decode errors — U+FFFD substitution or a
    /// cell that could not be read as its declared kind (tag 3).
    pub decode_errors: u32,
    /// Columns in source order.
    pub columns: Vec<RawColumn>,
}

/// One column of a batch.
#[derive(Debug, Clone, PartialEq)]
pub struct RawColumn {
    /// Column name.
    pub name: String,
    /// The Access type code the reader saw.
    pub source_type: u8,
    /// Typed values; every variant's Vec has `row_count` entries.
    pub data: RawData,
}

/// Typed column data (variant = the batch's kind byte).
#[derive(Debug, Clone, PartialEq)]
pub enum RawData {
    /// kind 0.
    Int(Vec<Option<i64>>),
    /// kind 1.
    Float(Vec<Option<f64>>),
    /// kind 2.
    Text(Vec<Option<String>>),
    /// kind 3.
    Bool(Vec<Option<bool>>),
    /// kind 4: Access serial (f64 days since 1899-12-30, fraction = time).
    DateTime(Vec<Option<f64>>),
    /// kind 5: binary column the reader does not export.
    BinarySkipped,
}

fn invalid(reason: &str) -> LegacyError {
    LegacyError::BatchInvalid {
        reason: reason.to_string(),
    }
}

struct Cur<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cur<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], LegacyError> {
        if self.b.len() - self.p < n {
            return Err(invalid("truncated batch"));
        }
        let s = &self.b[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, LegacyError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, LegacyError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, LegacyError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> Result<f64, LegacyError> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64, LegacyError> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn utf8(&mut self, n: usize) -> Result<String, LegacyError> {
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| invalid("invalid UTF-8"))
    }
}

fn collect_vals<T>(
    present: impl Iterator<Item = bool>,
    mut read: impl FnMut() -> Result<T, LegacyError>,
) -> Result<Vec<Option<T>>, LegacyError> {
    present
        .map(|p| if p { read().map(Some) } else { Ok(None) })
        .collect()
}

/// Decodes one EPTB v1 batch.
pub fn decode_batch(bytes: &[u8]) -> Result<RawTable, LegacyError> {
    let mut c = Cur { b: bytes, p: 0 };
    if c.take(4)? != b"EPTB" {
        return Err(invalid("bad magic"));
    }
    if c.u16()? != 1 {
        return Err(invalid("unsupported EPTB version"));
    }
    let row_count = c.u32()? as usize;
    let col_count = c.u16()? as usize;

    let ext_end = c.p + c.u16()? as usize;
    if ext_end > bytes.len() {
        return Err(invalid("truncated extension"));
    }
    let (mut name, mut code_page, mut decode_errors) = (None, None, 0);
    while c.p < ext_end {
        let tag = c.u8()?;
        let len = c.u16()? as usize;
        match tag {
            1 => name = Some(c.utf8(len)?),
            2 => code_page = Some(c.utf8(len)?),
            3 => {
                if len != 4 {
                    return Err(invalid("bad decode_errors field"));
                }
                decode_errors = c.u32()?;
            }
            _ => {
                c.take(len)?;
            }
        }
    }
    let name = name.ok_or_else(|| invalid("extension has no table name"))?;

    if row_count > (1 << 24) || col_count > 4096 {
        return Err(invalid("implausible counts"));
    }
    let bitmap_len = row_count.div_ceil(8);

    let mut columns = Vec::with_capacity(col_count);
    for _ in 0..col_count {
        let name_len = c.u32()? as usize;
        let cname = c.utf8(name_len)?;
        let source_type = c.u8()?;
        let kind = c.u8()?;
        if kind > 5 {
            return Err(invalid("unknown column kind"));
        }
        if kind == 5 {
            columns.push(RawColumn {
                name: cname,
                source_type,
                data: RawData::BinarySkipped,
            });
            continue;
        }
        let bitmap = c.take(bitmap_len)?.to_vec();
        let present = || (0..row_count).map(|r| bitmap[r / 8] & (1 << (r % 8)) == 0);
        let data = match kind {
            0 => RawData::Int(collect_vals(present(), || c.i64())?),
            1 | 4 => {
                let vals = collect_vals(present(), || c.f64())?;
                if kind == 1 {
                    RawData::Float(vals)
                } else {
                    RawData::DateTime(vals)
                }
            }
            3 => RawData::Bool(collect_vals(present(), || c.u8().map(|b| b != 0))?),
            _ => {
                let blob_len = c.u32()? as usize;
                let blob = c.take(blob_len)?.to_vec();
                let mut offs = Vec::new();
                for p in present() {
                    if p {
                        offs.push(c.u32()? as usize);
                    }
                }
                let mut texts = Vec::with_capacity(row_count);
                let mut it = offs.into_iter();
                for p in present() {
                    if !p {
                        texts.push(None);
                        continue;
                    }
                    let start = it.next().unwrap_or(0);
                    if start >= blob.len() {
                        return Err(invalid("text offset out of blob"));
                    }
                    let end = blob[start..]
                        .iter()
                        .position(|&b| b == 0)
                        .map(|i| start + i)
                        .ok_or_else(|| invalid("unterminated text"))?;
                    texts.push(Some(
                        String::from_utf8(blob[start..end].to_vec())
                            .map_err(|_| invalid("invalid UTF-8"))?,
                    ));
                }
                RawData::Text(texts)
            }
        };
        columns.push(RawColumn {
            name: cname,
            source_type,
            data,
        });
    }
    if c.p != bytes.len() {
        return Err(invalid("trailing bytes"));
    }
    Ok(RawTable {
        name,
        code_page,
        decode_errors,
        columns,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test encoder producing the same EPTB v1 layout the C encoder
    /// writes (kept independent of it on purpose).
    fn encode(table: &RawTable) -> Vec<u8> {
        let rows = table
            .columns
            .iter()
            .map(|c| match &c.data {
                RawData::Int(v) => v.len(),
                RawData::Float(v) => v.len(),
                RawData::Text(v) => v.len(),
                RawData::Bool(v) => v.len(),
                RawData::DateTime(v) => v.len(),
                RawData::BinarySkipped => 0,
            })
            .max()
            .unwrap_or(0);
        let mut b = b"EPTB".to_vec();
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&(rows as u32).to_le_bytes());
        b.extend_from_slice(&(table.columns.len() as u16).to_le_bytes());
        let mut ext = Vec::new();
        ext.push(1u8);
        ext.extend_from_slice(&(table.name.len() as u16).to_le_bytes());
        ext.extend_from_slice(table.name.as_bytes());
        if let Some(cp) = &table.code_page {
            ext.push(2u8);
            ext.extend_from_slice(&(cp.len() as u16).to_le_bytes());
            ext.extend_from_slice(cp.as_bytes());
        }
        ext.push(3u8);
        ext.extend_from_slice(&4u16.to_le_bytes());
        ext.extend_from_slice(&table.decode_errors.to_le_bytes());
        b.extend_from_slice(&(ext.len() as u16).to_le_bytes());
        b.extend_from_slice(&ext);
        for col in &table.columns {
            b.extend_from_slice(&(col.name.len() as u32).to_le_bytes());
            b.extend_from_slice(col.name.as_bytes());
            b.push(col.source_type);
            match &col.data {
                RawData::BinarySkipped => b.push(5),
                RawData::Int(v) => {
                    col_block(&mut b, 0, v, |b, v| b.extend_from_slice(&v.to_le_bytes()))
                }
                RawData::Float(v) | RawData::DateTime(v) => {
                    let kind = if matches!(col.data, RawData::DateTime(_)) {
                        4
                    } else {
                        1
                    };
                    col_block(&mut b, kind, v, |b, v| {
                        b.extend_from_slice(&v.to_le_bytes())
                    });
                }
                RawData::Bool(v) => col_block(&mut b, 3, v, |b, v| b.push(*v as u8)),
                RawData::Text(v) => {
                    b.push(2);
                    push_bitmap(&mut b, v);
                    let mut blob = Vec::new();
                    let mut offs = Vec::new();
                    for s in v.iter().flatten() {
                        offs.push(blob.len() as u32);
                        blob.extend_from_slice(s.as_bytes());
                        blob.push(0);
                    }
                    b.extend_from_slice(&(blob.len() as u32).to_le_bytes());
                    b.extend_from_slice(&blob);
                    for o in offs {
                        b.extend_from_slice(&o.to_le_bytes());
                    }
                }
            }
        }
        b
    }

    fn push_bitmap<T>(b: &mut Vec<u8>, vals: &[Option<T>]) {
        let mut bm = vec![0u8; vals.len().div_ceil(8)];
        for (i, v) in vals.iter().enumerate() {
            if v.is_none() {
                bm[i / 8] |= 1 << (i % 8);
            }
        }
        b.extend_from_slice(&bm);
    }

    fn col_block<T, F: Fn(&mut Vec<u8>, &T)>(
        b: &mut Vec<u8>,
        kind: u8,
        vals: &[Option<T>],
        write: F,
    ) {
        b.push(kind);
        push_bitmap(b, vals);
        for v in vals.iter().flatten() {
            write(b, v);
        }
    }

    #[test]
    fn roundtrip_all_kinds() {
        let table = RawTable {
            name: "T".into(),
            code_page: Some("windows-1252".into()),
            decode_errors: 3,
            columns: vec![
                RawColumn {
                    name: "i".into(),
                    source_type: 0x04,
                    data: RawData::Int(vec![Some(1), None, Some(-5)]),
                },
                RawColumn {
                    name: "f".into(),
                    source_type: 0x07,
                    data: RawData::Float(vec![None, Some(1.5), Some(-2.25)]),
                },
                RawColumn {
                    name: "t".into(),
                    source_type: 0x0a,
                    data: RawData::Text(vec![Some("héllo".into()), None, Some("".into())]),
                },
                RawColumn {
                    name: "b".into(),
                    source_type: 0x01,
                    data: RawData::Bool(vec![Some(true), Some(false), None]),
                },
                RawColumn {
                    name: "d".into(),
                    source_type: 0x08,
                    data: RawData::DateTime(vec![Some(12345.6789), None, Some(0.5)]),
                },
                RawColumn {
                    name: "pic".into(),
                    source_type: 0x0b,
                    data: RawData::BinarySkipped,
                },
            ],
        };
        assert_eq!(decode_batch(&encode(&table)).unwrap(), table);
    }

    #[test]
    fn rejects_bad_magic() {
        assert_eq!(
            decode_batch(b"XPTBxxxx").unwrap_err().code(),
            "legacy_batch_invalid"
        );
        assert_eq!(
            decode_batch(b"EPTB").unwrap_err().code(),
            "legacy_batch_invalid"
        );
        assert_eq!(
            decode_batch(
                &encode(&RawTable {
                    name: "T".into(),
                    code_page: None,
                    decode_errors: 0,
                    columns: vec![RawColumn {
                        name: "i".into(),
                        source_type: 0,
                        data: RawData::Int(vec![Some(1), Some(2), Some(3)]),
                    }],
                })[..20]
            )
            .unwrap_err()
            .code(),
            "legacy_batch_invalid"
        );
    }
}
