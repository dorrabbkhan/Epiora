//! `every_reference_program_step_matches` (v1.1 plan Task 11, spec
//! §15.2): replays every `fixtures/ei7/v11/*.json` reference program
//! step by step against the raw `.mdb` tables (loaded natively through
//! `epiora-mdb` + `epiora-legacy::decode_batch`), comparing every
//! `results` entry at the `ei7_display` tolerance (`0.5 × 10^-d`, d =
//! the fixture's displayed decimals) and every LIST row exactly.
//!
//! DISPLAY TABLES / DBVARIABLES rows are EI7-internal dumps (Access
//! formats and PK index names the EPTB batches do not carry) and are
//! not compared — their `results` are empty.
//!
//! The corpus runner lives in `tests/common/corpus/`.

mod common;
use common::corpus::*;
use epiora_mdb::Mdb;
use std::collections::BTreeMap;

#[test]
fn every_reference_program_step_matches() {
    let dir = root().join("fixtures/ei7/v11");
    let mut mdbs = BTreeMap::new();
    for name in ["Sample", "Nutrition"] {
        let p = root().join(format!("fixtures/legacy/samples/ei7/{name}/{name}.mdb"));
        mdbs.insert(
            name.to_ascii_lowercase(),
            Mdb::open(&p).unwrap_or_else(|e| panic!("open {p:?}: {e}")),
        );
    }
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("v11 fixture dir")
        .filter_map(|e| {
            let p = e.expect("entry").path();
            (p.extension().is_some_and(|x| x == "json")).then_some(p)
        })
        .collect();
    files.sort();
    assert!(files.len() >= 7, "expected the Task 3 programs in {dir:?}");
    for path in files {
        let fixture: Fixture = serde_json::from_str(
            &std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {path:?}")),
        )
        .unwrap_or_else(|_| panic!("parse {path:?}"));
        let mut sess: Option<Session> = None;
        let mut i = 0;
        while i < fixture.steps.len() {
            let step = &fixture.steps[i];
            if step
                .command
                .trim_start()
                .to_ascii_uppercase()
                .starts_with("READ ")
            {
                let (file, table) = read_target(step.command[4..].trim());
                match &mut sess {
                    Some(s) => s.read(&file, &table),
                    None => sess = Some(Session::open(&mdbs, &file, &table)),
                }
                check_results(
                    &selection_json(sess.as_ref().expect("session"), 0, false),
                    step,
                    "read",
                );
                i += 1;
                continue;
            }
            let s = sess
                .as_mut()
                .unwrap_or_else(|| panic!("no READ before L{}", step.line));
            i += run_line(s, &fixture.steps, i);
        }
    }
}
