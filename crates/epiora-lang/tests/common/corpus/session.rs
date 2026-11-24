//! The corpus-runner session: current view + loaded tables.

use super::fixture::load_table;
use epiora_core::dataset::Dataset;
use epiora_lang::{replay, AsOf, Command, Recipe, WorkingView};
use epiora_mdb::Mdb;
use std::collections::BTreeMap;

struct MapLinks<'a>(&'a BTreeMap<String, Dataset>);

impl epiora_lang::LinkedDatasets for MapLinks<'_> {
    fn get(&self, name: &str) -> Option<&Dataset> {
        self.0
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    }
    fn link(&self, _p: &str, _c: &str) -> Option<&epiora_core::dataset::DatasetLink> {
        None
    }
}

fn as_of() -> AsOf {
    AsOf {
        date: 20_000,
        seconds: 0,
    }
}

fn do_replay(
    source: &Dataset,
    tables: &BTreeMap<String, Dataset>,
    steps: &[Command],
) -> WorkingView {
    replay(
        source,
        &MapLinks(tables),
        &Recipe {
            as_of: as_of(),
            steps: steps.to_vec(),
        },
    )
    .expect("replay")
}

pub struct Session<'m> {
    pub mdbs: &'m BTreeMap<String, Mdb>,
    /// lower-case name → current table contents (MERGE writes its
    /// destination back, as EI7's merge writes the file).
    pub tables: BTreeMap<String, Dataset>,
    /// The READ table backing the view.
    pub source: Dataset,
    /// Cleaning commands since the last READ.
    pub steps: Vec<Command>,
    pub view: WorkingView,
}

impl<'m> Session<'m> {
    pub fn open(mdbs: &'m BTreeMap<String, Mdb>, file: &str, table: &str) -> Session<'m> {
        let mut tables = BTreeMap::new();
        let source = load_table(mdbs, file, table);
        tables.insert(table.to_ascii_lowercase(), source.clone());
        let view = do_replay(&source, &tables, &[]);
        Session {
            mdbs,
            tables,
            source,
            steps: vec![],
            view,
        }
    }

    /// Ensures `table` (from `file`) is loaded; MERGE destinations are
    /// already there.
    pub fn table(&mut self, file: &str, name: &str) {
        let key = name.to_ascii_lowercase();
        if !self.tables.contains_key(&key) {
            let ds = load_table(self.mdbs, file, name);
            self.tables.insert(key, ds);
        }
    }

    pub fn read(&mut self, file: &str, table: &str) {
        self.table(file, table);
        self.source = self.tables[&table.to_ascii_lowercase()].clone();
        self.steps.clear();
        self.view = do_replay(&self.source, &self.tables, &[]);
    }

    pub fn command(&mut self, cmd: Command) {
        self.steps.push(cmd);
        self.view = do_replay(&self.source, &self.tables, &self.steps);
    }

    pub fn count(&self) -> usize {
        self.view.materialize().row_count()
    }
}
