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

fn col_block<T, F: Fn(&mut Vec<u8>, &T)>(b: &mut Vec<u8>, kind: u8, vals: &[Option<T>], write: F) {
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
