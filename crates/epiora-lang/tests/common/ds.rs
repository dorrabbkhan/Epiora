//! Shared builders for the manage/recipe/receipt tests: datasets,
//! expressions, recipes, linked tables, cell assertions and the
//! no-stamp `RunContext`.

use epiora_core::dataset::{Column, ColumnData, Dataset, DatasetLink, Value};
use epiora_core::procedures::RunContext;
use epiora_lang::{parse_expr, replay, AsOf, Command, LinkedDatasets, Recipe, WorkingView};
use std::collections::BTreeMap;

pub fn col(name: &str, data: ColumnData) -> Column {
    Column {
        name: name.to_string(),
        data,
    }
}

pub fn ints(v: &[i64]) -> ColumnData {
    ColumnData::Integer(v.iter().map(|x| Some(*x)).collect())
}

pub fn opt_ints(v: &[Option<i64>]) -> ColumnData {
    ColumnData::Integer(v.to_vec())
}

pub fn reals(v: &[f64]) -> ColumnData {
    ColumnData::Real(v.iter().map(|x| Some(*x)).collect())
}

pub fn texts(v: &[&str]) -> ColumnData {
    ColumnData::Text(v.iter().map(|s| Some((*s).to_string())).collect())
}

pub fn ds(id: &str, cols: Vec<(&str, ColumnData)>) -> Dataset {
    Dataset::new(id, 1, cols.into_iter().map(|(n, d)| col(n, d)).collect()).expect("dataset")
}

pub fn expr(text: &str) -> epiora_lang::Expr {
    parse_expr(text).expect("expression parses")
}

pub fn as_of() -> AsOf {
    AsOf {
        date: 20_000,
        seconds: 0,
    }
}

pub fn recipe(steps: Vec<Command>) -> Recipe {
    Recipe {
        as_of: as_of(),
        steps,
    }
}

#[derive(Default)]
pub struct Links {
    pub tables: BTreeMap<String, Dataset>,
    pub links: Vec<DatasetLink>,
}

impl LinkedDatasets for Links {
    fn get(&self, name: &str) -> Option<&Dataset> {
        self.tables
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    }

    fn link(&self, parent: &str, child: &str) -> Option<&DatasetLink> {
        self.links
            .iter()
            .find(|l| l.parent.eq_ignore_ascii_case(parent) && l.child.eq_ignore_ascii_case(child))
    }
}

pub struct NoLinks;
impl LinkedDatasets for NoLinks {
    fn get(&self, _name: &str) -> Option<&Dataset> {
        None
    }
    fn link(&self, _parent: &str, _child: &str) -> Option<&epiora_core::dataset::DatasetLink> {
        None
    }
}

pub fn replay_one(src: &Dataset, steps: Vec<Command>) -> WorkingView {
    replay(src, &Links::default(), &recipe(steps)).expect("replay")
}

/// A column's cells as `Option<Value>` — the assertion form of every
/// test below.
pub fn cells(ds: &Dataset, name: &str) -> Vec<Option<Value>> {
    let c = ds.column(name).unwrap_or_else(|_| panic!("column {name}"));
    match &c.data {
        ColumnData::Integer(v) => v.iter().map(|x| x.map(Value::Integer)).collect(),
        ColumnData::Real(v) => v.iter().map(|x| x.map(Value::Real)).collect(),
        ColumnData::Text(v) => v.iter().map(|x| x.clone().map(Value::Text)).collect(),
        ColumnData::Boolean(v) => v.iter().map(|x| x.map(Value::Boolean)).collect(),
        ColumnData::Date(v) => v.iter().map(|x| x.map(Value::Date)).collect(),
        ColumnData::Time(v) => v.iter().map(|x| x.map(Value::Time)).collect(),
        ColumnData::DateTime(v) => v.iter().map(|x| x.map(Value::DateTime)).collect(),
    }
}

pub fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-11-23T00:00:00Z".into(),
        project: None,
        working_view: None,
    }
}
