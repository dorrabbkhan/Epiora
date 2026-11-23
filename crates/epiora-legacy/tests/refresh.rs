//! Task 7 re-import: `refresh_plan` diffs a remapped project against
//! the committed view datasets by `GlobalRecordId`, matching the
//! `fixtures/legacy/golden/reimport/expected-diff.json` vocabulary;
//! an unchanged re-import plans nothing.

mod common;

use std::collections::BTreeMap;

use common::*;
use epiora_import::{record_ids, RecordKey};
use epiora_legacy::{map_ei7, refresh_plan, RefreshOutcome, SourceFormat};

/// `modified_copy_matches_expected_diff` — the modified SampleDB copy
/// produces exactly the expected added/removed/changed records and
/// the added `ReimportFlag` column (spec §7.3).
#[test]
fn modified_copy_matches_expected_diff() {
    let prev_dir = golden_root().join("ei7/Sample/SampleDB");
    let reimport_db = golden_root().join("reimport/SampleDB.db");

    let mut prev_src =
        SqliteSource::open(&root().join("fixtures/legacy/samples/ei7/Sample/SampleDB.db"));
    let prev = map_ei7(&mut prev_src, SourceFormat::Ei7Sqlite).unwrap();
    assert_project_eq(&prev, &prev_dir);

    let mut next_src = SqliteSource::open(&reimport_db);
    let next = map_ei7(&mut next_src, SourceFormat::Ei7Sqlite).unwrap();
    assert_project_eq(&next, &golden_root().join("reimport"));

    let plan = refresh_plan(&view_datasets(&prev), &next).unwrap();
    let vr = plan
        .views
        .iter()
        .find(|v| v.name == "Oswego")
        .expect("Oswego refresh");
    let RefreshOutcome::Changed(diff) = &vr.outcome else {
        panic!("expected Oswego Changed, got {:?}", vr.outcome)
    };

    let exp = serde_json::from_str::<serde_json::Value>(
        &std::fs::read_to_string(golden_root().join("reimport/expected-diff.json")).unwrap(),
    )
    .unwrap();

    // Record ids in `next` order — `added` rows resolve to ids.
    let next_view = next.views.iter().find(|v| v.name == "Oswego").unwrap();
    let ids = record_ids(
        next_view.dataset.columns(),
        &RecordKey::Column {
            name: "GlobalRecordId".to_string(),
        },
    )
    .unwrap();
    let added: Vec<String> = diff
        .added
        .iter()
        .map(|&i| ids[i as usize].clone())
        .collect();
    assert_eq!(
        added,
        exp["added_records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        diff.removed,
        exp["removed_records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    );

    // Field changes grouped per record like expected-diff.json.
    let mut changed: BTreeMap<&str, BTreeMap<String, serde_json::Value>> = BTreeMap::new();
    for fc in &diff.changed {
        let fields = changed.entry(fc.record_id.as_str()).or_default();
        let to_json = |v: &Option<epiora_core::dataset::Value>| match v {
            None => serde_json::Value::Null,
            Some(epiora_core::dataset::Value::Integer(i)) => (*i).into(),
            Some(epiora_core::dataset::Value::Real(f)) => (*f).into(),
            Some(epiora_core::dataset::Value::Text(s)) => s.clone().into(),
            Some(epiora_core::dataset::Value::Boolean(b)) => (*b).into(),
            Some(other) => serde_json::to_value(other).unwrap(),
        };
        fields.insert(
            fc.field.clone(),
            serde_json::json!({"old": to_json(&fc.old), "new": to_json(&fc.new)}),
        );
    }
    let mut want: BTreeMap<String, BTreeMap<String, serde_json::Value>> = BTreeMap::new();
    for cr in exp["changed_records"].as_array().unwrap() {
        let rid = cr["record_id"].as_str().unwrap();
        let fields = cr["fields"].as_object().unwrap();
        want.insert(
            rid.to_string(),
            fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        );
    }
    let got_map: BTreeMap<String, BTreeMap<String, serde_json::Value>> = changed
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    assert_eq!(got_map, want, "changed records");

    assert_eq!(
        diff.schema.added_columns,
        exp["schema_changes"]["added_columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>()
    );
    assert!(diff.schema.removed_columns.is_empty());
    assert!(diff.schema.type_changes.is_empty());
    assert_eq!(
        diff.added.len() as u64,
        exp["counts"]["added"].as_u64().unwrap()
    );
    assert_eq!(
        diff.removed.len() as u64,
        exp["counts"]["removed"].as_u64().unwrap()
    );
    assert!(!diff.unchanged);
}

/// `unchanged_file_plans_nothing` — mapping the same file twice and
/// diffing produces an all-`Unchanged` plan.
#[test]
fn unchanged_file_plans_nothing() {
    let mut a_src =
        SqliteSource::open(&root().join("fixtures/legacy/samples/ei7/Sample/SampleDB.db"));
    let a = map_ei7(&mut a_src, SourceFormat::Ei7Sqlite).unwrap();
    let mut b_src =
        SqliteSource::open(&root().join("fixtures/legacy/samples/ei7/Sample/SampleDB.db"));
    let b = map_ei7(&mut b_src, SourceFormat::Ei7Sqlite).unwrap();
    let plan = refresh_plan(&view_datasets(&a), &b).unwrap();
    assert!(plan.is_unchanged(), "expected an unchanged plan: {plan:?}");
    assert_eq!(plan.views.len(), a.views.len());
    for v in &plan.views {
        assert!(
            matches!(v.outcome, RefreshOutcome::Unchanged),
            "{}: expected Unchanged, got {:?}",
            v.name,
            match &v.outcome {
                RefreshOutcome::Unchanged => "Unchanged",
                RefreshOutcome::Changed(_) => "Changed",
                RefreshOutcome::Missing => "Missing",
                RefreshOutcome::New => "New",
                RefreshOutcome::NoKey => "NoKey",
            }
        );
    }
}
