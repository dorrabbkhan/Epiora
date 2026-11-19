//! `corpus_export` (`corpus` feature) — `export-corpus`'s machinery:
//! one `analysis.R` + `expected.json` (+ `data.csv`) per corpus
//! fixture, plus the built-in `hostile_text` dataset-echo case
//! (v1.0 Task 17, spec §8.3).
//!
//! Every procedure/calculator/special driver goes through the same
//! emitters `export_r_procedure`/`export_r_calculator` use, so CI
//! runs the code paths the app exports.

use super::csv::{dataset_csv, missing_markers};
use super::r_calc::calc_call;
use super::r_emit::{
    datasets_env, header, lit_matrix, lit_str, lit_tables, needs_epitools, reader, ATTACH_EPITOOLS,
    PRELUDE,
};
use super::r_proc::{means_driver, tables_driver, var_spec};
use super::r_snippets::snippet_set;
use crate::dataset::Dataset;
use crate::EngineError;
use serde_json::Value;
use std::collections::BTreeMap;

fn invalid(reason: impl Into<String>) -> EngineError {
    EngineError::InvalidOptions {
        reason: reason.into(),
    }
}

/// One expected statistic: `value` (`None` = undefined leaf) and its
/// tolerance class, for `scripts/r-export/compare.mjs`.
#[derive(Debug, Clone)]
pub struct ExpectedStat {
    /// The fixture leaf's `value` (`None` for a `null` leaf).
    pub value: Option<f64>,
    /// `closed_form` or `iterative`.
    pub class: String,
}

/// What `export-corpus` writes for one case.
pub struct CaseExport {
    /// `analysis.R` text.
    pub script: String,
    /// `data.csv` bytes when the case reads a dataset.
    pub data_csv: Option<Vec<u8>>,
    /// `expected.json`'s `values` map: statistic path → leaf.
    pub expected: BTreeMap<String, ExpectedStat>,
}

/// The `expected` tree flattened to `path → {value, class}` —
/// `corpus::compare`'s walk: objects join with `.`, arrays `[i]`.
fn flatten(x: &Value, path: &str, out: &mut BTreeMap<String, ExpectedStat>) {
    match x {
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.retain(|k| k.as_str() != "r_default");
            keys.sort();
            if keys == ["class", "value"] {
                out.insert(
                    path.to_string(),
                    ExpectedStat {
                        value: m["value"].as_f64(),
                        class: m["class"].as_str().unwrap_or("").to_string(),
                    },
                );
                return;
            }
            for (k, v) in m {
                let child = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                flatten(v, &child, out);
            }
        }
        Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                flatten(v, &format!("{path}[{i}]"), out);
            }
        }
        _ => {}
    }
}

/// `header` + PRELUDE + snippets + body — mirrors `r_script`'s
/// assemble for the corpus drivers that need their own prelude.
fn assemble(title: &str, receipt: &str, seeds: &[&str], body: &str) -> Result<String, EngineError> {
    let snippets = snippet_set(seeds)?;
    let mut script = String::new();
    script.push_str(PRELUDE);
    script.push('\n');
    script.push_str(&snippets);
    script.push_str("\n\n");
    script.push_str(body);
    let attach = needs_epitools(&script);
    if attach {
        script.insert_str(PRELUDE.len(), ATTACH_EPITOOLS);
    }
    // The corpus path derives packages the same way the app path does
    // — reuse `r_script`'s header scan via the `pkg::` scan in
    // `header_for`'s twin here.
    let mut pkgs: Vec<String> = vec!["stats".into()];
    if attach {
        pkgs.push("epitools".into());
    }
    let bytes = script.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b':' && bytes[i + 1] == b':' {
            let mut start = i;
            while start > 0
                && (bytes[start - 1].is_ascii_alphanumeric()
                    || bytes[start - 1] == b'.'
                    || bytes[start - 1] == b'_')
            {
                start -= 1;
            }
            if start < i {
                pkgs.push(script[start..i].to_string());
            }
            i += 1;
        }
        i += 1;
    }
    pkgs.sort();
    pkgs.dedup();
    let refs: Vec<&str> = pkgs.iter().map(String::as_str).collect();
    Ok(format!("{}{script}", header(title, &refs, receipt)))
}

/// The `special.<fn>` driver: points literal + the per-point R
/// expression `special_fixture` receives — the same call the
/// generator made, with the grid read from `inputs` instead of the
/// generator's named constant.
fn special_driver(
    fn_name: &str,
    inputs: &Value,
) -> Result<(Vec<&'static str>, String), EngineError> {
    let (key, points) = if let Some(p) = inputs.get("points") {
        ("points", p)
    } else {
        (
            "grid",
            inputs
                .get("grid")
                .ok_or_else(|| invalid("special inputs: no points/grid"))?,
        )
    };
    // `points` is an array of argument arrays; emit each row as a
    // vector literal inside a list.
    let rows: Vec<String> = points
        .as_array()
        .ok_or_else(|| invalid("special points is not an array"))?
        .iter()
        .map(|r| {
            format!(
                "c({})",
                r.as_array()
                    .map(|c| c
                        .iter()
                        .map(crate::export::r_emit::lit_json)
                        .collect::<Vec<_>>()
                        .join(", "))
                    .unwrap_or_default()
            )
        })
        .collect();
    let pts = format!("list({})", rows.join(", "));
    let expr = match fn_name {
        "ln_gamma" => "leaf(lgamma(p[1]))",
        "normal_cdf" => "leaf(pnorm(p[1]))",
        "normal_sf" => "leaf(pnorm(p[1], lower.tail = FALSE))",
        "normal_quantile" => "leaf(qnorm(p[1]))",
        "chisq_sf" => "leaf(pchisq(p[1], p[2], lower.tail = FALSE))",
        "hypergeom_pmf" => "leaf(dhyper(p[1], p[2], p[3], p[4]))",
        "binomial_tail" => "leaf(pbinom(p[1], p[2], p[3], lower.tail = (p[4] == 1)))",
        "beta_quantile" => "leaf(qbeta(p[1], p[2], p[3]))",
        "t_cdf" => "leaf(pt(p[1], p[2]))",
        "t_quantile" => "leaf(qt(p[1], p[2]))",
        "f_sf" => "leaf(pf(p[1], p[2], p[3], lower.tail = FALSE))",
        "gamma_quantile" => "leaf(qgamma(p[1], p[2], scale = p[3]))",
        "chisq_quantile" => "leaf(qchisq(p[1], p[2]))",
        "poisson_tail" => {
            "if (p[3] == 1) leaf(ppois(p[1] - 1, p[2], lower.tail = FALSE)) else leaf(ppois(p[1], p[2]))"
        }
        "binomial_quantile" => "leaf(qbinom(p[1], p[2], p[3]))",
        "nc_hypergeom" => {
            "list(pmf = leaf(nch_dnhyper(h, psi)[h$x - h$lo + 1]), \
             mean = leaf(nch_mnhyper(h, psi)), \
             lower_tail = leaf(nch_pnhyper(h, h$x, psi)), \
             upper_tail = leaf(nch_pnhyper(h, h$x, psi, upper.tail = TRUE)))"
        }
        other => return Err(invalid(format!("no R expression for special.{other}"))),
    };
    let value_fn = if fn_name == "nc_hypergeom" {
        format!(
            "lapply(pts, function(p) {{ h <- nch_problem(m = p[1], n = p[2], k = p[3], x = p[4]); psi <- p[5]; {expr} }})"
        )
    } else {
        format!("lapply(pts, function(p) {expr})")
    };
    let body = format!(
        "pts <- {pts}\nspecial_fixture({}, pts, {value_fn}, key = {})\n",
        lit_str(fn_name),
        lit_str(key)
    );
    // The generated lambda's helpers (`nch_*`, `leaf`) live in the
    // body, not in `special_fixture`'s def, so they seed the closure.
    let seeds = if fn_name == "nc_hypergeom" {
        vec![
            "special_fixture",
            "leaf",
            "nch_problem",
            "nch_dnhyper",
            "nch_mnhyper",
            "nch_pnhyper",
        ]
    } else {
        vec!["special_fixture", "leaf"]
    };
    Ok((seeds, body))
}

/// Script + data.csv + expected map for one corpus fixture.
/// `actual` is the corpus run's serialized result — unused here (the
/// fixture's `expected` is the oracle `expected.json` mirrors) but
/// kept in the signature so callers pass the whole `CorpusLine`.
pub fn case_export(
    fixture: &Value,
    _actual: &Value,
    datasets: &BTreeMap<String, Dataset>,
) -> Result<CaseExport, EngineError> {
    let id = fixture["id"]
        .as_str()
        .ok_or_else(|| invalid("fixture has no id"))?
        .to_string();
    let procedure = fixture["procedure"]
        .as_str()
        .ok_or_else(|| invalid(format!("{id}: procedure is not a string")))?;
    let inputs = &fixture["inputs"];
    let receipt = format!("corpus:{id}");

    let ds_of = |name: &str| -> Result<&Dataset, EngineError> {
        datasets
            .get(name)
            .ok_or_else(|| invalid(format!("{id}: unknown dataset {name:?}")))
    };

    let (seeds, body, data_csv) = match procedure {
        "two_by_two" => match inputs.get("dataset").and_then(Value::as_str) {
            Some(dsid) => {
                let ds = ds_of(dsid)?;
                let markers = missing_markers(ds);
                let (s, mut b) = tables_driver(
                    &id,
                    dsid,
                    ds,
                    &markers,
                    &var_spec(&inputs["exposure"])?,
                    &var_spec(&inputs["outcome"])?,
                    &[],
                    false,
                )?;
                b = reader(ds.row_count()) + &b;
                (s, b, Some(dataset_csv(ds)))
            }
            None => (
                vec!["twobytwo_write"],
                format!(
                    "twobytwo_write({}, {})\n",
                    lit_str(&id),
                    lit_matrix(&inputs["table"])
                ),
                None,
            ),
        },
        "stratified_two_by_two" => match inputs.get("dataset").and_then(Value::as_str) {
            Some(dsid) => {
                let ds = ds_of(dsid)?;
                let markers = missing_markers(ds);
                let stratify_by: Vec<String> = inputs["stratify_by"]
                    .as_str()
                    .map(|s| vec![s.to_string()])
                    .unwrap_or_default();
                let (s, mut b) = tables_driver(
                    &id,
                    dsid,
                    ds,
                    &markers,
                    &var_spec(&inputs["exposure"])?,
                    &var_spec(&inputs["outcome"])?,
                    &stratify_by,
                    false,
                )?;
                b = reader(ds.row_count()) + &b;
                (s, b, Some(dataset_csv(ds)))
            }
            None => {
                let labels = inputs["labels"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|v| lit_str(v.as_str().unwrap_or("")))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                (
                    vec!["stratified_write"],
                    format!(
                        "stratified_write({}, {}, c({}))\n",
                        lit_str(&id),
                        lit_tables(&inputs["strata"]),
                        labels
                    ),
                    None,
                )
            }
        },
        "rxc" => (
            vec!["rxc_write"],
            format!(
                "rxc_write({}, {})\n",
                lit_str(&id),
                lit_matrix(&inputs["table"])
            ),
            None,
        ),
        "freq" => {
            let dsid = inputs["dataset"]
                .as_str()
                .ok_or_else(|| invalid(format!("{id}: freq fixture needs inputs.dataset")))?;
            let ds = ds_of(dsid)?;
            let markers = missing_markers(ds);
            let variable = inputs["variable"]
                .as_str()
                .ok_or_else(|| invalid(format!("{id}: inputs.variable")))?;
            let inc = inputs["include_missing"].as_bool().unwrap_or(false);
            let mut b = reader(ds.row_count());
            // `freq_write` computes labels itself — including the
            // Boolean Yes-first rule — so the verbatim call is exact.
            let mut env_body = String::new();
            datasets_env(&mut env_body, dsid, ds, &markers, &[(variable, false)])?;
            b.push_str(&env_body);
            b.push_str(&format!(
                "freq_write({}, {}, {}, {})\n",
                lit_str(&id),
                lit_str(dsid),
                lit_str(variable),
                if inc { "TRUE" } else { "FALSE" }
            ));
            (vec!["freq_write"], b, Some(dataset_csv(ds)))
        }
        "means" => {
            let dsid = inputs["dataset"]
                .as_str()
                .ok_or_else(|| invalid(format!("{id}: means fixture needs inputs.dataset")))?;
            let ds = ds_of(dsid)?;
            let markers = missing_markers(ds);
            let variable = inputs["variable"]
                .as_str()
                .ok_or_else(|| invalid(format!("{id}: inputs.variable")))?;
            let (s, mut b) = means_driver(
                &id,
                dsid,
                ds,
                &markers,
                variable,
                inputs.get("group_by").and_then(Value::as_str),
                inputs["include_missing"].as_bool().unwrap_or(false),
            )?;
            b = reader(ds.row_count()) + &b;
            (s, b, Some(dataset_csv(ds)))
        }
        name if name.starts_with("special.") => {
            let (s, b) = special_driver(&name[8..], inputs)?;
            (s, b, None)
        }
        name if name.starts_with("calc.") => {
            let (seed, call) = calc_call(&id, &name[5..], inputs)?;
            (vec![seed], format!("{call}\n"), None)
        }
        other => return Err(invalid(format!("{id}: unknown procedure {other:?}"))),
    };

    let script = assemble(&format!("corpus {id}"), &receipt, &seeds, &body)?;

    let mut expected = BTreeMap::new();
    flatten(&fixture["expected"], "", &mut expected);

    Ok(CaseExport {
        script,
        data_csv,
        expected,
    })
}
