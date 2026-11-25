//! `legacy_meta_any` (spec §6.5, plan Task 7): arbitrary bytes drive
//! the construction of arbitrary meta tables, each encoded back to an
//! EPTB batch, pushed into `BatchSource`, then `detect` + `map_ei7`
//! run. Every path must return `Ok` or a typed `Err` — never panic.

use arbitrary::Unstructured;

use crate::project::SourceFormat;
use crate::source::BatchSource;

/// Table-name pool: real meta names mixed with data-ish names so
/// generated inputs sometimes form valid views.
const TABLE_POOL: [&str; 16] = [
    "metaViews",
    "metaPages",
    "metaFields",
    "metaDbInfo",
    "metaPrograms",
    "metaGridColumns",
    "metaLinks",
    "Programs",
    "Oswego",
    "Oswego12",
    "Oswego1",
    "codeYesNo",
    "viewLegacy",
    "metaExtra",
    "codeStatus",
    "Oswego13",
];

/// Column-name pool: the meta/data names the mapper looks up.
const COL_POOL: [&str; 20] = [
    "ViewId",
    "PageId",
    "FieldId",
    "GridColumnId",
    "FieldTypeId",
    "Name",
    "PromptText",
    "Text",
    "DataTableName",
    "SourceTableName",
    "CodeColumnName",
    "TextColumnName",
    "List",
    "GlobalRecordId",
    "UniqueKey",
    "RecStatus",
    "FKEY",
    "Pattern",
    "CheckCode",
    "Content",
];

/// One generated cell: kind tag → encoded value.
enum GCell {
    Missing,
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Serial(f64),
}

fn gcell(u: &mut Unstructured, kind: u8) -> GCell {
    if u.ratio(1, 4).unwrap_or(false) {
        return GCell::Missing;
    }
    match kind {
        0 => GCell::Int(u.arbitrary().unwrap_or(0)),
        1 => GCell::Float(u.arbitrary::<u32>().unwrap_or(0) as f64 / 7.0),
        2 => GCell::Text(short_text(u)),
        3 => GCell::Bool(u.arbitrary().unwrap_or(false)),
        _ => GCell::Serial(u.arbitrary::<u32>().unwrap_or(0) as f64 / 3.0 - 500.0),
    }
}

fn short_text(u: &mut Unstructured) -> String {
    const TEXTS: [&str; 10] = [
        "",
        "1",
        "0",
        "12",
        "abc",
        "2003-05-17 00:00:00",
        "Yes",
        "{00000000-0000-0000-0000-000000000001}",
        "Form Oswego\nField AGE",
        "x",
    ];
    match u.choose(&TEXTS) {
        Ok(s) => (*s).to_string(),
        Err(_) => u
            .arbitrary::<[u8; 4]>()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default(),
    }
}

/// Serialize one generated table to EPTB v1 bytes (the inverse of
/// `batch::decode_batch` for kinds 0–4; kind 5 columns have no values).
fn encode_table(
    name: &str,
    code_page: Option<&str>,
    cols: &[(String, u8, u8, Vec<GCell>)],
) -> Vec<u8> {
    let rows = cols.first().map_or(0, |c| c.3.len()) as u32;
    let mut b = Vec::new();
    b.extend_from_slice(b"EPTB");
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&rows.to_le_bytes());
    b.extend_from_slice(&(cols.len() as u16).to_le_bytes());
    let mut ext = Vec::new();
    ext.push(1u8);
    ext.extend_from_slice(&(name.len() as u16).to_le_bytes());
    ext.extend_from_slice(name.as_bytes());
    if let Some(cp) = code_page {
        ext.push(2u8);
        ext.extend_from_slice(&(cp.len() as u16).to_le_bytes());
        ext.extend_from_slice(cp.as_bytes());
    }
    b.extend_from_slice(&(ext.len() as u16).to_le_bytes());
    b.extend_from_slice(&ext);
    let bitmap_len = (rows as usize).div_ceil(8);
    for (cname, stype, kind, cells) in cols {
        b.extend_from_slice(&(cname.len() as u32).to_le_bytes());
        b.extend_from_slice(cname.as_bytes());
        b.push(*stype);
        b.push(*kind);
        if *kind == 5 {
            continue;
        }
        let mut bitmap = vec![0u8; bitmap_len];
        for (r, c) in cells.iter().enumerate() {
            if matches!(c, GCell::Missing) {
                bitmap[r / 8] |= 1 << (r % 8);
            }
        }
        b.extend_from_slice(&bitmap);
        if *kind == 2 {
            let mut blob = Vec::new();
            let mut offs = Vec::new();
            for c in cells {
                if let GCell::Text(s) = c {
                    offs.push(blob.len() as u32);
                    blob.extend_from_slice(s.as_bytes());
                    blob.push(0);
                }
            }
            b.extend_from_slice(&(blob.len() as u32).to_le_bytes());
            b.extend_from_slice(&blob);
            for o in offs {
                b.extend_from_slice(&o.to_le_bytes());
            }
            continue;
        }
        for c in cells {
            match c {
                GCell::Int(i) => b.extend_from_slice(&i.to_le_bytes()),
                GCell::Float(f) | GCell::Serial(f) => b.extend_from_slice(&f.to_le_bytes()),
                GCell::Bool(v) => b.push(u8::from(*v)),
                _ => {}
            }
        }
    }
    b
}

/// The fuzz entry point: build up to 6 tables of ≤8 rows × ≤8 columns
/// from the pools, feed them through `BatchSource` (a real EPTB
/// decode), then `detect` and `map_ei7` when detection succeeds.
pub fn legacy_meta_any(data: &[u8]) {
    let mut u = Unstructured::new(data);
    let n_tables = u.int_in_range(0usize..=6).unwrap_or(0);
    let mut src = BatchSource::new();
    for _ in 0..n_tables {
        let name = match u.choose(&TABLE_POOL) {
            Ok(s) => (*s).to_string(),
            Err(_) => short_text(&mut u),
        };
        let n_cols = u.int_in_range(0usize..=8).unwrap_or(0);
        let n_rows = u.int_in_range(0usize..=8).unwrap_or(0);
        let mut cols = Vec::new();
        for _ in 0..n_cols {
            let cname = match u.choose(&COL_POOL) {
                Ok(s) => (*s).to_string(),
                Err(_) => short_text(&mut u),
            };
            let kind = u.int_in_range(0u8..=5).unwrap_or(2);
            let cells = (0..n_rows).map(|_| gcell(&mut u, kind)).collect();
            cols.push((cname, 10u8, kind, cells));
        }
        let cp = if u.ratio(1, 4).unwrap_or(false) {
            Some("windows-1252")
        } else {
            None
        };
        let batch = encode_table(&name, cp, &cols);
        // A mismatched or duplicate-chunk push is a typed Err — fine.
        if src.push(&name, &batch).is_err() {
            return;
        }
    }
    let fmt = match crate::detect(&src) {
        Ok(SourceFormat::Ei7Access) => SourceFormat::Ei7Access,
        Ok(f) => f,
        Err(_) => return,
    };
    let _ = crate::map_ei7(&mut src, fmt);
}

/// `eptb_any` (v1.1 plan Task 13): arbitrary bytes hit `decode_batch`
/// — the boundary `legacyPush` exposes over wasm. A decoded batch must
/// re-push into `BatchSource` identically; a failure must be a typed
/// `LegacyError`, never a panic. `detect` runs over whatever survived.
#[allow(clippy::panic)]
pub fn eptb_any(data: &[u8]) {
    if let Ok(table) = crate::batch::decode_batch(data) {
        let mut src = BatchSource::new();
        assert!(src.push(&table.name, data).is_ok());
        let _ = crate::detect(&src);
    }
}
