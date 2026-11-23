//! Task 7 golden parity: every `fixtures/legacy/golden/ei7/**` project
//! maps structurally to its `project.json` + dataset files — views,
//! grids, links, programs, check code and the ordered report — through
//! both the Access (`epiora-mdb`) and SQLite (`rusqlite`) backends,
//! plus the verbatim-copy cross-backend check and the datetime
//! semantics pins.

mod common;

use common::*;
use epiora_core::dataset::ColumnData;
use epiora_legacy::batch::RawData;
use epiora_legacy::{detect, map_ei7, BatchSource, LegacySource, SourceFormat};

/// `fixtures/legacy/golden/ei7/**`: map the source file and compare
/// the whole project structure against `project.json`.
#[test]
fn every_ei7_sample_matches_golden() {
    let projects = ei7_projects();
    assert!(projects.len() >= 20, "expected the ei7 golden set");
    for (dir, src) in &projects {
        let ext = src.extension().unwrap().to_string_lossy().to_lowercase();
        let format = source_format(golden_project(dir)["source_format"].as_str().unwrap());
        let mut source: Box<dyn epiora_legacy::LegacySource> = match ext.as_str() {
            "mdb" => Box::new(mdb_source(src)),
            "db" => Box::new(SqliteSource::open(src)),
            other => panic!("unexpected source extension {other} for {src:?}"),
        };
        // Layout detection must see this as an EI7 project.
        assert_eq!(
            detect(source.as_ref()).unwrap(),
            SourceFormat::Ei7Access,
            "{dir:?}: detect"
        );
        let project = map_ei7(source.as_mut(), format)
            .unwrap_or_else(|e| panic!("{dir:?}: map_ei7 failed: {e} ({})", e.code()));
        assert_project_eq(&project, dir);
    }
}

/// `hdcaseform_check_code_is_not_truncated` — HDCaseForm's CheckCode
/// MEMO is 271,820 bytes, over upstream mdb-export's 200000-byte bind
/// size; the golden (regenerated at the product 4 MiB bind size in
/// #115, owner decision 1A) pins 164 blocks. A truncated read reports
/// 137, so this count fails if the bind size regresses.
#[test]
fn hdcaseform_check_code_is_not_truncated() {
    let mdb = root().join("fixtures/legacy/samples/ei7/HansensDisease/HansensDisease.mdb");
    let mut src = mdb_source(&mdb);
    let project = map_ei7(&mut src, SourceFormat::Ei7Access).unwrap();
    let check = project
        .check_code
        .iter()
        .find(|c| c.view == "HDCaseForm")
        .expect("HDCaseForm check code");
    assert_eq!(check.blocks, 164, "HDCaseForm check_code blocks");
}

/// `sqlite_copy_matches_access` — a verbatim SQLite copy of the Access
/// Sample maps to the same project (spec §8.2: the copy-tables check).
#[test]
fn sqlite_copy_matches_access() {
    let mdb = root().join("fixtures/legacy/samples/ei7/Sample/Sample.mdb");
    let mut acc = mdb_source(&mdb);
    let tmp = std::env::temp_dir().join("epiora-legacy-copy-sample.db");
    let _ = std::fs::remove_file(&tmp);
    copy_to_sqlite(&acc, &tmp);
    let mut lite = SqliteSource::open(&tmp);

    let a = map_ei7(&mut acc, SourceFormat::Ei7Access).unwrap();
    let b = map_ei7(&mut lite, SourceFormat::Ei7Sqlite).unwrap();
    let _ = std::fs::remove_file(&tmp);

    assert_projects_eq(&a, &b);
}

/// `multi_page_view_joins_on_globalrecordid` — a view with several
/// pages joins each page table's rows onto the spine by
/// `GlobalRecordId` (SampleDB `Surveillance` has three pages).
#[test]
fn multi_page_view_joins_on_globalrecordid() {
    let mut src =
        SqliteSource::open(&root().join("fixtures/legacy/samples/ei7/Sample/SampleDB.db"));
    let project = map_ei7(&mut src, SourceFormat::Ei7Sqlite).unwrap();
    let view = project
        .views
        .iter()
        .find(|v| v.name == "Surveillance")
        .expect("Surveillance view");
    let golden_dir = golden_root().join("ei7/Sample/SampleDB");
    let golden = golden_dataset(&golden_dir, "Surveillance.dataset.json");
    assert_dataset_eq(&view.dataset, &golden, "Surveillance");
    let tables: Vec<String> = src
        .tables()
        .into_iter()
        .filter(|t| t.starts_with("Surveillance2"))
        .collect();
    assert!(
        tables.len() >= 3,
        "expected >=3 page tables, got {tables:?}"
    );
}

/// `access_datetime_epoch_and_time_split` — Access serials become
/// `date` days, `time` seconds-of-day and `date_time` epoch seconds;
/// serials outside `[0, 1e6]` take `mdb_date_to_tm`'s early return —
/// the zeroed `tm` strftime renders as `"1900-01-00 00:00:00"`, the
/// literal text the golden stores (Review Focus 3).
#[test]
fn access_datetime_epoch_and_time_split() {
    let mut src = BatchSource::new();
    push_meta(
        &mut src,
        "Dt",
        &[
            (1, "GlobalRecordId", 26, 0),
            (2, "UniqueKey", 23, 0),
            (3, "RecStatus", 22, 0),
            (4, "D", 7, 1),
            (5, "T", 8, 1),
            (6, "DT", 9, 1),
        ],
    );
    // Spine rows in g1,g2,g3 order; the page table stores g2,g1,g3 —
    // the join is by GlobalRecordId, not position.
    src.push(
        "Dt",
        &encode_batch(
            "Dt",
            &[
                ("GlobalRecordId".into(), 10, text_col(&["g1", "g2", "g3"])),
                ("UniqueKey".into(), 10, int_col(&[1, 2, 3])),
                ("RecStatus".into(), 10, int_col(&[1, 1, 0])),
            ],
        ),
    )
    .unwrap();
    src.push(
        "Dt1",
        &encode_batch(
            "Dt1",
            &[
                ("GlobalRecordId".into(), 10, text_col(&["g2", "g1", "g3"])),
                (
                    "D".into(),
                    8,
                    RawData::DateTime(vec![Some(0.0), Some(45658.0), Some(-1.5)]),
                ),
                (
                    "T".into(),
                    8,
                    RawData::DateTime(vec![Some(0.25), Some(0.75), Some(1_000_001.0)]),
                ),
                (
                    "DT".into(),
                    8,
                    RawData::DateTime(vec![Some(1.0), Some(45658.5), Some(45658.99999)]),
                ),
            ],
        ),
    )
    .unwrap();

    let p = map_ei7(&mut src, SourceFormat::Ei7Access).unwrap();
    let v = &p.views[0];
    assert_eq!(v.name, "Dt");
    assert_eq!(v.deleted_records, 1); // g3 has RecStatus 0
    let col = |n: &str| &v.dataset.column(n).unwrap().data;
    // Spine order g1,g2,g3; values join through the page table's
    // GlobalRecordId index (g2,g1,g3 there), not row position.
    // g1's D=45658 → 2025-01-01 → 20089; g2's D=0.0 → 1899-12-30 →
    // -25569; g3's -1.5 is out of mdbtools range → "1900-01-00" →
    // days_from_civil(1900,1,0) = -25568.
    assert_eq!(
        col("D"),
        &ColumnData::Date(vec![Some(20089), Some(-25569), Some(-25568)])
    );
    // g1 T=0.75 → 18:00; g2 T=0.25 → 06:00; g3 out of range →
    // "00:00:00" → 0.
    assert_eq!(
        col("T"),
        &ColumnData::Time(vec![Some(64800), Some(21600), Some(0)])
    );
    // g1 DT=45658.5 → 2025-01-01 12:00; g2 DT=1.0 → 1899-12-31 →
    // -25568 days; g3 45658.99999 → 23:59:59 (round-to-second,
    // no carry).
    assert_eq!(
        col("DT"),
        &ColumnData::DateTime(vec![Some(1735732800), Some(-2209075200), Some(1735775999)])
    );
    assert_eq!(v.key_column, "GlobalRecordId");
}

/// `deleted_records_kept_hidden_and_counted` — rows with `RecStatus`
/// 0 stay in the view dataset (the `RecStatus` system column carries
/// them) and only `deleted_records` counts them.
#[test]
fn deleted_records_kept_hidden_and_counted() {
    let mut src = BatchSource::new();
    push_meta(
        &mut src,
        "D",
        &[
            (1, "GlobalRecordId", 26, 0),
            (2, "UniqueKey", 23, 0),
            (3, "RecStatus", 22, 0),
            (4, "X", 5, 1),
        ],
    );
    src.push(
        "D",
        &encode_batch(
            "D",
            &[
                ("GlobalRecordId".into(), 10, text_col(&["a", "b", "c"])),
                ("UniqueKey".into(), 10, int_col(&[1, 2, 3])),
                ("RecStatus".into(), 10, int_col(&[1, 0, 1])),
            ],
        ),
    )
    .unwrap();
    src.push(
        "D1",
        &encode_batch(
            "D1",
            &[
                ("GlobalRecordId".into(), 10, text_col(&["a", "b", "c"])),
                ("X".into(), 5, int_col(&[10, 20, 30])),
            ],
        ),
    )
    .unwrap();

    let p = map_ei7(&mut src, SourceFormat::Ei7Access).unwrap();
    let v = &p.views[0];
    // The deleted row is kept: 3 dataset rows, its values intact.
    assert_eq!(v.dataset.row_count(), 3);
    assert_eq!(v.deleted_records, 1);
    assert_eq!(
        v.dataset.column("RecStatus").unwrap().data,
        ColumnData::Integer(vec![Some(1), Some(0), Some(1)])
    );
    assert_eq!(
        v.dataset.column("X").unwrap().data,
        ColumnData::Integer(vec![Some(10), Some(20), Some(30)])
    );
}

/// `every_unimported_item_has_report_entry` — form-metadata fields,
/// binary fields, unknown field types, unclaimed physical columns and
/// unclaimed tables each produce exactly one report entry.
#[test]
fn every_unimported_item_has_report_entry() {
    let mut src = BatchSource::new();
    push_meta(
        &mut src,
        "V",
        &[
            (1, "GlobalRecordId", 26, 0),
            (2, "UniqueKey", 23, 0),
            (3, "RecStatus", 22, 0),
            (4, "Lab", 2, 1),  // LabelTitle — form metadata
            (5, "Pic", 14, 1), // Image — binary OLE
            (6, "Wut", 77, 1), // unknown type — imported as text
            (7, "X", 5, 1),    // Number
        ],
    );
    src.push(
        "V",
        &encode_batch(
            "V",
            &[
                ("GlobalRecordId".into(), 10, text_col(&["a"])),
                ("UniqueKey".into(), 10, int_col(&[1])),
                ("RecStatus".into(), 10, int_col(&[1])),
            ],
        ),
    )
    .unwrap();
    src.push(
        "V1",
        &encode_batch(
            "V1",
            &[
                ("GlobalRecordId".into(), 10, text_col(&["a"])),
                ("X".into(), 5, int_col(&[7])),
                ("Stray".into(), 10, text_col(&["z"])),
            ],
        ),
    )
    .unwrap();
    // Unclaimed tables of each flavour.
    for t in ["Junk", "metaWhatever", "codeFoo", "viewOld"] {
        src.push(t, &encode_batch(t, &[("A".into(), 10, int_col(&[1]))]))
            .unwrap();
    }

    let p = map_ei7(&mut src, SourceFormat::Ei7Access).unwrap();
    let has = |kind: &str, item: &str| {
        p.report
            .iter()
            .any(|e| e.kind.as_str() == kind && e.item == item)
    };
    assert!(has("field_not_imported", "V/Lab"), "{:?}", p.report);
    assert!(has("field_not_imported", "V/Pic"), "{:?}", p.report);
    assert!(has("field_not_imported", "V/Wut"), "{:?}", p.report);
    assert!(has("column_not_imported", "V1/Stray"), "{:?}", p.report);
    assert!(has("table_not_imported", "Junk"), "{:?}", p.report);
    assert!(has("table_not_imported", "metaWhatever"), "{:?}", p.report);
    assert!(has("table_not_imported", "codeFoo"), "{:?}", p.report);
    assert!(has("table_not_imported", "viewOld"), "{:?}", p.report);
}

/// `undecodable_text_is_reported` — a batch that declares decode
/// errors surfaces a `text_decoding` report entry (Review Focus 1).
#[test]
fn undecodable_text_is_reported() {
    let mut src = ei7_meta_source(&[(1, "GlobalRecordId", 26, 0)]);
    // A data batch whose tag-3 extension declares substitutions.
    let mut batch = encode_batch(
        "V",
        &[
            ("GlobalRecordId".into(), 10, text_col(&["g1"])),
            ("FKEY".into(), 10, text_col(&[""])),
        ],
    );
    // ext_len is at offset 12; append a tag-3 ext record.
    let ext_len = u16::from_le_bytes([batch[12], batch[13]]) as usize;
    let mut ext3 = Vec::new();
    ext3.push(3u8);
    ext3.extend_from_slice(&4u16.to_le_bytes());
    ext3.extend_from_slice(&2u32.to_le_bytes());
    let new_ext_len = (ext_len + ext3.len()) as u16;
    batch[12..14].copy_from_slice(&new_ext_len.to_le_bytes());
    let mut rebuilt = Vec::new();
    rebuilt.extend_from_slice(&batch[..14 + ext_len]);
    rebuilt.extend_from_slice(&ext3);
    rebuilt.extend_from_slice(&batch[14 + ext_len..]);
    src.push("V", &rebuilt).unwrap();

    let p = map_ei7(&mut src, SourceFormat::Ei7Access).unwrap();
    assert!(
        p.report
            .iter()
            .any(|e| e.kind.as_str() == "text_decoding" && e.item == "V"),
        "expected a text_decoding entry, got {:?}",
        p.report
            .iter()
            .map(|e| (e.kind.as_str(), &e.item))
            .collect::<Vec<_>>()
    );
}
