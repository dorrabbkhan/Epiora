#![cfg(feature = "corpus")]

//! Task 11a: with `stratify_by` set, `tables` takes its crude
//! `TwoByTwoStats` from `StratifiedStats::crude` instead of running
//! `two_by_two` a second time on the pooled table. The crude it
//! returns must stay bit-identical to `two_by_two` on the pooled
//! table — exactly what it returned before the change.

use epiora_core::corpus::load_dataset;
use epiora_core::dataset::Dataset;
use epiora_core::procedures::{
    tables, ProcOutput, RunContext, TablesOptions, TablesResult, VarSpec,
};
use epiora_core::stats::{two_by_two, Table2x2, TwoByTwoStats};
use epiora_core::EngineError;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".to_string(),
        project: None,
    }
}

fn spec(name: &str) -> VarSpec {
    VarSpec {
        name: name.to_string(),
        levels: None,
    }
}

fn fixture(name: &str) -> Value {
    let path = fixtures_root()
        .join("stratified")
        .join(format!("{name}.json"));
    serde_json::from_str(&std::fs::read_to_string(&path).expect("read fixture")).expect("parse")
}

fn strata_of(fixture: &Value) -> Vec<Table2x2> {
    fixture["inputs"]["strata"]
        .as_array()
        .expect("inputs.strata")
        .iter()
        .map(|t| Table2x2 {
            a: t[0][0].as_u64().expect("a"),
            b: t[0][1].as_u64().expect("b"),
            c: t[1][0].as_u64().expect("c"),
            d: t[1][1].as_u64().expect("d"),
        })
        .collect()
}

fn pooled(strata: &[Table2x2]) -> Table2x2 {
    strata.iter().fold(
        Table2x2 {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
        },
        |s, t| Table2x2 {
            a: s.a.saturating_add(t.a),
            b: s.b.saturating_add(t.b),
            c: s.c.saturating_add(t.c),
            d: s.d.saturating_add(t.d),
        },
    )
}

/// Every f64 of a serialized `TwoByTwoStats`, as bit patterns — a
/// bit-identity comparison, including `-0.0` vs `0.0`.
fn bits(stats: &TwoByTwoStats) -> Vec<u64> {
    fn walk(v: &Value, out: &mut Vec<u64>) {
        match v {
            Value::Number(n) => out.push(n.as_f64().unwrap_or(f64::NAN).to_bits()),
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            Value::Object(o) => o.values().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(&serde_json::to_value(stats).expect("serialize"), &mut out);
    out
}

/// The dataset a case runs on: the corpus dataset it names, or, for a
/// strata-only fixture, a synthesized dataset that tabulates to the
/// declared strata — a stratum whose declared table is all zeros gets
/// two records with the exposure missing so the category exists but
/// its table is empty.
fn dataset_of(fixture: &Value) -> Result<Dataset, EngineError> {
    if let Some(id) = fixture["inputs"]["dataset"].as_str() {
        let path = fixtures_root()
            .join("data")
            .join(format!("{id}.dataset.json"));
        return load_dataset(&std::fs::read_to_string(&path).expect("read dataset"));
    }
    let strata = strata_of(fixture);
    let labels = fixture["inputs"]["labels"]
        .as_array()
        .expect("inputs.labels")
        .iter()
        .map(|l| l.as_str().expect("label"))
        .collect::<Vec<_>>();
    let (mut e, mut o, mut s) = (Vec::new(), Vec::new(), Vec::new());
    for (t, label) in strata.iter().zip(&labels) {
        if t.a + t.b + t.c + t.d == 0 {
            for _ in 0..2 {
                e.push(None);
                o.push(Some(true));
                s.push(Some(label.to_string()));
            }
            continue;
        }
        for &(ev, ov, n) in &[
            (true, true, t.a),
            (true, false, t.b),
            (false, true, t.c),
            (false, false, t.d),
        ] {
            for _ in 0..n {
                e.push(Some(ev));
                o.push(Some(ov));
                s.push(Some(label.to_string()));
            }
        }
    }
    load_dataset(
        &serde_json::json!({
            "id": fixture["id"].as_str().expect("id"),
            "version": 1,
            "columns": [
                {"name": "E", "type": "boolean", "values": e},
                {"name": "O", "type": "boolean", "values": o},
                {"name": "S", "type": "text", "values": s},
            ]
        })
        .to_string(),
    )
}

/// The `crude` and the stratified stats' own `crude` of a `tables`
/// output must be bit-identical to `two_by_two` on the pooled declared
/// strata — the same value `tables` reuses.
fn assert_crude_is_pooled(case: &str, out: &ProcOutput<TablesResult>, declared: &[Table2x2]) {
    let TablesResult::TwoByTwo {
        crude,
        stratified: Some(st),
        ..
    } = &out.result
    else {
        panic!("{case}: expected a stratified 2×2 result");
    };
    let reference = two_by_two(&pooled(declared));
    assert_eq!(
        bits(crude),
        bits(&reference),
        "{case}: crude is not two_by_two of the pooled table"
    );
    assert_eq!(
        bits(&st.stats.crude),
        bits(&reference),
        "{case}: stratified crude is not two_by_two of the pooled table"
    );
}

#[test]
fn oswego_vanilla_by_sex_crude_is_pooled_two_by_two() {
    let fixture = fixture("oswego_vanilla_by_sex");
    let ds = dataset_of(&fixture).expect("dataset");
    let opts = TablesOptions {
        exposure: spec("VANILLA"),
        outcome: spec("ILL"),
        stratify_by: vec!["SEX".to_string()],
        include_missing: false,
    };
    let out = tables(&ds, &opts, &ctx()).expect("tables");
    assert_crude_is_pooled(
        "stratified/oswego_vanilla_by_sex",
        &out,
        &strata_of(&fixture),
    );
}

#[test]
fn empty_stratum_crude_is_pooled_two_by_two() {
    let fixture = fixture("empty_stratum");
    let ds = dataset_of(&fixture).expect("dataset");
    let opts = TablesOptions {
        exposure: spec("E"),
        outcome: spec("O"),
        stratify_by: vec!["S".to_string()],
        include_missing: false,
    };
    let out = tables(&ds, &opts, &ctx()).expect("tables");
    let TablesResult::TwoByTwo {
        stratified: Some(st),
        ..
    } = &out.result
    else {
        panic!("empty_stratum: expected a stratified 2×2 result");
    };
    assert_eq!(
        st.stats.excluded_strata,
        vec![1],
        "the zero stratum must be excluded and listed"
    );
    assert_crude_is_pooled("stratified/empty_stratum", &out, &strata_of(&fixture));
}
