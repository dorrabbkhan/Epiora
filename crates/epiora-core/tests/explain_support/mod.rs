//! Shared machinery of `tests/explain.rs` (v1.0 Task 10 fix round
//! 3): each corpus case runs its procedure/calculator ONCE through
//! the `run`/`trace` split of `epiora_core::explain` and every
//! emitted statistic traces off that output, all behind one
//! `OnceLock` sweep the explain tests share. `explain_for` keeps the
//! per-statistic public API path so `every_registry_statistic_explains`
//! can assert the two paths produce identical traces.

mod synth;
mod tree;

use epiora_core::corpus::run_all::load_datasets;
use epiora_core::corpus::{corpus_lines_with, run_case, CorpusLine};
use epiora_core::dataset::Dataset;
use epiora_core::explain::{
    explain_calculator, explain_procedure, run_calculator, run_procedure, CalcRun, Explanation,
    ProcedureRun,
};
use epiora_core::stats::Estimate;
use epiora_core::validation::statistic_ids;
use epiora_core::EngineError;
use serde_json::Value;
use sha2::Digest;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use synth::{fixture_dataset, freq_opts, means_opts, rxc_case, tables_case};
use tree::leaf_estimate;

pub use tree::concretize;

pub fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = sha2::Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// The corpus datasets, loaded once per test process.
pub fn datasets() -> &'static BTreeMap<String, Dataset> {
    static DATASETS: OnceLock<BTreeMap<String, Dataset>> = OnceLock::new();
    DATASETS.get_or_init(|| load_datasets(&fixtures_root()).expect("corpus datasets"))
}

/// Runs the fixture's explain call through the public API: `stat`
/// is the full registry id with concrete indices
/// (`calc.proportion.estimate`, `freq.categories[0].percent`).
pub fn explain_for(
    fixture: &Value,
    datasets: &BTreeMap<String, Dataset>,
    stat: &str,
) -> Result<Explanation, EngineError> {
    let procedure = fixture["procedure"].as_str().expect("procedure");
    match procedure {
        "freq" => explain_procedure(
            fixture_dataset(fixture, datasets),
            "freq",
            &freq_opts(fixture),
            stat,
        ),
        "means" => explain_procedure(
            fixture_dataset(fixture, datasets),
            "means",
            &means_opts(fixture),
            stat,
        ),
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

/// The run half of a fixture's explain calls — one procedure or
/// calculator run that every emitted statistic traces from.
pub enum ExplainRun {
    Proc(ProcedureRun),
    Calc(CalcRun),
}

impl ExplainRun {
    pub fn trace(&self, stat: &str) -> Result<Explanation, EngineError> {
        match self {
            ExplainRun::Proc(r) => r.trace(stat),
            ExplainRun::Calc(r) => r.trace(stat),
        }
    }
}

/// One explain run for `fixture` — `explain_for`'s run half, shared
/// by every statistic the case emits.
fn run_for(
    fixture: &Value,
    datasets: &BTreeMap<String, Dataset>,
) -> Result<ExplainRun, EngineError> {
    let procedure = fixture["procedure"].as_str().expect("procedure");
    match procedure {
        "freq" => run_procedure(
            fixture_dataset(fixture, datasets),
            "freq",
            &freq_opts(fixture),
        )
        .map(ExplainRun::Proc),
        "means" => run_procedure(
            fixture_dataset(fixture, datasets),
            "means",
            &means_opts(fixture),
        )
        .map(ExplainRun::Proc),
        "two_by_two" | "stratified_two_by_two" => {
            let (ds, opts) = tables_case(fixture, datasets);
            run_procedure(&ds, "tables", &opts).map(ExplainRun::Proc)
        }
        "rxc" => {
            let (ds, opts) = rxc_case(fixture);
            run_procedure(&ds, "tables", &opts).map(ExplainRun::Proc)
        }
        name if name.starts_with("calc.") => {
            run_calculator(&name[5..], &fixture["inputs"]).map(ExplainRun::Calc)
        }
        other => panic!("unexplainable procedure {other}"),
    }
}

/// Every emitted leaf of `actual` traced through one explain run of
/// `fixture`: `(statistic, explanation, leaf estimate)` — the same
/// triple the old per-statistic sweep produced. The run is built on
/// the first leaf, so a fixture with no leaves explains nothing and
/// never runs.
fn trace_case(
    fixture: &Value,
    actual: &Value,
    datasets: &BTreeMap<String, Dataset>,
) -> Vec<(String, Explanation, Estimate)> {
    let procedure = fixture["procedure"].as_str().expect("procedure");
    let mut run: Option<Result<ExplainRun, EngineError>> = None;
    let mut out = Vec::new();
    for pattern in statistic_ids(actual) {
        for rel in concretize(actual, &pattern) {
            let stat = format!("{procedure}.{rel}");
            if run.is_none() {
                run = Some(run_for(fixture, datasets));
            }
            let exp = match run.as_ref().expect("run") {
                Ok(r) => r.trace(&stat),
                Err(e) => Err(e.clone()),
            }
            .unwrap_or_else(|e| panic!("{stat}: {e:?}"));
            out.push((stat, exp, leaf_estimate(actual, &rel)));
        }
    }
    out
}

/// One corpus case's run-once sweep entry: the `CorpusLine` plus
/// every emitted leaf's trace off the case's single
/// procedure/calculator run (`special.*` cases carry no traces —
/// they are not a procedure or calculator surface).
pub struct CaseRun {
    pub line: CorpusLine,
    pub exps: Vec<(String, Explanation, Estimate)>,
}

impl CaseRun {
    /// The traced explanation of one concrete statistic, if the
    /// case emits it.
    pub fn exp(&self, stat: &str) -> Option<&Explanation> {
        self.exps
            .iter()
            .find(|(s, _, _)| s == stat)
            .map(|(_, e, _)| e)
    }

    /// The `dir/id` fixture key `REGISTRY`'s `fixtures` lists use.
    pub fn fixture_key(&self) -> String {
        self.line
            .rel_path
            .strip_suffix(".json")
            .unwrap_or(&self.line.rel_path)
            .to_string()
    }
}

/// Every corpus case traced once — the `OnceLock` sweep the explain
/// tests share.
pub fn sweep() -> &'static [CaseRun] {
    static SWEEP: OnceLock<Vec<CaseRun>> = OnceLock::new();
    SWEEP
        .get_or_init(|| {
            let root = fixtures_root();
            corpus_lines_with(&root, datasets())
                .expect("corpus")
                .into_iter()
                .map(|line| {
                    let procedure = line.fixture["procedure"].as_str().expect("procedure");
                    let exps = if procedure.starts_with("special.") {
                        Vec::new()
                    } else {
                        trace_case(&line.fixture, &line.output.actual, datasets())
                    };
                    CaseRun { line, exps }
                })
                .collect()
        })
        .as_slice()
}

/// The sweep indexed by `dir/id` fixture key.
fn sweep_by_id() -> &'static BTreeMap<String, &'static CaseRun> {
    static MAP: OnceLock<BTreeMap<String, &'static CaseRun>> = OnceLock::new();
    MAP.get_or_init(|| sweep().iter().map(|c| (c.fixture_key(), c)).collect())
}

/// The leaf traces of one covering fixture — the shared sweep case
/// when `corpus_lines` visits it, else a fresh run kept in `owned`
/// (a fixture file outside the corpus `FAMILIES` list explains the
/// same way it did when every statistic re-ran the case).
pub fn case_for<'a>(
    fixture_id: &str,
    owned: &'a mut BTreeMap<String, CaseRun>,
    root: &Path,
    datasets: &BTreeMap<String, Dataset>,
) -> Result<&'a CaseRun, String> {
    if let Some(case) = sweep_by_id().get(fixture_id) {
        return Ok(case);
    }
    if !owned.contains_key(fixture_id) {
        let path = root.join(format!("{fixture_id}.json"));
        let text = std::fs::read_to_string(&path).map_err(|e| format!("read: {e}"))?;
        let fixture: Value = serde_json::from_str(&text).map_err(|e| format!("parse: {e}"))?;
        let output = run_case(&fixture, datasets).map_err(|e| format!("run_case: {e:?}"))?;
        let rel_path = format!("{fixture_id}.json");
        let exps = if fixture["procedure"]
            .as_str()
            .expect("procedure")
            .starts_with("special.")
        {
            Vec::new()
        } else {
            trace_case(&fixture, &output.actual, datasets)
        };
        owned.insert(
            fixture_id.to_string(),
            CaseRun {
                line: CorpusLine {
                    rel_path,
                    fixture,
                    output,
                },
                exps,
            },
        );
    }
    Ok(&owned[fixture_id])
}

/// The registry id's family prefix (`freq`, `calc.proportion`).
pub fn registry_prefix(statistic: &str) -> &str {
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

/// The registry statistic's relative path (`freq.categories[].percent`
/// → `categories[].percent`, `calc.proportion.estimate` → `estimate`).
pub fn registry_rel(statistic: &str) -> &str {
    if statistic.starts_with("calc.") {
        statistic.splitn(3, '.').nth(2).expect("calc.<id>.<path>")
    } else {
        statistic
            .split_once('.')
            .map(|(_, r)| r)
            .expect("family.<path>")
    }
}

/// Bit-for-bit equality of two `Explanation`s — `Estimate::Value`
/// compared on `to_bits`: the public API and the run-once trace
/// must never diverge.
pub fn assert_same_explanation(public: &Explanation, traced: &Explanation, stat: &str) {
    assert_eq!(public.statistic, traced.statistic, "{stat}: statistic");
    assert_eq!(public.kind, traced.kind, "{stat}: kind");
    assert_eq!(public.steps.len(), traced.steps.len(), "{stat}: step count");
    for (i, (a, b)) in public.steps.iter().zip(traced.steps.iter()).enumerate() {
        assert_eq!(a.formula, b.formula, "{stat}: step {i} formula");
        assert_eq!(
            a.values.len(),
            b.values.len(),
            "{stat}: step {i} value count"
        );
        for (k, v) in &a.values {
            let w = b
                .values
                .get(k)
                .unwrap_or_else(|| panic!("{stat}: step {i} missing value {k}"));
            assert_eq!(v.to_bits(), w.to_bits(), "{stat}: step {i} value {k}");
        }
        match (&a.result, &b.result) {
            (Estimate::Value(x), Estimate::Value(y)) => {
                assert_eq!(x.to_bits(), y.to_bits(), "{stat}: step {i} result")
            }
            (x, y) => assert_eq!(x, y, "{stat}: step {i} result"),
        }
    }
}
