//! A minimal EPTB v1 encoder (the inverse of `batch::decode_batch`)
//! for synthetic `BatchSource` inputs, plus the meta-table builders
//! the synthetic EI7 tests share.

use epiora_legacy::batch::RawData;
use epiora_legacy::BatchSource;

pub(crate) fn raw_len(d: &RawData) -> usize {
    match d {
        RawData::Int(v) => v.len(),
        RawData::Float(v) => v.len(),
        RawData::Bool(v) => v.len(),
        RawData::Text(v) => v.len(),
        RawData::DateTime(v) => v.len(),
        RawData::BinarySkipped => 0,
    }
}

/// Integer column (all present).
pub fn int_col(v: &[i64]) -> RawData {
    RawData::Int(v.iter().copied().map(Some).collect())
}

/// Text column (all present).
pub fn text_col(v: &[&str]) -> RawData {
    RawData::Text(v.iter().map(|s| Some((*s).to_string())).collect())
}

/// Encode one table's columns as an EPTB v1 batch.
pub fn encode_batch(name: &str, columns: &[(String, u8, RawData)]) -> Vec<u8> {
    let rows = columns
        .iter()
        .find_map(|(_, _, d)| match d {
            RawData::BinarySkipped => None,
            other => Some(raw_len(other)),
        })
        .unwrap_or(0) as u32;
    let mut b = Vec::new();
    b.extend_from_slice(b"EPTB");
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&rows.to_le_bytes());
    b.extend_from_slice(&(columns.len() as u16).to_le_bytes());
    let mut ext = Vec::new();
    ext.push(1u8);
    ext.extend_from_slice(&(name.len() as u16).to_le_bytes());
    ext.extend_from_slice(name.as_bytes());
    b.extend_from_slice(&(ext.len() as u16).to_le_bytes());
    b.extend_from_slice(&ext);
    let bitmap_len = rows.div_ceil(8) as usize;
    for (cname, stype, data) in columns {
        b.extend_from_slice(&(cname.len() as u32).to_le_bytes());
        b.extend_from_slice(cname.as_bytes());
        b.push(*stype);
        let kind = match data {
            RawData::Int(_) => 0u8,
            RawData::Float(_) => 1,
            RawData::Text(_) => 2,
            RawData::Bool(_) => 3,
            RawData::DateTime(_) => 4,
            RawData::BinarySkipped => 5,
        };
        b.push(kind);
        if kind == 5 {
            continue;
        }
        let missing: Vec<bool> = match data {
            RawData::Int(v) => v.iter().map(|x| x.is_none()).collect(),
            RawData::Float(v) => v.iter().map(|x| x.is_none()).collect(),
            RawData::Text(v) => v.iter().map(|x| x.is_none()).collect(),
            RawData::Bool(v) => v.iter().map(|x| x.is_none()).collect(),
            RawData::DateTime(v) => v.iter().map(|x| x.is_none()).collect(),
            RawData::BinarySkipped => Vec::new(),
        };
        let mut bitmap = vec![0u8; bitmap_len];
        for (r, m) in missing.iter().enumerate() {
            if *m {
                bitmap[r / 8] |= 1 << (r % 8);
            }
        }
        b.extend_from_slice(&bitmap);
        match data {
            RawData::Int(v) => {
                for x in v.iter().flatten() {
                    b.extend_from_slice(&x.to_le_bytes());
                }
            }
            RawData::Float(v) | RawData::DateTime(v) => {
                for x in v.iter().flatten() {
                    b.extend_from_slice(&x.to_le_bytes());
                }
            }
            RawData::Bool(v) => {
                for x in v.iter().flatten() {
                    b.push(u8::from(*x));
                }
            }
            RawData::Text(v) => {
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
            RawData::BinarySkipped => {}
        }
    }
    b
}

/// A metaFields batch: each `fields` tuple is
/// `(FieldId, Name, FieldTypeId, PageId)` — `ViewId`/`DataTableName`
/// etc. default as documented.
pub fn metafields_batch(fields: &[(i64, &str, i64, i64)]) -> Vec<u8> {
    encode_batch(
        "metaFields",
        &[
            (
                "FieldId".into(),
                10,
                int_col(&fields.iter().map(|f| f.0).collect::<Vec<_>>()),
            ),
            ("ViewId".into(), 10, int_col(&vec![1; fields.len()])),
            (
                "PageId".into(),
                10,
                int_col(&fields.iter().map(|f| f.3).collect::<Vec<_>>()),
            ),
            (
                "FieldTypeId".into(),
                10,
                int_col(&fields.iter().map(|f| f.2).collect::<Vec<_>>()),
            ),
            (
                "Name".into(),
                10,
                text_col(&fields.iter().map(|f| f.1).collect::<Vec<_>>()),
            ),
        ],
    )
}

/// `metaViews` + `metaFields` for one view named `view` (the errors
/// tests' minimal meta set; no `metaDbInfo`/`metaPages`).
pub fn ei7_meta_source(fields: &[(i64, &str, i64, i64)]) -> BatchSource {
    let mut src = BatchSource::new();
    src.push(
        "metaViews",
        &encode_batch(
            "metaViews",
            &[
                ("ViewId".into(), 10, int_col(&[1])),
                ("Name".into(), 10, text_col(&["V"])),
            ],
        ),
    )
    .unwrap();
    if !fields.is_empty() {
        src.push("metaFields", &metafields_batch(fields)).unwrap();
    }
    src
}

/// The full synthetic meta set the golden.rs tests need:
/// `metaDbInfo` + `metaViews` + `metaPages` + `metaFields` for one
/// single-page view named `view`.
pub fn push_meta(src: &mut BatchSource, view: &str, fields: &[(i64, &str, i64, i64)]) {
    src.push(
        "metaDbInfo",
        &encode_batch(
            "metaDbInfo",
            &[("ProjectName".into(), 10, text_col(&[view]))],
        ),
    )
    .unwrap();
    src.push(
        "metaViews",
        &encode_batch(
            "metaViews",
            &[
                ("ViewId".into(), 10, int_col(&[1])),
                ("Name".into(), 10, text_col(&[view])),
            ],
        ),
    )
    .unwrap();
    src.push(
        "metaPages",
        &encode_batch(
            "metaPages",
            &[
                ("PageId".into(), 10, int_col(&[1])),
                ("ViewId".into(), 10, int_col(&[1])),
                ("Position".into(), 10, int_col(&[1])),
            ],
        ),
    )
    .unwrap();
    src.push("metaFields", &metafields_batch(fields)).unwrap();
}
