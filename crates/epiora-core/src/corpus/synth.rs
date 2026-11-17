//! `synth` — the fixture → `(dataset, options)` mapping the corpus
//! explain and command parity lines share (v1.0 Task 11). Table
//! fixtures carry no dataset, so their declared counts are expanded
//! into a record-level `Dataset` the procedures can run, exactly as
//! `tests/explain_support/synth.rs` does for the explain tests —
//! this is the same construction, `EngineError` instead of panics so
//! `corpus-runner` and the wasm parity build can use it.

use crate::dataset::{Column, ColumnData, Dataset, Value as CellValue};
use crate::procedures::{FreqOptions, MeansOptions, ProportionCi, TablesOptions, VarSpec};
use crate::stats::Table2x2;
use crate::EngineError;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn corpus(reason: impl Into<String>) -> EngineError {
    EngineError::CorpusFormat {
        reason: reason.into(),
    }
}

/// One synthetic row: (stratum, exposure, outcome); `None` is a
/// missing cell — it excludes the record from the cross (spec §4.1)
/// while its present siblings still register their category, so a
/// declared level or stratum survives a zero count.
type Row = [Option<String>; 3];

/// A synthetic dataset whose cross-tab is a declared table. `levels`
/// in the `VarSpec`s pin the orientation (`"e"`/first row first).
fn expand(cells: &[Vec<Row>], stratified: bool) -> Result<Dataset, EngineError> {
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
    Dataset::new("synthetic", 1, columns)
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

/// The `tables` options of a dataset-backed `two_by_two` or
/// `stratified_two_by_two` fixture — the same options `run_case`
/// builds (`stratify_by` holds at most one level).
pub fn tables_options(fixture: &Value) -> Result<Value, EngineError> {
    let inputs = &fixture["inputs"];
    let spec = |k: &str| -> Result<VarSpec, EngineError> {
        Ok(VarSpec {
            name: inputs[k]
                .as_str()
                .ok_or_else(|| corpus(format!("inputs.{k} is not a string")))?
                .to_string(),
            levels: None,
        })
    };
    let stratify_by = match inputs["stratify_by"].as_str() {
        Some(s) => vec![s.to_string()],
        None => vec![],
    };
    Ok(json!(TablesOptions {
        exposure: spec("exposure")?,
        outcome: spec("outcome")?,
        stratify_by,
        include_missing: false,
    }))
}

/// The `(dataset, options)` an explain run needs for a `tables`
/// fixture — the corpus dataset when declared, else a synthetic one
/// expanded from the fixture's counts.
pub fn tables_case(
    fixture: &Value,
    datasets: &BTreeMap<String, Dataset>,
) -> Result<(Dataset, Value), EngineError> {
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
            let ds = datasets
                .get(id)
                .ok_or_else(|| corpus(format!("unknown dataset {id:?}")))?;
            Ok((ds.clone(), tables_options(fixture)?))
        }
        None => match inputs.get("strata") {
            Some(strata) => {
                let items = strata
                    .as_array()
                    .ok_or_else(|| corpus("inputs.strata is not an array"))?;
                let mut cells: Vec<Vec<Row>> = items
                    .iter()
                    .enumerate()
                    .map(|(i, t)| cells_2x2(&format!("s{i}"), &table_2x2(t)))
                    .collect();
                pad(&mut cells, &[(1, "e"), (1, "n"), (2, "o"), (2, "n")]);
                let (mut ex, mut out) = (oriented("e"), oriented("o"));
                ex.name = "E".into();
                out.name = "O".into();
                Ok((
                    expand(&cells, true)?,
                    json!(TablesOptions {
                        exposure: ex,
                        outcome: out,
                        stratify_by: vec!["S".to_string()],
                        include_missing: false,
                    }),
                ))
            }
            None => {
                let t = table_2x2(&inputs["table"]);
                let mut cells = vec![cells_2x2("s0", &t)];
                pad(&mut cells, &[(1, "e"), (1, "n"), (2, "o"), (2, "n")]);
                let (mut ex, mut out) = (oriented("e"), oriented("o"));
                ex.name = "E".into();
                out.name = "O".into();
                Ok((
                    expand(&cells, false)?,
                    json!(TablesOptions {
                        exposure: ex,
                        outcome: out,
                        stratify_by: vec![],
                        include_missing: false,
                    }),
                ))
            }
        },
    }
}

/// A synthetic `ROW`/`COL` dataset for an `rxc` table fixture.
pub fn rxc_case(fixture: &Value) -> Result<(Dataset, Value), EngineError> {
    let table = &fixture["inputs"]["table"];
    let rows_vec = table
        .as_array()
        .ok_or_else(|| corpus("inputs.table is not an array"))?;
    let first = rows_vec
        .first()
        .and_then(Value::as_array)
        .ok_or_else(|| corpus("inputs.table[0] is not an array"))?;
    let (nrows, ncols) = (rows_vec.len(), first.len());
    let mut rows: Vec<Row> = Vec::new();
    for (r, row) in rows_vec.iter().enumerate() {
        let cells = row
            .as_array()
            .ok_or_else(|| corpus(format!("inputs.table[{r}] is not an array")))?;
        for (c, n) in cells.iter().enumerate() {
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
    Ok((
        expand(&[rows], false)?,
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
    ))
}

/// The `freq` options of a fixture — the same options `run_case`
/// builds (`ci_method` pinned to Clopper–Pearson).
pub fn freq_opts(fixture: &Value) -> Result<Value, EngineError> {
    let inputs = &fixture["inputs"];
    Ok(json!(FreqOptions {
        variables: vec![inputs["variable"]
            .as_str()
            .ok_or_else(|| corpus("inputs.variable is not a string"))?
            .to_string()],
        stratify_by: inputs
            .get("stratify_by")
            .and_then(Value::as_str)
            .map(str::to_string),
        include_missing: inputs["include_missing"].as_bool().unwrap_or(false),
        ci_method: ProportionCi::ClopperPearson,
    }))
}

/// The `means` options of a fixture — the same options `run_case`
/// builds.
pub fn means_opts(fixture: &Value) -> Result<Value, EngineError> {
    let inputs = &fixture["inputs"];
    Ok(json!(MeansOptions {
        variable: inputs["variable"]
            .as_str()
            .ok_or_else(|| corpus("inputs.variable is not a string"))?
            .to_string(),
        group_by: inputs
            .get("group_by")
            .and_then(Value::as_str)
            .map(str::to_string),
        include_missing: inputs["include_missing"].as_bool().unwrap_or(false),
    }))
}

/// The corpus dataset a `freq`/`means` fixture names.
pub fn fixture_dataset<'a>(
    fixture: &Value,
    datasets: &'a BTreeMap<String, Dataset>,
) -> Result<&'a Dataset, EngineError> {
    let id = fixture["inputs"]["dataset"]
        .as_str()
        .ok_or_else(|| corpus("inputs.dataset is not a string"))?;
    datasets
        .get(id)
        .ok_or_else(|| corpus(format!("unknown dataset {id:?}")))
}
