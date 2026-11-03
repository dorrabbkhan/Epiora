//! `tabulate` — `Dataset` → count tables (spec §4.2). This is the only
//! code on the 1M-row hot path: one pass over the rows maps every used
//! variable's values to category indices (a dictionary per column) and
//! accumulates `u64` counts.
//!
//! Missing handling (spec §4.1): a record missing any used variable is
//! excluded and counted per variable in `exclusions`;
//! `include_missing = true` instead adds a `"(missing)"` category —
//! last in every label list.
//!
//! Category order and 2×2 orientation follow spec §3.3 (Boolean Yes,
//! No; Integer/Real/Date ascending; Text ascending UTF-8 byte order;
//! Yes/1 first in a 2×2; an unorientable two-level variable without
//! `levels` is `AmbiguousLevels`, never a guess).

mod categories;

use crate::dataset::{ColumnData, Dataset};
use crate::procedures::VarSpec;
use crate::EngineError;
use categories::{collect, orient, resolved, Cats};
use std::collections::BTreeMap;

/// Sparse-stratum fallback bound: a dense `strata × rows × cols` cell
/// vector is allocated only below this many cells.
const MAX_DENSE_CELLS: usize = 1_048_576;

/// A per-category count vector with its label list, as `frequency`
/// returns it.
#[derive(Debug, Clone, PartialEq)]
pub struct FreqCounts {
    /// Category labels; `"(missing)"` last when present.
    pub labels: Vec<String>,
    /// Count per category, aligned with `labels`.
    pub counts: Vec<u64>,
    /// Records with a missing value (also counted in the `"(missing)"`
    /// category when `include_missing`).
    pub missing: u64,
    /// Records in scope (dataset rows, or rows of the stratum).
    pub total: u64,
}

/// Per-stratum `FreqCounts` of one variable.
#[derive(Debug, Clone, PartialEq)]
pub struct StratifiedFreq {
    /// Stratum labels in category order.
    pub strata_labels: Vec<String>,
    /// One table per stratum, aligned with `strata_labels`.
    pub tables: Vec<FreqCounts>,
    /// Excluded records per used variable (the stratify variables;
    /// records missing the analysed variable are reported in
    /// `FreqCounts::missing`, not excluded).
    pub exclusions: BTreeMap<String, u64>,
}

/// An R×C count cube per stratum (spec §4.2).
#[derive(Debug, Clone, PartialEq)]
pub struct CrossTab {
    /// Row categories in orientation order (2×2: exposed, unexposed).
    pub row_labels: Vec<String>,
    /// Column categories in orientation order (2×2: ill, well).
    pub col_labels: Vec<String>,
    /// Combined stratum labels in category order.
    pub strata_labels: Vec<String>,
    /// `counts[stratum][row][col]`.
    pub counts: Vec<Vec<Vec<u64>>>,
    /// Excluded records per used variable: `exclusions[v]` counts
    /// excluded records where `v` is missing.
    pub exclusions: BTreeMap<String, u64>,
}

/// `cross` — one pass counting `row × col` per stratum (spec §4.2).
/// `strata` are the stratify variable names; combined strata are
/// ordered by category order. `AmbiguousLevels` fires only when the
/// result would be a 2×2 — orientation is a 2×2 concern (spec §3.3).
pub fn cross(
    ds: &Dataset,
    row: &VarSpec,
    col: &VarSpec,
    strata: &[String],
    include_missing: bool,
) -> Result<CrossTab, EngineError> {
    let rdata = &ds.column(&row.name)?.data;
    let cdata = &ds.column(&col.name)?.data;
    let sdata: Vec<&ColumnData> = strata
        .iter()
        .map(|n| ds.column(n).map(|c| &c.data))
        .collect::<Result<_, _>>()?;

    let (mut rkeys, rmiss) = resolved(&row.name, rdata, row.levels.as_ref())?;
    let (mut ckeys, cmiss) = resolved(&col.name, cdata, col.levels.as_ref())?;
    if rkeys.len() + usize::from(include_missing && rmiss > 0) == 2
        && ckeys.len() + usize::from(include_missing && cmiss > 0) == 2
    {
        if row.levels.is_none() {
            rkeys = orient(&rkeys, rdata, &row.name)?;
        }
        if col.levels.is_none() {
            ckeys = orient(&ckeys, cdata, &col.name)?;
        }
    }
    let rcats = Cats::build(&rkeys, rmiss, rdata, include_missing);
    let ccats = Cats::build(&ckeys, cmiss, cdata, include_missing);
    let scats: Vec<Cats> = sdata
        .iter()
        .map(|d| {
            let (k, m) = collect(d);
            Cats::build(&k, m, d, include_missing)
        })
        .collect();

    let mut exclusions: BTreeMap<String, u64> = [row.name.as_str(), col.name.as_str()]
        .into_iter()
        .chain(strata.iter().map(String::as_str))
        .map(|n| (n.to_string(), 0))
        .collect();

    let (rows, cols) = (rcats.labels.len(), ccats.labels.len());
    let dense_cells = scats
        .iter()
        .map(|c| c.labels.len())
        .try_fold(rows * cols, |p, w| p.checked_mul(w))
        .is_some_and(|p| p <= MAX_DENSE_CELLS);
    let mut dense: Vec<Vec<Vec<u64>>> = if dense_cells {
        // product() is 1 for no stratum variables; a variable with zero
        // categories gives 0 strata — every row is excluded.
        let n = scats.iter().map(|c| c.labels.len()).product::<usize>();
        vec![vec![vec![0; cols]; rows]; n]
    } else {
        Vec::new()
    };
    let mut sparse: BTreeMap<Vec<usize>, Vec<Vec<u64>>> = BTreeMap::new();

    for i in 0..ds.row_count() {
        let mut missing_vars = Vec::new();
        let mut cell = |cats: &Cats, data: &ColumnData, name: &str| -> usize {
            match cats.idx(data, i) {
                Some(x) => x,
                None => match cats.missing_idx {
                    Some(m) => m,
                    None => {
                        missing_vars.push(name.to_string());
                        usize::MAX
                    }
                },
            }
        };
        let ri = cell(&rcats, rdata, &row.name);
        let ci = cell(&ccats, cdata, &col.name);
        let skey: Vec<usize> = scats
            .iter()
            .zip(&sdata)
            .zip(strata)
            .map(|((c, d), n)| cell(c, d, n))
            .collect();
        if !missing_vars.is_empty() {
            for v in missing_vars {
                *exclusions.entry(v).or_default() += 1;
            }
            continue;
        }
        let table = if dense_cells {
            let mut s = 0_usize;
            for (x, c) in skey.iter().zip(&scats) {
                s = s * c.labels.len() + x;
            }
            &mut dense[s]
        } else {
            sparse
                .entry(skey)
                .or_insert_with(|| vec![vec![0; cols]; rows])
        };
        table[ri][ci] = table[ri][ci].saturating_add(1);
    }

    let strata_labels = |combo: &[usize]| -> String {
        combo
            .iter()
            .enumerate()
            .map(|(v, &x)| scats[v].labels[x].as_str())
            .collect::<Vec<_>>()
            .join(",")
    };
    let (strata_labels, counts) = if dense_cells {
        let mut combo = vec![0_usize; scats.len()];
        let labels = (0..dense.len())
            .map(|mut rem| {
                for (v, c) in scats.iter().enumerate().rev() {
                    combo[v] = rem % c.labels.len();
                    rem /= c.labels.len();
                }
                strata_labels(&combo)
            })
            .collect();
        (labels, dense)
    } else {
        let labels = sparse.keys().map(|c| strata_labels(c)).collect();
        (labels, sparse.into_values().collect())
    };
    Ok(CrossTab {
        row_labels: rcats.labels,
        col_labels: ccats.labels,
        strata_labels,
        counts,
        exclusions,
    })
}

/// `frequency` — the one-variable table of `var` (spec §4.2).
/// `include_missing` appends a `"(missing)"` category (last) when the
/// variable has missing values.
pub fn frequency(
    ds: &Dataset,
    var: &str,
    include_missing: bool,
) -> Result<FreqCounts, EngineError> {
    let data = &ds.column(var)?.data;
    let (keys, missing) = collect(data);
    let cats = Cats::build(&keys, missing, data, include_missing);
    let mut counts = vec![0_u64; cats.labels.len()];
    for i in 0..ds.row_count() {
        if let Some(x) = cats.idx(data, i).or(cats.missing_idx) {
            counts[x] = counts[x].saturating_add(1);
        }
    }
    Ok(FreqCounts {
        labels: cats.labels,
        counts,
        missing: cats.missing,
        total: ds.row_count() as u64,
    })
}

/// Per-stratum frequency of `var` — the shared machinery behind
/// `procedures::freq`'s `stratify_by`. Combined strata labels join the
/// per-variable labels with `,`.
pub fn frequency_stratified(
    ds: &Dataset,
    var: &str,
    strata: &[String],
    include_missing: bool,
) -> Result<StratifiedFreq, EngineError> {
    let data = &ds.column(var)?.data;
    let sdata: Vec<_> = strata
        .iter()
        .map(|n| ds.column(n).map(|c| &c.data))
        .collect::<Result<_, _>>()?;
    let (keys, vmissing) = collect(data);
    let cats = Cats::build(&keys, vmissing, data, include_missing);
    let scats: Vec<Cats> = sdata
        .iter()
        .map(|d| {
            let (k, m) = collect(d);
            Cats::build(&k, m, d, include_missing)
        })
        .collect();

    let mut exclusions: BTreeMap<String, u64> = [var]
        .into_iter()
        .chain(strata.iter().map(String::as_str))
        .map(|n| (n.to_string(), 0))
        .collect();
    let mut tables: BTreeMap<Vec<usize>, FreqCounts> = BTreeMap::new();
    for i in 0..ds.row_count() {
        let mut skey = Vec::with_capacity(scats.len());
        let mut dropped: Vec<String> = Vec::new();
        for ((c, d), n) in scats.iter().zip(&sdata).zip(strata) {
            match c.idx(d, i).or(c.missing_idx) {
                Some(x) => skey.push(x),
                None => dropped.push(n.to_string()),
            }
        }
        if !dropped.is_empty() {
            // An excluded record is counted under every variable it is
            // missing (spec §4.1), including the analysed variable.
            if cats.idx(data, i).is_none() {
                *exclusions.entry(var.to_string()).or_default() += 1;
            }
            for v in dropped {
                *exclusions.entry(v).or_default() += 1;
            }
            continue;
        }
        let table = tables.entry(skey).or_insert_with(|| FreqCounts {
            labels: cats.labels.clone(),
            counts: vec![0; cats.labels.len()],
            missing: 0,
            total: 0,
        });
        table.total = table.total.saturating_add(1);
        match cats.idx(data, i) {
            Some(x) => table.counts[x] = table.counts[x].saturating_add(1),
            None => {
                table.missing = table.missing.saturating_add(1);
                if let Some(m) = cats.missing_idx {
                    table.counts[m] = table.counts[m].saturating_add(1);
                }
            }
        }
    }

    let strata_labels = tables
        .keys()
        .map(|combo| {
            combo
                .iter()
                .enumerate()
                .map(|(v, &x)| scats[v].labels[x].as_str())
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect();
    Ok(StratifiedFreq {
        strata_labels,
        tables: tables.into_values().collect(),
        exclusions,
    })
}
