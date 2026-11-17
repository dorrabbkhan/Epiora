//! Explain-trace tests (v1.0 Task 10, spec §6): registry coverage,
//! bit-identical final steps, declared formula ids, method-kind
//! classification, the `results_unchanged` corpus parity golden and
//! `ExplainUnknown`.
#![cfg(feature = "corpus")]

use epiora_core::corpus::run_all::load_datasets;
use epiora_core::corpus::{corpus_lines, run_case, CorpusLine};
use epiora_core::dataset::{Column, ColumnData, Dataset, Value as CellValue};
use epiora_core::explain::{
    explain_calculator, explain_procedure, ExplainKind, Explanation, FORMULAS,
};
use epiora_core::procedures::{FreqOptions, MeansOptions, ProportionCi, TablesOptions, VarSpec};
use epiora_core::stats::{Estimate, Table2x2, UndefinedReason};
use epiora_core::validation::{statistic_ids, ToleranceClass, REGISTRY};
use epiora_core::EngineError;
use serde_json::{json, Value};
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = sha2::Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// One synthetic row: (stratum, exposure, outcome); `None` is a
/// missing cell — it excludes the record from the cross (spec §4.1)
/// while its present siblings still register their category, so a
/// declared level or stratum survives a zero count.
type Row = [Option<String>; 3];

/// A synthetic dataset whose cross-tab is a declared table — table
/// fixtures carry no dataset, and `explain_procedure` needs one. `levels`
/// in the `VarSpec`s pin the orientation (`"e"`/first row first).
fn expand(cells: &[Vec<Row>], stratified: bool) -> Dataset {
    let (mut s, mut e, mut o) = (Vec::new(), Vec::new(), Vec::new());
    for stratum in cells {
        for row in stratum {
            s.push(row[0].clone());
            e.push(row[1].clone());
            o.push(row[2].clone());
        }
    }
    let mut columns = vec![
        Column {
            name: "E".to_string(),
            data: ColumnData::Text(e),
        },
        Column {
            name: "O".to_string(),
            data: ColumnData::Text(o),
        },
    ];
    if stratified {
        columns.insert(
            0,
            Column {
                name: "S".to_string(),
                data: ColumnData::Text(s),
            },
        );
    }
    Dataset::new("synthetic", 1, columns).expect("synthetic dataset")
}

/// `Table2x2` expanded to `(stratum, exposure, outcome)` rows —
/// `("s0","e","o")` is the `a` cell.
fn cells_2x2(s: &str, t: &Table2x2) -> Vec<Row> {
    let mut rows = Vec::new();
    let row = |e: &str, o: &str| {
        [
            Some(s.to_string()),
            Some(e.to_string()),
            Some(o.to_string()),
        ]
    };
    for _ in 0..t.a {
        rows.push(row("e", "o"));
    }
    for _ in 0..t.b {
        rows.push(row("e", "n"));
    }
    for _ in 0..t.c {
        rows.push(row("n", "o"));
    }
    for _ in 0..t.d {
        rows.push(row("n", "n"));
    }
    rows
}

/// Padding rows so every declared stratum label and every needed
/// category appears in the synthetic dataset: an all-zero stratum
/// has no rows of its own (its `s{i}` label would vanish) and a
/// declared table can leave `"e"`/`"o"`/`"n"` unobserved. A row
/// with a missing cell registers the category yet counts nowhere.
fn pad<S: AsRef<str>>(cells: &mut [Vec<Row>], want: &[(usize, S)]) {
    fn seen(cells: &[Vec<Row>], cell: usize, v: &str) -> bool {
        cells
            .iter()
            .flatten()
            .any(|r| r[cell].as_deref() == Some(v))
    }
    for (i, rows) in cells.iter_mut().enumerate() {
        if rows.is_empty() {
            rows.push([Some(format!("s{i}")), Some("e".into()), None]);
        }
    }
    for (cell, v) in want {
        if !seen(cells, *cell, v.as_ref()) {
            let mut row: Row = [Some("s0".into()), None, None];
            row[*cell] = Some(v.as_ref().to_string());
            cells[0].push(row);
        }
    }
}

fn table_2x2(t: &Value) -> Table2x2 {
    let cell = |r: usize, c: usize| t[r][c].as_u64().unwrap_or(0);
    Table2x2 {
        a: cell(0, 0),
        b: cell(0, 1),
        c: cell(1, 0),
        d: cell(1, 1),
    }
}

/// The `(dataset, options)` an explain call needs for a `tables`
/// fixture — the corpus dataset when declared, else a synthetic one
/// expanded from the fixture's counts.
fn tables_case(fixture: &Value, datasets: &BTreeMap<String, Dataset>) -> (Dataset, Value) {
    let inputs = &fixture["inputs"];
    // `"e"` is the first exposure level, `"o"` the first outcome
    // level — orientation is pinned because text levels are
    // ambiguous without it (spec §3.3).
    let oriented = |first: &str| VarSpec {
        name: String::new(),
        levels: Some([
            CellValue::Text(first.to_string()),
            CellValue::Text("n".into()),
        ]),
    };
    match inputs.get("dataset").and_then(Value::as_str) {
        Some(id) => {
            let ds = datasets.get(id).expect("dataset");
            let spec = |k: &str| VarSpec {
                name: inputs[k].as_str().expect("variable name").to_string(),
                levels: None,
            };
            let stratify_by = match inputs["stratify_by"].as_str() {
                Some(s) => vec![s.to_string()],
                None => vec![],
            };
            (
                ds.clone(),
                json!(TablesOptions {
                    exposure: spec("exposure"),
                    outcome: spec("outcome"),
                    stratify_by,
                    include_missing: false,
                }),
            )
        }
        None => match inputs.get("strata") {
            Some(strata) => {
                let mut cells: Vec<Vec<Row>> = strata
                    .as_array()
                    .expect("strata")
                    .iter()
                    .enumerate()
                    .map(|(i, t)| cells_2x2(&format!("s{i}"), &table_2x2(t)))
                    .collect();
                pad(&mut cells, &[(1, "e"), (1, "n"), (2, "o"), (2, "n")]);
                let (mut ex, mut out) = (oriented("e"), oriented("o"));
                ex.name = "E".into();
                out.name = "O".into();
                (
                    expand(&cells, true),
                    json!(TablesOptions {
                        exposure: ex,
                        outcome: out,
                        stratify_by: vec!["S".to_string()],
                        include_missing: false,
                    }),
                )
            }
            None => {
                let t = table_2x2(&inputs["table"]);
                let mut cells = vec![cells_2x2("s0", &t)];
                pad(&mut cells, &[(1, "e"), (1, "n"), (2, "o"), (2, "n")]);
                let (mut ex, mut out) = (oriented("e"), oriented("o"));
                ex.name = "E".into();
                out.name = "O".into();
                (
                    expand(&cells, false),
                    json!(TablesOptions {
                        exposure: ex,
                        outcome: out,
                        stratify_by: vec![],
                        include_missing: false,
                    }),
                )
            }
        },
    }
}

/// A synthetic `ROW`/`COL` dataset for an `rxc` table fixture.
fn rxc_case(fixture: &Value) -> (Dataset, Value) {
    let table = &fixture["inputs"]["table"];
    let (nrows, ncols) = (
        table.as_array().expect("table").len(),
        table[0].as_array().expect("row").len(),
    );
    let mut rows: Vec<Row> = Vec::new();
    for (r, row) in table.as_array().expect("table").iter().enumerate() {
        for (c, n) in row.as_array().expect("row").iter().enumerate() {
            for _ in 0..n.as_u64().unwrap_or(0) {
                rows.push([None, Some(format!("r{r}")), Some(format!("c{c}"))]);
            }
        }
    }
    let mut cells = vec![rows];
    pad(
        &mut cells,
        &(0..nrows)
            .map(|r| (1usize, format!("r{r}")))
            .chain((0..ncols).map(|c| (2usize, format!("c{c}"))))
            .collect::<Vec<_>>(),
    );
    let rows = std::mem::take(&mut cells[0]);
    (
        expand(&[rows], false),
        json!(TablesOptions {
            exposure: VarSpec {
                name: "E".into(),
                levels: None,
            },
            outcome: VarSpec {
                name: "O".into(),
                levels: None,
            },
            stratify_by: vec![],
            include_missing: false,
        }),
    )
}

/// Runs the fixture's explain call: `stat` is the full registry id
/// with concrete indices (`calc.proportion.estimate`,
/// `freq.categories[0].percent`).
fn explain_for(
    fixture: &Value,
    datasets: &BTreeMap<String, Dataset>,
    stat: &str,
) -> Result<Explanation, EngineError> {
    let procedure = fixture["procedure"].as_str().expect("procedure");
    match procedure {
        "freq" => {
            let inputs = &fixture["inputs"];
            let ds = datasets
                .get(inputs["dataset"].as_str().expect("inputs.dataset"))
                .expect("dataset");
            let opts = json!(FreqOptions {
                variables: vec![inputs["variable"].as_str().expect("variable").to_string()],
                stratify_by: inputs
                    .get("stratify_by")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                include_missing: inputs["include_missing"].as_bool().unwrap_or(false),
                ci_method: ProportionCi::ClopperPearson,
            });
            explain_procedure(ds, "freq", &opts, stat)
        }
        "means" => {
            let inputs = &fixture["inputs"];
            let ds = datasets
                .get(inputs["dataset"].as_str().expect("inputs.dataset"))
                .expect("dataset");
            let opts = json!(MeansOptions {
                variable: inputs["variable"].as_str().expect("variable").to_string(),
                group_by: inputs
                    .get("group_by")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                include_missing: inputs["include_missing"].as_bool().unwrap_or(false),
            });
            explain_procedure(ds, "means", &opts, stat)
        }
        "two_by_two" | "stratified_two_by_two" => {
            let (ds, opts) = tables_case(fixture, datasets);
            explain_procedure(&ds, "tables", &opts, stat)
        }
        "rxc" => {
            let (ds, opts) = rxc_case(fixture);
            explain_procedure(&ds, "tables", &opts, stat)
        }
        name if name.starts_with("calc.") => {
            explain_calculator(&name[5..], &fixture["inputs"], stat)
        }
        other => panic!("unexplainable procedure {other}"),
    }
}

/// The concrete-index paths a `[]`-pattern expands to inside `tree`;
/// a container path expands to every leaf beneath it.
fn concretize(tree: &Value, pattern: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut frontier: Vec<(&Value, String)> = vec![(tree, String::new())];
    for seg in pattern.split('.') {
        let (key, is_arr) = match seg.strip_suffix("[]") {
            Some(k) => (k, true),
            None => (seg, false),
        };
        let mut next = Vec::new();
        for (node, path) in frontier {
            let child = &node[key];
            if child.is_null() {
                continue;
            }
            let prefix = if path.is_empty() {
                key.to_string()
            } else {
                format!("{path}.{key}")
            };
            if is_arr {
                for (i, item) in child.as_array().expect("array").iter().enumerate() {
                    next.push((item, format!("{prefix}[{i}]")));
                }
            } else {
                next.push((child, prefix));
            }
        }
        frontier = next;
    }
    for (node, path) in frontier {
        match node {
            // A `{"undefined": code}` object is a leaf, not a
            // container — `statistic_ids` names it with the empty
            // relative id, which would emit `path.` here.
            Value::Object(m) if m.contains_key("undefined") => out.push(path),
            Value::Object(_) | Value::Array(_) => {
                for rel in statistic_ids(node) {
                    out.push(format!("{path}.{}", rel.replace("[]", "[0]")));
                }
            }
            _ => out.push(path),
        }
    }
    out
}

/// The `Estimate` a concrete leaf path carries in `tree` —
/// `{"undefined": code}` and bare numbers are leaves.
fn leaf_estimate(tree: &Value, concrete: &str) -> Estimate {
    let mut node = tree;
    for seg in concrete.split('.') {
        match seg.strip_suffix(']').and_then(|h| h.split_once('[')) {
            Some((k, i)) => node = &node[k][i.parse::<usize>().expect("index")],
            None => node = &node[seg],
        }
    }
    match node {
        Value::Number(n) => Estimate::Value(n.as_f64().expect("leaf is f64")),
        Value::Object(m) if m.contains_key("undefined") => {
            let code = m["undefined"].as_str().expect("undefined code");
            Estimate::Undefined {
                reason: UNDEFINED_CODES
                    .iter()
                    .find(|(_, c)| *c == code)
                    .map(|(r, _)| *r)
                    .expect("undefined reason code"),
            }
        }
        other => panic!("not a leaf: {other}"),
    }
}

const UNDEFINED_CODES: &[(UndefinedReason, &str)] = &[
    (UndefinedReason::ZeroCell, "zero_cell"),
    (UndefinedReason::ZeroMargin, "zero_margin"),
    (UndefinedReason::EmptyTable, "empty_table"),
    (UndefinedReason::NotBracketed, "not_bracketed"),
    (UndefinedReason::NoConvergence, "no_convergence"),
    (UndefinedReason::Unbounded, "unbounded"),
    (UndefinedReason::TooFew, "too_few"),
    (UndefinedReason::ZeroVariance, "zero_variance"),
    (UndefinedReason::OutOfRange, "out_of_range"),
    (UndefinedReason::TooLarge, "too_large"),
    (UndefinedReason::InfiniteSampleSize, "infinite_sample_size"),
    (UndefinedReason::ZeroExpected, "zero_expected"),
];

/// Every emitted leaf of every corpus case that a procedure or
/// calculator produces: `(statistic, explanation, leaf estimate)`.
/// `special.*` fixtures are low-level special functions — not a
/// procedure or calculator surface — and are skipped.
fn sweep() -> Vec<(String, Explanation, Estimate)> {
    let root = fixtures_root();
    let datasets = load_datasets(&root).expect("corpus datasets");
    let mut out = Vec::new();
    for line in corpus_lines(&root).expect("corpus") {
        let procedure = line.fixture["procedure"].as_str().expect("procedure");
        if procedure.starts_with("special.") {
            continue;
        }
        for pattern in statistic_ids(&line.output.actual) {
            for rel in concretize(&line.output.actual, &pattern) {
                let stat = format!("{procedure}.{rel}");
                match explain_for(&line.fixture, &datasets, &stat) {
                    Ok(exp) => {
                        out.push((stat.clone(), exp, leaf_estimate(&line.output.actual, &rel)))
                    }
                    Err(e) => panic!("{stat}: {e:?}"),
                }
            }
        }
    }
    out
}

fn registry_prefix(statistic: &str) -> &str {
    if statistic.starts_with("calc.") {
        let mut it = statistic.splitn(3, '.');
        match (it.next(), it.next()) {
            (Some(a), Some(b)) => &statistic[..a.len() + b.len() + 1],
            _ => statistic,
        }
    } else {
        match statistic.split_once('.') {
            Some((f, _)) => &statistic[..f.len()],
            None => statistic,
        }
    }
}

/// Every REGISTRY row with at least one covering fixture explains —
/// the task's `every_registry_statistic_explains` (spec §6).
#[test]
fn every_registry_statistic_explains() {
    let root = fixtures_root();
    let datasets = load_datasets(&root).expect("corpus datasets");
    let mut problems = Vec::new();
    let mut covered_rows = 0usize;
    let mut covered_stats = 0usize;
    for coverage in REGISTRY {
        if coverage.fixtures.is_empty() {
            problems.push(format!(
                "{}: registry row lists no covering fixtures",
                coverage.statistic
            ));
            continue;
        }
        let rel = if coverage.statistic.starts_with("calc.") {
            coverage
                .statistic
                .splitn(3, '.')
                .nth(2)
                .expect("calc.<id>.<path>")
        } else {
            coverage
                .statistic
                .split_once('.')
                .map(|(_, r)| r)
                .expect("family.<path>")
        };
        let prefix = registry_prefix(coverage.statistic);
        let mut row_explained = 0usize;
        for fixture_id in coverage.fixtures {
            let path = root.join(format!("{fixture_id}.json"));
            if !path.exists() {
                problems.push(format!(
                    "{}: covering fixture {fixture_id}.json missing",
                    coverage.statistic
                ));
                continue;
            }
            let fixture: Value =
                serde_json::from_str(&std::fs::read_to_string(&path).expect("read fixture"))
                    .expect("parse fixture");
            let output = match run_case(&fixture, &datasets) {
                Ok(o) => o,
                Err(e) => {
                    problems.push(format!(
                        "{}: run_case {fixture_id}: {e:?}",
                        coverage.statistic
                    ));
                    continue;
                }
            };
            for concrete in concretize(&output.actual, rel) {
                let stat = format!("{prefix}.{concrete}");
                match explain_for(&fixture, &datasets, &stat) {
                    Ok(ex) => {
                        assert_eq!(ex.statistic, stat, "explanation carries the asked path");
                        row_explained += 1;
                    }
                    Err(e) => problems.push(format!("{stat} ({fixture_id}): {e:?}")),
                }
            }
        }
        if row_explained == 0 {
            problems.push(format!(
                "{}: no concrete instance explained by any covering fixture",
                coverage.statistic
            ));
        } else {
            covered_rows += 1;
            covered_stats += row_explained;
        }
    }
    assert!(
        problems.is_empty(),
        "registry explain coverage failures:\n{}",
        problems.join("\n")
    );
    assert!(covered_rows > 0 && covered_stats > 0);
    eprintln!(
        "every_registry_statistic_explains: {covered_rows} rows, {covered_stats} concrete stats"
    );
}

/// `last_step_equals_result_bitwise` — the last step's `result` is
/// the run's own leaf, bit for bit (spec §6).
#[test]
fn last_step_equals_result_bitwise() {
    let seen = sweep();
    assert!(!seen.is_empty());
    let mut compared = 0usize;
    let mut undefined = 0usize;
    for (stat, exp, want) in &seen {
        let last = exp.steps.last().expect("at least one step");
        match (&last.result, want) {
            (Estimate::Value(a), Estimate::Value(b)) => assert_eq!(
                a.to_bits(),
                b.to_bits(),
                "{stat}: trace result is not bit-identical"
            ),
            (a, b) => {
                assert_eq!(a, b, "{stat}: trace result is not identical");
                undefined += 1;
            }
        }
        compared += 1;
    }
    eprintln!("last_step_equals_result_bitwise: {compared} leaves, {undefined} undefined");
}

/// `formula_ids_are_declared` — every emitted step names a `FORMULAS`
/// id.
#[test]
fn formula_ids_are_declared() {
    let declared: BTreeSet<&str> = FORMULAS.iter().copied().collect();
    let mut problems = Vec::new();
    for (stat, exp, _) in sweep() {
        for step in &exp.steps {
            if !declared.contains(step.formula.as_str()) {
                problems.push(format!("{stat}: undeclared formula id {}", step.formula));
            }
            for (k, v) in &step.values {
                assert!(v.is_finite(), "{stat}: non-finite value {k}={v}");
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// `iterative_statistics_are_method_kind` — every iterative-class
/// registry statistic is `ExplainKind::Method` with one step.
#[test]
fn iterative_statistics_are_method_kind() {
    let root = fixtures_root();
    let datasets = load_datasets(&root).expect("corpus datasets");
    let mut problems = Vec::new();
    let mut checked = 0usize;
    for coverage in REGISTRY {
        if coverage.class != ToleranceClass::Iterative {
            continue;
        }
        let rel = if coverage.statistic.starts_with("calc.") {
            coverage.statistic.splitn(3, '.').nth(2).expect("calc path")
        } else {
            coverage
                .statistic
                .split_once('.')
                .map(|(_, r)| r)
                .expect("rel")
        };
        let prefix = registry_prefix(coverage.statistic);
        if coverage.fixtures.is_empty() {
            problems.push(format!(
                "{}: iterative registry row lists no covering fixtures",
                coverage.statistic
            ));
            continue;
        }
        let mut row_checked = 0usize;
        for fixture_id in coverage.fixtures {
            let path = root.join(format!("{fixture_id}.json"));
            if !path.exists() {
                problems.push(format!(
                    "{}: covering fixture {fixture_id}.json missing",
                    coverage.statistic
                ));
                continue;
            }
            let fixture: Value =
                serde_json::from_str(&std::fs::read_to_string(&path).expect("read"))
                    .expect("parse");
            let output = match run_case(&fixture, &datasets) {
                Ok(o) => o,
                Err(e) => {
                    problems.push(format!(
                        "{}: run_case {fixture_id}: {e:?}",
                        coverage.statistic
                    ));
                    continue;
                }
            };
            for concrete in concretize(&output.actual, rel) {
                let stat = format!("{prefix}.{concrete}");
                match explain_for(&fixture, &datasets, &stat) {
                    Ok(ex) => {
                        if ex.kind != ExplainKind::Method || ex.steps.len() != 1 {
                            problems.push(format!(
                                "{stat}: kind {:?}, {} steps",
                                ex.kind,
                                ex.steps.len()
                            ));
                        }
                        checked += 1;
                        row_checked += 1;
                    }
                    Err(e) => problems.push(format!("{stat}: {e:?}")),
                }
            }
        }
        if row_checked == 0 {
            problems.push(format!(
                "{}: iterative row exercised by no fixture",
                coverage.statistic
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "iterative statistics not Method traces:\n{}",
        problems.join("\n")
    );
    assert!(checked > 0, "no iterative statistic was exercised");
    eprintln!("iterative_statistics_are_method_kind: {checked} concrete stats");
}

/// `results_unchanged` — the corpus parity hash golden generated on
/// main (owner decision D1, v1.0 Task 10). A row whose fixture file
/// (or dataset file) hash changed is reported, not compared; a row
/// whose fixture vanished from the run is reported; every unchanged
/// row's parity hash must match, and at least one row compares.
#[test]
fn results_unchanged() {
    let root = fixtures_root();
    let golden_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/corpus_parity_main.tsv");
    let golden = std::fs::read_to_string(&golden_path).expect("read golden");
    let lines: BTreeMap<(String, String), CorpusLine> = corpus_lines(&root)
        .expect("corpus")
        .into_iter()
        .map(|l| ((l.output.id.clone(), l.rel_path.clone()), l))
        .collect();

    let mut compared = 0usize;
    let mut notes = Vec::new();
    let mut failures = Vec::new();
    for (n, row) in golden.lines().enumerate() {
        let cols: Vec<&str> = row.split('\t').collect();
        let (id, fixture_path, fixture_sha, dataset_path, dataset_sha, parity_sha) =
            match cols.as_slice() {
                [a, b, c, d, e, f] => (*a, *b, *c, *d, *e, *f),
                other => panic!("golden row {n} has {} columns", other.len()),
            };
        let rel = fixture_path
            .strip_prefix("fixtures/")
            .expect("golden fixture path is fixtures/<rel>");
        let Some(line) = lines.get(&(id.to_string(), rel.to_string())) else {
            // D1: a missing row fails when its fixture file (and
            // dataset file, if any) still exists unchanged — only a
            // genuinely changed or removed file is a note.
            let fpath = root.join(rel);
            let fixture_same = fpath.exists()
                && sha256_hex(&std::fs::read(&fpath).expect("read fixture")) == fixture_sha;
            let dataset_same = if dataset_path == "-" {
                true
            } else {
                let drel = dataset_path
                    .strip_prefix("fixtures/")
                    .expect("golden dataset path is fixtures/<rel>");
                let dpath = root.join(drel);
                dpath.exists()
                    && sha256_hex(&std::fs::read(&dpath).expect("read dataset")) == dataset_sha
            };
            if fixture_same && dataset_same {
                failures.push(format!(
                    "row {n} ({id}): fixture absent from corpus run                      but its fixture/dataset files are unchanged"
                ));
            } else {
                notes.push(format!(
                    "row {n} ({id}): fixture absent from corpus run;                      source file changed or removed"
                ));
            }
            continue;
        };
        let current_sha =
            sha256_hex(&std::fs::read(root.join(&line.rel_path)).expect("read fixture"));
        if current_sha != fixture_sha {
            notes.push(format!(
                "row {n} ({id}): fixture file changed — not compared"
            ));
            continue;
        }
        if dataset_path != "-" {
            let drel = dataset_path
                .strip_prefix("fixtures/")
                .expect("golden dataset path is fixtures/<rel>");
            let dsha = sha256_hex(&std::fs::read(root.join(drel)).expect("read dataset"));
            if dsha != dataset_sha {
                notes.push(format!(
                    "row {n} ({id}): dataset file changed — not compared"
                ));
                continue;
            }
        }
        let got = sha256_hex(line.output.parity_line().as_bytes());
        if got != parity_sha {
            failures.push(format!(
                "row {n} ({id}): parity hash changed\n  want {parity_sha}\n  got  {got}"
            ));
        }
        compared += 1;
    }
    for note in &notes {
        eprintln!("results_unchanged note: {note}");
    }
    assert!(
        failures.is_empty(),
        "parity results changed:\n{}",
        failures.join("\n")
    );
    assert!(compared >= 1, "no golden row was compared");
    eprintln!(
        "results_unchanged: {compared} rows compared, {} notes",
        notes.len()
    );
}

/// `explain_unknown` — a statistic that resolves to no leaf is a
/// typed `ExplainUnknown`, never a panic (spec §6).
#[test]
fn explain_unknown() {
    let datasets = load_datasets(&fixtures_root()).expect("corpus datasets");
    let ds = datasets.values().next().expect("a dataset");
    let variable = ds.columns()[0].name.clone();
    let opts = json!(FreqOptions {
        variables: vec![variable],
        stratify_by: None,
        include_missing: false,
        ci_method: ProportionCi::ClopperPearson,
    });
    match explain_procedure(ds, "freq", &opts, "freq.no_such_leaf") {
        Err(EngineError::ExplainUnknown { statistic }) => {
            assert_eq!(statistic, "freq.no_such_leaf");
        }
        other => panic!("expected ExplainUnknown, got {other:?}"),
    }
    let err = explain_calculator(
        "proportion",
        &json!({
            "numerator": 10,
            "denominator": 45,
            "conf_level": 95,
            "multiplier": 100,
            "population_size": 1000000,
            "compare_with": 50
        }),
        "calc.proportion.no_such_leaf",
    )
    .expect_err("unknown calc statistic");
    assert_eq!(err.code(), "explain_unknown");
}
