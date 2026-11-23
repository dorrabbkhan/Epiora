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
        let s = self.take(2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }
    fn u32(&mut self) -> Result<u32, LegacyError> {
        let s = self.take(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn f64(&mut self) -> Result<f64, LegacyError> {
        let s = self.take(8)?;
        let mut b = [0u8; 8];
        b.copy_from_slice(s);
        Ok(f64::from_le_bytes(b))
    }
    fn i64(&mut self) -> Result<i64, LegacyError> {
        let s = self.take(8)?;
        let mut b = [0u8; 8];
        b.copy_from_slice(s);
        Ok(i64::from_le_bytes(b))
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
mod tests;
