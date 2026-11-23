//! Task 7 typed errors: every failure path lands on a `LegacyError`
//! with a stable `code()` — truncated batches, unknown layouts,
//! non-EI7 files handed to `map_ei7`, and `epiora-import` diff
//! errors bubbling through `refresh_plan` unchanged.

mod common;

use common::*;
use epiora_legacy::{detect, map_ei7, refresh_plan, BatchSource, LegacyError, SourceFormat};

/// `truncated_batch_is_batch_invalid` — a cut-off EPTB chunk is a
/// typed `legacy_batch_invalid`, never a panic.
#[test]
fn truncated_batch_is_batch_invalid() {
    let good = encode_batch("T", &[("A".into(), 10, int_col(&[1, 2, 3]))]);
    for cut in [4, 10, 20, good.len() - 1] {
        let mut src = BatchSource::new();
        let err = src.push("T", &good[..cut]).unwrap_err();
        assert_eq!(err.code(), "legacy_batch_invalid", "cut {cut}");
    }
    // Trailing bytes are also invalid.
    let mut extra = good.clone();
    extra.push(0);
    let mut src = BatchSource::new();
    assert_eq!(
        src.push("T", &extra).unwrap_err().code(),
        "legacy_batch_invalid"
    );
}

/// `map_ei7_not_ei7` — a source without `metaViews` is a typed
/// `legacy_layout_unknown`, not an empty project.
#[test]
fn map_ei7_not_ei7() {
    let mut src = BatchSource::new();
    src.push(
        "Ordinary",
        &encode_batch("Ordinary", &[("A".into(), 10, int_col(&[1]))]),
    )
    .unwrap();
    let err = map_ei7(&mut src, SourceFormat::Ei7Access).unwrap_err();
    assert_eq!(err.code(), "legacy_layout_unknown");
    match err {
        LegacyError::LayoutUnknown { tables } => assert_eq!(tables, vec!["Ordinary"]),
        other => panic!("expected LayoutUnknown, got {other:?}"),
    }
}

/// `plain_access_db_is_layout_unknown_with_tables` — a file with no
/// meta*/view* tables detects as `legacy_layout_unknown` and the error
/// carries the plain data-table list (Review Focus 2).
#[test]
fn plain_access_db_is_layout_unknown_with_tables() {
    let mut src = BatchSource::new();
    src.push(
        "Table1",
        &encode_batch("Table1", &[("A".into(), 10, int_col(&[1]))]),
    )
    .unwrap();
    src.push(
        "Foo",
        &encode_batch("Foo", &[("B".into(), 10, text_col(&["x"]))]),
    )
    .unwrap();
    src.push(
        "metaOdd",
        &encode_batch("metaOdd", &[("C".into(), 10, int_col(&[1]))]),
    )
    .unwrap();
    let err = detect(&src).unwrap_err();
    assert_eq!(err.code(), "legacy_layout_unknown");
    match err {
        LegacyError::LayoutUnknown { tables } => {
            // meta*/sqlite_* tables are excluded from the offered list.
            assert_eq!(tables, vec!["Table1", "Foo"]);
        }
        other => panic!("expected LayoutUnknown, got {other:?}"),
    }
}

/// `one_broken_view_does_not_block_others` — a view with no fields
/// (an empty dataset the model cannot represent) becomes a
/// `view_failed` report entry; the other view still maps (plan §7).
#[test]
fn one_broken_view_does_not_block_others() {
    let mut src = BatchSource::new();
    src.push(
        "metaViews",
        &encode_batch(
            "metaViews",
            &[
                ("ViewId".into(), 10, int_col(&[1, 2])),
                ("Name".into(), 10, text_col(&["Bad", "Good"])),
            ],
        ),
    )
    .unwrap();
    src.push(
        "metaFields",
        &encode_batch(
            "metaFields",
            &[
                ("FieldId".into(), 10, int_col(&[1, 2])),
                ("ViewId".into(), 10, int_col(&[2, 2])),
                ("PageId".into(), 10, int_col(&[0, 0])),
                ("FieldTypeId".into(), 10, int_col(&[26, 23])),
                (
                    "Name".into(),
                    10,
                    text_col(&["GlobalRecordId", "UniqueKey"]),
                ),
            ],
        ),
    )
    .unwrap();
    src.push(
        "Good",
        &encode_batch(
            "Good",
            &[
                ("GlobalRecordId".into(), 10, text_col(&["g1"])),
                ("UniqueKey".into(), 10, int_col(&[1])),
            ],
        ),
    )
    .unwrap();
    let p = map_ei7(&mut src, SourceFormat::Ei7Access).unwrap();
    assert_eq!(p.views.len(), 1);
    assert_eq!(p.views[0].name, "Good");
    assert!(
        p.report
            .iter()
            .any(|e| e.kind.as_str() == "view_failed" && e.item == "Bad"),
        "expected a view_failed entry for Bad, got {:?}",
        p.report
            .iter()
            .map(|e| (e.kind.as_str(), &e.item))
            .collect::<Vec<_>>()
    );
}

/// Diff key problems bubble through `refresh_plan` as the import
/// crate's own typed errors (`duplicate_key`, `missing_key`,
/// `unknown_column`, `invalid_key_type`).
#[test]
fn refresh_key_errors_bubble() {
    let mut src = ei7_meta_source(&[(1, "GlobalRecordId", 26, 0), (2, "UniqueKey", 23, 0)]);
    // Two rows with the same GlobalRecordId → DuplicateKey on diff.
    src.push(
        "V",
        &encode_batch(
            "V",
            &[
                ("GlobalRecordId".into(), 10, text_col(&["g1", "g1"])),
                ("UniqueKey".into(), 10, int_col(&[1, 2])),
            ],
        ),
    )
    .unwrap();
    let project = map_ei7(&mut src, SourceFormat::Ei7Access).unwrap();
    let prev = view_datasets(&project);
    let err = refresh_plan(&prev, &project).unwrap_err();
    assert_eq!(err.code(), "duplicate_key", "got {err:?}");
}

/// The report kind vocabulary is the golden's: every emitted kind
/// resolves through `as_str`.
#[test]
fn report_kind_vocabulary_is_stable() {
    use epiora_legacy::ReportKind::*;
    let pairs = [
        (FieldNotImported, "field_not_imported"),
        (FieldChanged, "field_imported_changed"),
        (TableNotImported, "table_not_imported"),
        (FieldColumnMissing, "field_column_missing"),
        (ViewDataMissing, "view_data_missing"),
        (ColumnNotImported, "column_not_imported"),
        (CodeTableMissing, "code_table_missing"),
        (RecordLinksNotImported, "record_links_not_imported"),
        (LayoutUnknown, "legacy_layout_unknown"),
        (CodePageAssumed, "code_page_assumed"),
        (TextDecoding, "text_decoding"),
        (ViewFailed, "view_failed"),
    ];
    for (kind, want) in pairs {
        assert_eq!(kind.as_str(), want);
    }
}
