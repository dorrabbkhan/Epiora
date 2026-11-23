//! `corpus_datasets` (`corpus` feature) — the built-in dataset
//! cases `export-corpus` emits alongside the fixture corpus: the
//! `hostile_text` dataset (Review Focus 4) and the dataset-echo
//! `analysis.R` that prints `EPIORA value|missing <col>[<row>]` per
//! cell for `compare.mjs` to check exactly.

use super::csv::{dataset_csv, missing_markers};
use super::r_emit::{header, lit_str, PRELUDE, READER_PRELUDE};
use crate::dataset::{Column, ColumnData, Dataset};
use crate::EngineError;
use serde_json::{Map, Value};

/// Days-from-civil for building datasets in code (Hinnant).
fn days_from_civil(y: i64, m: i64, d: i64) -> i32 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe - 719468) as i32
}

/// The built-in hostile-text dataset (Review Focus 4): commas,
/// embedded quotes and newlines, `é`, `日本語`, literal `"NA"`, an
/// empty string next to a real `None`, `∅` forcing a two-`∅` marker,
/// and pre-epoch + post-epoch dates.
pub fn hostile_text_dataset() -> Result<Dataset, EngineError> {
    let col = |name: &str, data: ColumnData| Column {
        name: name.to_string(),
        data,
    };
    Dataset::new(
        "hostile_text",
        1,
        vec![
            col(
                "count",
                ColumnData::Integer(vec![
                    Some(1),
                    None,
                    Some(-7),
                    Some(0),
                    Some(5),
                    Some(6),
                    Some(7),
                    Some(8),
                    Some(9),
                ]),
            ),
            col(
                "real",
                ColumnData::Real(vec![
                    Some(0.1),
                    None,
                    Some(-1.5e300),
                    Some(2.5),
                    Some(4.0),
                    Some(5.0),
                    Some(6.0),
                    Some(7.0),
                    Some(8.0),
                ]),
            ),
            col(
                "text",
                ColumnData::Text(vec![
                    Some("a,b".into()),
                    Some("say \"hi\"".into()),
                    Some("line\nbreak".into()),
                    Some("café".into()),
                    Some("日本語".into()),
                    Some("NA".into()),
                    Some("".into()),
                    None,
                    Some("∅".into()),
                ]),
            ),
            col(
                "flag",
                ColumnData::Boolean(vec![
                    Some(true),
                    None,
                    Some(false),
                    Some(true),
                    Some(false),
                    Some(true),
                    Some(false),
                    Some(true),
                    Some(false),
                ]),
            ),
            col(
                "date",
                ColumnData::Date(vec![
                    Some(days_from_civil(1950, 6, 1)),
                    None,
                    Some(days_from_civil(2026, 11, 12)),
                    Some(0),
                    Some(1),
                    Some(-1),
                    Some(100),
                    Some(200),
                    Some(300),
                ]),
            ),
        ],
    )
}

/// The dataset-echo case: `analysis.R` prints one `EPIORA
/// value|missing <col>[<row>]` line per cell — `compare.mjs` checks
/// them exactly against `expected.json`'s `cells` map (Review
/// Focus 4).
pub fn dataset_echo_export(ds: &Dataset) -> (String, Vec<u8>, Map<String, Value>) {
    let markers = missing_markers(ds);
    let mut body = String::from(READER_PRELUDE);
    body.push_str(&format!("stopifnot(nrow(raw) == {})\n\n", ds.row_count()));
    body.push_str(
        r#"epi_json <- function(v) {
  if (inherits(v, "Date")) return(sprintf("\"%s\"", format(v)))
  if (is.character(v)) {
    v <- gsub("\\", "\\\\", v, fixed = TRUE)
    v <- gsub("\"", "\\\"", v, fixed = TRUE)
    v <- gsub("\n", "\\n", v, fixed = TRUE)
    v <- gsub("\r", "\\r", v, fixed = TRUE)
    v <- gsub("\t", "\\t", v, fixed = TRUE)
    return(paste0("\"", v, "\""))
  }
  if (is.logical(v)) return(tolower(as.character(v)))
  if (v == floor(v) && abs(v) < 1e15) return(sprintf("%d", as.integer(v)))
  sprintf("%.17g", v)
}
cells <- list(
"#,
    );
    let mut cell_items = Vec::new();
    let mut cells = Map::new();
    for (j, c) in ds.columns().iter().enumerate() {
        cell_items.push(format!(
            "  {} = {}(raw, {}, {})",
            c.name,
            super::r_emit::col_reader(&c.data),
            lit_str(&c.name),
            lit_str(&markers[j])
        ));
        for i in 0..ds.row_count() {
            let key = format!("{}[{i}]", c.name);
            let lit = match &c.data {
                ColumnData::Integer(v) => v[i].map(Value::from),
                ColumnData::Real(v) => v[i].map(Value::from),
                ColumnData::Text(v) => v[i].clone().map(Value::from),
                ColumnData::Boolean(v) => v[i].map(Value::from),
                ColumnData::Date(v) => v[i].map(|d| Value::from(super::csv::iso_date(d))),
                // Time/DateTime reach R as their stored seconds — no
                // `epi_time` reader exists and the unit is what MEANS
                // groups and compares (v1.1 spec §4.3).
                ColumnData::Time(v) => v[i].map(Value::from),
                ColumnData::DateTime(v) => v[i].map(Value::from),
            };
            cells.insert(key, lit.unwrap_or(Value::Null));
        }
    }
    body.push_str(&cell_items.join(",\n"));
    body.push_str("\n)\n");
    body.push_str(
        r#"for (nm in names(cells)) {
  for (i in seq_along(cells[[nm]])) {
    v <- cells[[nm]][i]
    if (is.na(v)) {
      cat(sprintf("EPIORA missing %s[%d]\n", nm, i - 1L))
    } else {
      cat(sprintf("EPIORA value %s[%d] %s\n", nm, i - 1L, epi_json(v)))
    }
  }
}
"#,
    );
    let mut script = String::from(PRELUDE);
    script.push('\n');
    script.push_str(&body);
    let head = header(
        "dataset_echo hostile_text",
        &["stats"],
        "corpus:hostile_text",
    );
    (format!("{head}{script}"), dataset_csv(ds), cells)
}
