//! epiora-mdb read tests (v1.1 Task 5): the sample Epi Info 7 database,
//! the spike's committed mdb-export CSV as the value oracle, a JET3
//! code-page file, truncation, a password-protected file and a proptest
//! that arbitrary bytes never panic the reader.

use std::io::Write;
use std::path::{Path, PathBuf};

use epiora_legacy::batch::{decode_batch, RawData};
use epiora_mdb::{Mdb, MdbError, MdbFormat};
use proptest::prelude::*;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sample_mdb() -> PathBuf {
    repo_root().join("fixtures/legacy/samples/ei7/Sample/Sample.mdb")
}

fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name)
}

fn temp_file(bytes: &[u8]) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "epiora-mdb-test-{}-{}.mdb",
        std::process::id(),
        bytes.len()
    ));
    std::fs::File::create(&path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
    path
}

#[test]
fn sample_mdb_lists_epi_info_tables() {
    let mdb = Mdb::open(&sample_mdb()).expect("Sample.mdb opens");
    assert_eq!(mdb.format(), MdbFormat::Jet4);
    let tables = mdb.tables().expect("tables");
    assert_eq!(tables.len(), 93, "Sample.mdb user-table count");
    for expected in ["Oswego", "Oswego12", "metaViews", "Epi105"] {
        assert!(tables.iter().any(|t| t == expected), "missing {expected}");
    }
    assert!(
        !tables.iter().any(|t| t.starts_with("MSys")),
        "system tables must be filtered"
    );
}

/// `MM/DD/YY HH:MM:SS` as produced by `mdb-export` for an Access
/// datetime serial (days since 1899-12-30, fraction = time of day).
fn access_serial_to_export_string(serial: f64) -> String {
    let days = serial.floor() as i64 - 25569; // 1899-12-30 = -25569 unix days
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    let secs = ((serial - serial.floor()) * 86400.0).round() as u32;
    format!(
        "{:02}/{:02}/{:02} {:02}:{:02}:{:02}",
        m,
        d,
        year % 100,
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

#[test]
fn oswego12_batch_matches_mdb_export_csv() {
    let mdb = Mdb::open(&sample_mdb()).expect("Sample.mdb opens");
    let bytes = mdb.batch("Oswego12", 0, u32::MAX).expect("batch");
    let table = decode_batch(&bytes).expect("EPTB decodes");
    assert_eq!(table.name, "Oswego12");

    let mut rdr = csv::Reader::from_path(data("Oswego12.native.csv")).expect("csv");
    let headers: Vec<String> = rdr
        .headers()
        .unwrap()
        .iter()
        .map(|h| h.to_string())
        .collect();
    assert_eq!(
        headers,
        table
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        "column names and order"
    );

    let mut n_rows = 0usize;
    for (row_idx, rec) in rdr.records().enumerate() {
        let rec = rec.unwrap();
        n_rows += 1;
        for (col_idx, cell) in rec.iter().enumerate() {
            let col = &table.columns[col_idx];
            match &col.data {
                RawData::Text(vals) => {
                    // mdb-export prints "" for NULL and "" for empty;
                    // the batch may encode either — accept both.
                    let v = vals[row_idx].as_deref();
                    assert!(
                        cell == v.unwrap_or("") && (v.is_some() || cell.is_empty()),
                        "r{row_idx} text col {}: csv {cell:?} batch {v:?}",
                        col.name
                    );
                }
                RawData::Int(vals) => {
                    let expected = cell.parse::<i64>().ok();
                    assert!(
                        expected == vals[row_idx] || (cell.is_empty() && vals[row_idx].is_none()),
                        "r{row_idx} int col {}: csv {cell} batch {:?}",
                        col.name,
                        vals[row_idx]
                    );
                }
                RawData::Float(vals) => match &vals[row_idx] {
                    Some(v) => {
                        let parsed: f64 = cell.parse().unwrap_or(f64::NAN);
                        assert!(
                            (parsed - v).abs() < 1e-9 * v.abs().max(1.0),
                            "r{row_idx} float col {}: csv {parsed} batch {v}",
                            col.name
                        );
                    }
                    None => assert_eq!(cell, "", "r{row_idx} float col {}", col.name),
                },
                RawData::Bool(vals) => {
                    let expected = cell == "1";
                    assert_eq!(
                        Some(expected),
                        vals[row_idx],
                        "r{row_idx} bool col {}",
                        col.name
                    );
                }
                RawData::DateTime(vals) => match &vals[row_idx] {
                    Some(v) => assert_eq!(
                        cell,
                        access_serial_to_export_string(*v),
                        "r{row_idx} datetime col {}",
                        col.name
                    ),
                    None => assert_eq!(cell, "", "r{row_idx} datetime col {}", col.name),
                },
                RawData::BinarySkipped => {
                    panic!(
                        "Oswego12 has no skipped column (r{row_idx} col {})",
                        col.name
                    )
                }
            }
        }
    }
    assert_eq!(n_rows, 75, "Oswego12 row count");
}

#[test]
fn jet3_cp1252_text_decodes() {
    // testIndexCodesV1997.mdb (JET3, cp1252): Table1 `data` cells hold
    // "AA<byte>AA" for every code-page byte; the `name` col is rowNNN.
    let mdb = Mdb::open(&data("testIndexCodesV1997.mdb")).expect("opens");
    assert_eq!(mdb.format(), MdbFormat::Jet3);
    let bytes = mdb.batch("Table1", 0, u32::MAX).expect("batch");
    let table = decode_batch(&bytes).expect("EPTB decodes");
    assert_eq!(table.name, "Table1");
    assert_eq!(table.code_page.as_deref(), Some("windows-1252"));

    let name_col = table.columns.iter().find(|c| c.name == "name").unwrap();
    let data_col = table.columns.iter().find(|c| c.name == "data").unwrap();
    let (RawData::Text(names), RawData::Text(datas)) = (&name_col.data, &data_col.data) else {
        panic!("name/data must be text columns");
    };
    let mut by_name = std::collections::HashMap::new();
    for (n, d) in names.iter().zip(datas.iter()) {
        if let (Some(n), Some(d)) = (n, d) {
            by_name.insert(n.clone(), d.clone());
        }
    }
    // The file stores Unicode chars cp1252-encoded on write: defined
    // cp1252 bytes roundtrip (row233 holds byte 0xE9 -> é), the five
    // cp1252-undefined bytes are stored raw (0x81/8D/8F/90/9D), and
    // C1 control chars became '?' on write (row145). The discriminator
    // vs the mdbtools latin1 fallback: undefined bytes must come out
    // U+FFFD (latin1 would emit U+0081 etc.) and count as decode errors.
    assert_eq!(by_name["row233"], "AA\u{e9}AA");
    assert_eq!(by_name["row160"], "AA\u{a0}AA");
    assert_eq!(by_name["row145"], "AA?AA");
    assert_eq!(by_name["row129"], "AA\u{fffd}AA");
    assert_eq!(by_name["row141"], "AA\u{fffd}AA");
    assert_eq!(by_name["row143"], "AA\u{fffd}AA");
    assert_eq!(by_name["row144"], "AA\u{fffd}AA");
    assert_eq!(by_name["row157"], "AA\u{fffd}AA");
    assert!(table.decode_errors >= 5, "undefined bytes counted");
}

/// Rows in a decoded batch: every non-skipped column holds
/// `row_count` entries.
fn decoded_rows(t: &epiora_legacy::batch::RawTable) -> usize {
    fn vals_len(d: &RawData) -> usize {
        match d {
            RawData::Int(v) => v.len(),
            RawData::Float(v) => v.len(),
            RawData::Text(v) => v.len(),
            RawData::Bool(v) => v.len(),
            RawData::DateTime(v) => v.len(),
            RawData::BinarySkipped => 0,
        }
    }
    t.columns
        .iter()
        .map(|c| vals_len(&c.data))
        .max()
        .unwrap_or(0)
}

/// A chunk's values must equal the full table's rows
/// `[off, off + chunk_rows)` column by column.
fn assert_rows_match(full: &RawData, chunk: &RawData, off: usize) {
    match (full, chunk) {
        (RawData::Int(f), RawData::Int(c)) => assert_eq!(&f[off..off + c.len()], &c[..]),
        (RawData::Float(f), RawData::Float(c)) => assert_eq!(&f[off..off + c.len()], &c[..]),
        (RawData::Text(f), RawData::Text(c)) => assert_eq!(&f[off..off + c.len()], &c[..]),
        (RawData::Bool(f), RawData::Bool(c)) => assert_eq!(&f[off..off + c.len()], &c[..]),
        (RawData::DateTime(f), RawData::DateTime(c)) => {
            assert_eq!(&f[off..off + c.len()], &c[..])
        }
        (RawData::BinarySkipped, RawData::BinarySkipped) => {}
        _ => panic!("column kind changed between batches"),
    }
}

#[test]
fn chunked_batches_concatenate_to_full_table() {
    // Regression: the encoder used to count the row that tripped the
    // limit in the header, so batch(0, 2) claimed 3 rows, encoded 2
    // and failed to decode as truncated.
    let mdb = Mdb::open(&sample_mdb()).expect("Sample.mdb opens");

    let two = decode_batch(&mdb.batch("Oswego12", 0, 2).expect("batch")).expect("decodes");
    assert_eq!(decoded_rows(&two), 2, "limit honoured exactly");

    let full = decode_batch(&mdb.batch("Oswego12", 0, u32::MAX).expect("batch")).expect("decodes");
    let full_rows = decoded_rows(&full);
    assert_eq!(full_rows, 75, "Oswego12 row count");

    let mut offset = 0usize;
    let mut total = 0usize;
    loop {
        let chunk = decode_batch(&mdb.batch("Oswego12", offset as u32, 7).expect("batch"))
            .expect("every batch decodes");
        let n = decoded_rows(&chunk);
        if n == 0 {
            break;
        }
        assert!(n <= 7, "batch at offset {offset} exceeded the limit");
        for (fc, cc) in full.columns.iter().zip(chunk.columns.iter()) {
            assert_eq!(fc.name, cc.name, "column order stable");
            assert_rows_match(&fc.data, &cc.data, offset);
        }
        total += n;
        offset += 7;
    }
    assert_eq!(total, full_rows, "chunks add up to the full table");

    let past_end = decode_batch(
        &mdb.batch("Oswego12", (full_rows + 10) as u32, 7)
            .expect("batch"),
    )
    .expect("decodes");
    assert_eq!(decoded_rows(&past_end), 0, "offset past the end is empty");
}

#[test]
fn truncated_file_is_mdb_truncated() {
    let bytes = std::fs::read(sample_mdb()).unwrap();
    let path = temp_file(&bytes[..4096]);
    let err = Mdb::open(&path).expect_err("truncated file must error");
    assert_eq!(err.code(), "mdb_truncated", "{err:?}");
    std::fs::remove_file(path).ok();
}

#[test]
fn password_protected_is_typed() {
    // jackcessencrypt pwd-set.mdb: JET4 file with database password
    // "testpassword" (see tests/data/NOTICE).
    let err = Mdb::open(&data("pwd-set.mdb")).expect_err("password file must error");
    assert_eq!(err.code(), "mdb_password_protected", "{err:?}");
    assert!(matches!(err, MdbError::PasswordProtected { .. }));
}

#[test]
fn not_a_database_is_mdb_corrupt() {
    let path = temp_file(b"this is not an access database at all, just text");
    let err = Mdb::open(&path).expect_err("must error");
    assert_eq!(err.code(), "mdb_corrupt", "{err:?}");
    std::fs::remove_file(path).ok();
}

proptest! {
    #[test]
    fn random_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..=8192)) {
        let path = temp_file(&bytes);
        // No input may panic: open either errors or yields a usable
        // handle whose every call also returns without panicking.
        if let Ok(mdb) = Mdb::open(&path) {
            let _ = mdb.format();
            if let Ok(tables) = mdb.tables() {
                for t in &tables {
                    let _ = mdb.batch(t, 0, 8);
                }
            }
        }
        std::fs::remove_file(path).ok();
    }
}
