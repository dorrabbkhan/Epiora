//! The working view (v1.1 spec §5.4, plan Task 11): the in-memory
//! image of the READ table that the cleaning commands mutate. The
//! view holds the full table (defined columns and MERGE/RELATE results
//! included), a SELECT row-index set, a SORT permutation and the SET
//! display/process options; `materialize` applies them in EI7's order.
//!
//! Deletion follows EI7's two paths: on a table carrying a RecStatus
//! column (`__recstatus`, 1 = normal / 0 = deleted) DELETE writes
//! marks and PROCESS decides whether they show, while a raw import
//! without that column loses the rows entirely — `UNDELETE` can only
//! ever restore marked rows (the ei7_v11 corpus pins `0` restored for
//! a raw table).

use epiora_core::dataset::{ColumnData, Dataset};
use serde::{Deserialize, Serialize};

/// `SET PROCESS=` — which records the view includes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Process {
    /// Undeleted records only (EI7's default).
    #[default]
    Undeleted,
    /// Both undeleted and deleted records.
    Both,
    /// Only deleted records.
    Deleted,
}

/// `SET STATISTICS=` — the analysis verbosity EI7 carries per view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatisticsLevel {
    /// Suppress all statistics.
    None,
    /// Counts only.
    Minimal,
    /// Counts and basic statistics.
    Intermediate,
    /// Everything EI7 computes (the default).
    #[default]
    Complete,
}

/// The SET options the view carries (spec §5.4; grammar's
/// `SetClause`). Display-only options are stored, not applied, by the
/// view itself — `materialize` reacts to `process` and the corpus
/// runner reads `include_missing` for FREQ.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewSettings {
    /// `SET PROCESS=`.
    pub process: Process,
    /// `SET MISSING=ON|OFF` — FREQ gains the missing category.
    pub include_missing: bool,
    /// `SET DELETED=ON|OFF` — display deleted-mark flag.
    pub deleted: bool,
    /// `SET STATISTICS=`.
    pub statistics: StatisticsLevel,
    /// `SET PERCENTS=ON|OFF`.
    pub percents: bool,
    /// `SET IGNORE=ON|OFF` — ignore-case text comparisons flag.
    pub ignore: bool,
    /// `SET SELECT=ON|OFF` — selectable-grid flag.
    pub select: bool,
    /// Rows a raw-table DELETE removed in this session; the marks a
    /// RecStatus table would have carried. Drives
    /// [`WorkingView::deleted_records_included`].
    pub(crate) physically_deleted: u64,
}

impl Default for ViewSettings {
    fn default() -> Self {
        ViewSettings {
            process: Process::Undeleted,
            include_missing: false,
            deleted: false,
            statistics: StatisticsLevel::Complete,
            percents: true,
            ignore: false,
            select: true,
            physically_deleted: 0,
        }
    }
}

/// A DEFINE'd variable's bookkeeping — whether a TEXTINPUT
/// declaration uppercases ASSIGN writes.
#[derive(Clone, Debug)]
pub(crate) struct DefinedVar {
    /// `DEFINE x TEXTINPUT` — text assigns arrive uppercased.
    pub text_input: bool,
}

/// `MERGE`/`RELATE` join modes; `MATCHING` is the unmarked form.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JoinMode {
    /// Keep only destination rows with at least one match.
    #[default]
    Matching,
    /// Keep every destination row; unmatched carry missing children.
    All,
}

/// A dataset reference in commands: the name and, for RELATE, the
/// `ALL` modifier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetRef {
    /// Table name as the command spelled it.
    pub name: String,
    /// RELATE's join mode.
    pub join: JoinMode,
}

impl DatasetRef {
    /// A bare name with the default (matching) join.
    pub fn named(name: &str) -> DatasetRef {
        DatasetRef {
            name: name.to_string(),
            join: JoinMode::Matching,
        }
    }
}

/// SORT direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    /// Ascending — missing cells first (ei7_v11 `SORT TIMESUPPER`).
    Asc,
    /// Descending — missing cells last.
    Desc,
}

/// The session's linked datasets — supplied by the worker, which owns
/// the project tables and their stored links.
pub trait LinkedDatasets {
    /// The current contents of dataset `name` (case-insensitive).
    fn get(&self, name: &str) -> Option<&Dataset>;
    /// The stored `Relate`/`Grid` link parent→child, when the project
    /// knows one — used by a keyless `RELATE`.
    fn link(&self, parent: &str, child: &str) -> Option<&epiora_core::dataset::DatasetLink>;
}

/// The working view a recipe replays into (spec §5.4; the Interfaces
/// block's `{dataset, selected, order, settings}`).
#[derive(Clone, Debug)]
pub struct WorkingView {
    /// The current table: source rows plus every DEFINE/ASSIGN/
    /// RECODE/MERGE/RELATE effect. Raw-table DELETEs shrink it;
    /// RecStatus-table DELETEs mark inside it.
    pub dataset: Dataset,
    /// Row indices passing every SELECT so far (AND-combined), or
    /// `None` when nothing is selected.
    pub selected: Option<Vec<u32>>,
    /// The SORT permutation over all dataset rows, or `None` when the
    /// view is in file order.
    pub order: Option<Vec<u32>>,
    /// The SET options.
    pub settings: ViewSettings,
    /// DEFINE bookkeeping by (lower-cased) variable name.
    pub(crate) defined: std::collections::BTreeMap<String, DefinedVar>,
    /// `KEYVARS` — the view's declared key columns.
    pub key_vars: Vec<String>,
}

impl WorkingView {
    /// The view over `source` before any command ran.
    pub(crate) fn new(source: Dataset) -> WorkingView {
        WorkingView {
            dataset: source,
            selected: None,
            order: None,
            settings: ViewSettings::default(),
            defined: std::collections::BTreeMap::new(),
            key_vars: vec![],
        }
    }

    /// The index of the RecStatus column (`__recstatus`, legacy type
    /// `RecStatus`), when the table carries one.
    pub(crate) fn recstatus_col(&self) -> Option<usize> {
        self.dataset
            .columns()
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case("__recstatus"))
            .filter(|&i| {
                self.dataset
                    .meta()
                    .get(i)
                    .and_then(|m| m.legacy_type.as_deref())
                    .is_some_and(|t| t.eq_ignore_ascii_case("RecStatus"))
            })
    }

    /// `true` for rows marked deleted (RecStatus 0).
    pub(crate) fn marked_deleted(&self, row: usize) -> bool {
        match self.recstatus_col() {
            Some(i) => matches!(
                &self.dataset.columns()[i].data,
                ColumnData::Integer(v) if v[row] == Some(0)
            ),
            None => false,
        }
    }

    /// EI7's `Deleted Records Included` flag: `true` while the view
    /// excludes no deleted records — trivially true before any
    /// session delete, false once a raw-table DELETE removed rows or
    /// marks exist under `PROCESS=UNDELETED`, true again when marks
    /// are shown (`BOTH`/`DELETED`) or cleared by `UNDELETE`.
    pub fn deleted_records_included(&self) -> bool {
        if self.settings.physically_deleted > 0 {
            return false;
        }
        match (self.recstatus_col(), self.settings.process) {
            (Some(_), Process::Undeleted) => {
                !(0..self.dataset.row_count()).any(|r| self.marked_deleted(r))
            }
            _ => true,
        }
    }

    /// SELECT, PROCESS and SORT applied: the rows output commands see,
    /// in view order (spec §5.4 — "the hash of the working view" is
    /// this dataset's content hash).
    pub fn materialize(&self) -> Dataset {
        let n = self.dataset.row_count();
        let mut idx: Vec<u32> = match &self.order {
            Some(o) => o.clone(),
            None => (0..n as u32).collect(),
        };
        if let Some(sel) = &self.selected {
            let member: std::collections::HashSet<u32> = sel.iter().copied().collect();
            idx.retain(|i| member.contains(i));
        }
        match self.settings.process {
            Process::Both => {}
            p => {
                let want_deleted = p == Process::Deleted;
                idx.retain(|&i| self.marked_deleted(i as usize) == want_deleted);
            }
        }
        let cols = self
            .dataset
            .columns()
            .iter()
            .map(|c| epiora_core::dataset::Column {
                name: c.name.clone(),
                data: gather(&c.data, &idx),
            })
            .collect();
        let mut out = match Dataset::new(self.dataset.id(), self.dataset.version(), cols) {
            Ok(d) => d,
            Err(_) => return self.dataset.clone(),
        };
        if !self.dataset.meta().is_empty() {
            if let Ok(d) = out.clone().with_meta(
                self.dataset.meta().to_vec(),
                self.dataset.label_sets().clone(),
            ) {
                out = d;
            }
        }
        out
    }
}

/// Gathers the `idx` rows of a column, preserving its type.
pub(crate) fn gather(data: &ColumnData, idx: &[u32]) -> ColumnData {
    match data {
        ColumnData::Integer(v) => ColumnData::Integer(idx.iter().map(|&i| v[i as usize]).collect()),
        ColumnData::Real(v) => ColumnData::Real(idx.iter().map(|&i| v[i as usize]).collect()),
        ColumnData::Text(v) => {
            ColumnData::Text(idx.iter().map(|&i| v[i as usize].clone()).collect())
        }
        ColumnData::Boolean(v) => ColumnData::Boolean(idx.iter().map(|&i| v[i as usize]).collect()),
        ColumnData::Date(v) => ColumnData::Date(idx.iter().map(|&i| v[i as usize]).collect()),
        ColumnData::Time(v) => ColumnData::Time(idx.iter().map(|&i| v[i as usize]).collect()),
        ColumnData::DateTime(v) => {
            ColumnData::DateTime(idx.iter().map(|&i| v[i as usize]).collect())
        }
    }
}
