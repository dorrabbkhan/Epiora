//! `r_proc` — the dataset drivers (v1.0 Task 17, spec §8.1):
//! `tables`, `freq` and `means` each emit the `data.csv` reader's
//! `DATASETS` env plus the verbatim writer call, with a `stopifnot`
//! against the engine's own cross-tab counts where R rebuilds the
//! table. Both `export_r_procedure` (app) and `corpus_export` (CI)
//! go through these.

use super::r_emit::{datasets_env, lit_num, lit_str, XTAB_PRELUDE};
use super::r_script::invalid;
use crate::dataset::Dataset;
use crate::procedures::VarSpec;
use crate::tabulate::{categories, cross};
use crate::EngineError;
use serde_json::Value;

// ---------------------------------------------------------------------
// Procedures (spec §8.1): `export_r_procedure` and shared dataset
// drivers.

/// `exposure`/`outcome` accept the corpus's bare string and the app's
/// `{name, levels}` spec.
pub(crate) fn var_spec(v: &Value) -> Result<VarSpec, EngineError> {
    match v {
        Value::String(name) => Ok(VarSpec {
            name: name.clone(),
            levels: None,
        }),
        Value::Object(_) => serde_json::from_value(v.clone()).map_err(|e| invalid(e.to_string())),
        _ => Err(invalid("variable spec is not a name or {name, levels}")),
    }
}

/// A `Key` as an R literal in the column's own type.
fn key_lit(k: &categories::Key, data: &crate::dataset::ColumnData) -> String {
    use crate::dataset::ColumnData as CD;
    match (k, data) {
        (categories::Key::I(v), _) => v.to_string(),
        (categories::Key::R(v), _) => lit_num(f64::from_bits(*v)),
        (categories::Key::T(s), _) => lit_str(s),
        (categories::Key::B(b), _) => if *b { "TRUE" } else { "FALSE" }.into(),
        (categories::Key::D(d), CD::Date(_)) => {
            format!("as.Date({})", lit_str(&super::csv::iso_date(*d)))
        }
        _ => "NA".into(),
    }
}

/// The level vector for `var` as the R expression — explicit levels
/// as literals, otherwise the orientation the engine applied. `is2x2`
/// is the table shape `cross` produced (orientation is a 2×2 concern).
fn levels_lit(
    ds: &Dataset,
    spec: &VarSpec,
    is2x2: bool,
    include_missing: bool,
) -> Result<String, EngineError> {
    let data = &ds.column(&spec.name)?.data;
    let (mut keys, miss) = categories::resolved(&spec.name, data, spec.levels.as_ref())?;
    if is2x2 && spec.levels.is_none() {
        keys = categories::orient(&keys, data, &spec.name)?;
    }
    let mut parts: Vec<String> = keys.iter().map(|k| key_lit(k, data)).collect();
    if include_missing && miss > 0 {
        parts.push("NA".into());
    }
    Ok(format!("c({})", parts.join(", ")))
}

/// The stratify columns' level vectors: `list(c(…), c(…))`.
fn strata_levels(
    ds: &Dataset,
    stratify_by: &[String],
    include_missing: bool,
) -> Result<String, EngineError> {
    let mut lists = Vec::new();
    for name in stratify_by {
        let data = &ds.column(name)?.data;
        let (keys, miss) = categories::collect(data);
        let mut parts: Vec<String> = keys.iter().map(|k| key_lit(k, data)).collect();
        if include_missing && miss > 0 {
            parts.push("NA".into());
        }
        lists.push(format!("c({})", parts.join(", ")));
    }
    Ok(format!("list({})", lists.join(", ")))
}

/// The `tables` driver: DATASETS env + XTAB helpers + the write call
/// with a `stopifnot` against the engine's own counts. `declared` is
/// what `cross` (or the corpus fixture) says the tables are — the R
/// rebuild must reproduce it bit-for-bit.
#[allow(clippy::too_many_arguments)]
pub(crate) fn tables_driver(
    call_id: &str,
    ds_id: &str,
    ds: &Dataset,
    markers: &[String],
    exposure: &VarSpec,
    outcome: &VarSpec,
    stratify_by: &[String],
    include_missing: bool,
) -> Result<(Vec<&'static str>, String), EngineError> {
    let xt = cross(ds, exposure, outcome, stratify_by, include_missing)?;
    let is_2x2 = xt.row_labels.len() == 2 && xt.col_labels.len() == 2;
    let mut cols: Vec<(&str, bool)> = vec![(&exposure.name, false), (&outcome.name, false)];
    for s in stratify_by {
        let data = &ds.column(s)?.data;
        cols.push((s, matches!(data, crate::dataset::ColumnData::Boolean(_))));
    }
    let mut body = String::new();
    datasets_env(&mut body, ds_id, ds, markers, &cols)?;
    body.push_str(XTAB_PRELUDE);
    let env = |v: &str| format!("DS${v}$values");
    body.push_str(&format!(
        "DS <- get({ds}, envir = DATASETS)\n",
        ds = lit_str(ds_id)
    ));
    let e = env(&exposure.name);
    let o = env(&outcome.name);
    if is_2x2 && stratify_by.is_empty() {
        let el = levels_lit(ds, exposure, true, include_missing)?;
        let ol = levels_lit(ds, outcome, true, include_missing)?;
        body.push_str(&format!("tab <- epi_xtab({e}, {o}, {el}, {ol})\n"));
        let t = &xt.counts[0];
        let lit = format!(
            "matrix(c({}L, {}L, {}L, {}L), nrow = 2, byrow = TRUE)",
            t[0][0], t[0][1], t[1][0], t[1][1]
        );
        body.push_str(&format!("stopifnot(identical(tab, {lit}))\n"));
        body.push_str(&format!("twobytwo_write({}, tab)\n", lit_str(call_id)));
        Ok((vec!["twobytwo_write"], body))
    } else if is_2x2 {
        let el = levels_lit(ds, exposure, true, include_missing)?;
        let ol = levels_lit(ds, outcome, true, include_missing)?;
        let slv = strata_levels(ds, stratify_by, include_missing)?;
        let sv: Vec<String> = stratify_by.iter().map(|s| env(s)).collect();
        body.push_str(&format!(
            "strata <- epi_strata(list({}), {slv}, {e}, {o}, {el}, {ol})\n",
            sv.join(", ")
        ));
        let lits: Vec<String> = xt
            .counts
            .iter()
            .filter(|t| t.len() >= 2)
            .map(|t| {
                format!(
                    "matrix(c({}L, {}L, {}L, {}L), nrow = 2, byrow = TRUE)",
                    t[0][0], t[0][1], t[1][0], t[1][1]
                )
            })
            .collect();
        body.push_str(&format!(
            "stopifnot(identical(strata, list({})))\n",
            lits.join(", ")
        ));
        let labels = format!(
            "c({})",
            xt.strata_labels
                .iter()
                .map(|l| lit_str(l))
                .collect::<Vec<_>>()
                .join(", ")
        );
        body.push_str(&format!(
            "stratified_write({}, strata, {})\n",
            lit_str(call_id),
            labels
        ));
        Ok((vec!["stratified_write"], body))
    } else {
        if !stratify_by.is_empty() {
            return Err(invalid("stratify_by requires a two-by-two table"));
        }
        let el = levels_lit(ds, exposure, false, include_missing)?;
        let ol = levels_lit(ds, outcome, false, include_missing)?;
        if include_missing {
            // NA lands in the factor's NA level — last, as the engine
            // appends "(missing)".
            body.push_str(&format!(
                "tab <- table(factor({e}, levels = {el}, exclude = NULL),\n\
                 \x20           factor({o}, levels = {ol}, exclude = NULL),\n\
                 \x20           useNA = \"ifany\")\n"
            ));
        } else {
            body.push_str(&format!(
                "keep <- !is.na({e}) & !is.na({o})\n\
                 tab <- table(factor({e}[keep], levels = {el}),\n\
                 \x20           factor({o}[keep], levels = {ol}))\n"
            ));
        }
        let flat: Vec<String> = xt.counts[0]
            .iter()
            .flat_map(|r| r.iter().map(|c| format!("{c}L")))
            .collect();
        let lit = format!(
            "matrix(c({}), nrow = {}, byrow = TRUE)",
            flat.join(", "),
            xt.counts[0].len()
        );
        body.push_str(&format!(
            "stopifnot(identical(unname(matrix(as.integer(tab), nrow = nrow(tab))), {lit}))\n"
        ));
        body.push_str(&format!("rxc_write({}, tab)\n", lit_str(call_id)));
        Ok((vec!["rxc_write"], body))
    }
}

/// The `freq` driver: one `freq_expected` per variable (per stratum
/// when `stratify_by` is set), walked under `tables[i]` — the app
/// result's own paths.
#[allow(clippy::too_many_arguments)]
pub(crate) fn freq_driver(
    call_id: &str,
    ds_id: &str,
    ds: &Dataset,
    markers: &[String],
    variables: &[String],
    stratify_by: Option<&str>,
    include_missing: bool,
) -> Result<(Vec<&'static str>, String), EngineError> {
    let mut cols: Vec<(&str, bool)> = variables.iter().map(|v| (v.as_str(), false)).collect();
    if let Some(s) = stratify_by {
        let data = &ds.column(s)?.data;
        cols.push((s, matches!(data, crate::dataset::ColumnData::Boolean(_))));
    }
    let mut body = String::new();
    datasets_env(&mut body, ds_id, ds, markers, &cols)?;
    body.push_str(&format!(
        "DS <- get({}, envir = DATASETS)\n",
        lit_str(ds_id)
    ));
    let env = |v: &str| format!("DS${v}$values");
    match stratify_by {
        None => {
            for (i, v) in variables.iter().enumerate() {
                let data = &ds.column(v)?.data;
                let (keys, _miss) = categories::collect(data);
                let labels: Vec<String> = keys.iter().map(|k| key_lit(k, data)).collect();
                let inc = if include_missing { "TRUE" } else { "FALSE" };
                body.push_str(&format!(
                    "epi_walk(freq_expected({}, c({}), {inc}), \"tables[{i}]\")\n",
                    env(v),
                    labels.join(", ")
                ));
            }
        }
        Some(s) => {
            let sdata = &ds.column(s)?.data;
            let (skeys, smiss) = categories::collect(sdata);
            let mut sl: Vec<String> = skeys.iter().map(|k| key_lit(k, sdata)).collect();
            if include_missing && smiss > 0 {
                sl.push("NA".into());
            }
            let n_strata = sl.len();
            let inc = if include_missing { "TRUE" } else { "FALSE" };
            body.push_str(&format!("sv <- {}\nsl <- c({})\n", env(s), sl.join(", ")));
            body.push_str(
                "epi_freq_strata <- function(x, labels, inc, force, base) {\n\
                \x20 i <- 0\n\
                \x20 for (lv in sl) {\n\
                \x20   sel <- if (is.na(lv)) is.na(sv) else (!is.na(sv) & sv == lv)\n\
                \x20   xs <- x[sel]\n\
                \x20   e <- freq_expected(xs, labels, inc)\n\
                \x20   if (force && !(inc && sum(is.na(xs)) > 0)) {\n\
                \x20     e$categories[[length(e$categories) + 1]] <-\n\
                \x20       freq_category(0, length(xs), length(xs) - sum(is.na(xs)))\n\
                \x20   }\n\
                \x20   epi_walk(e, paste0(\"tables[\", base + i, \"]\"))\n\
                \x20   i <- i + 1\n\
                \x20 }\n}\n",
            );
            for (vi, v) in variables.iter().enumerate() {
                let data = &ds.column(v)?.data;
                let (keys, vmiss) = categories::collect(data);
                let labels: Vec<String> = keys.iter().map(|k| key_lit(k, data)).collect();
                let force = include_missing && vmiss > 0;
                body.push_str(&format!(
                    "epi_freq_strata({x}, c({labs}), {inc}, {force}, {base})\n",
                    x = env(v),
                    labs = labels.join(", "),
                    force = if force { "TRUE" } else { "FALSE" },
                    base = vi * n_strata,
                ));
            }
        }
    }
    let _ = call_id;
    Ok((vec!["freq_expected"], body))
}

/// The `means` driver — verbatim `means_write` against DATASETS.
pub(crate) fn means_driver(
    call_id: &str,
    ds_id: &str,
    ds: &Dataset,
    markers: &[String],
    variable: &str,
    group_by: Option<&str>,
    include_missing: bool,
) -> Result<(Vec<&'static str>, String), EngineError> {
    let mut cols: Vec<(&str, bool)> = vec![(variable, false)];
    if let Some(g) = group_by {
        let data = &ds.column(g)?.data;
        cols.push((g, matches!(data, crate::dataset::ColumnData::Boolean(_))));
    }
    let mut body = String::new();
    datasets_env(&mut body, ds_id, ds, markers, &cols)?;
    let gb = group_by
        .map(|g| format!(", group_by = {}", lit_str(g)))
        .unwrap_or_default();
    body.push_str(&format!(
        "means_write({}, {}, {}, {}, include_missing = {})\n",
        lit_str(call_id),
        lit_str(ds_id),
        lit_str(variable),
        if gb.is_empty() {
            "NULL".to_string()
        } else {
            gb.trim_start_matches(", group_by = ").to_string()
        },
        if include_missing { "TRUE" } else { "FALSE" }
    ));
    Ok((vec!["means_write"], body))
}
