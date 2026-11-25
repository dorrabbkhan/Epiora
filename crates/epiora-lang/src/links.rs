//! Stored `dataset_links` (v1.1 plan Task 13b, spec §4.2): the
//! [`LinkedDatasets`] implementation a session's replay uses — the
//! project's linked tables plus its stored link rows, so a keyless
//! `RELATE` finds the key pair the store knows (`relate_no_link`
//! when it does not).

use crate::view::LinkedDatasets;
use epiora_core::dataset::{Dataset, DatasetLink};
use std::collections::BTreeMap;

/// The session's linked tables (keys lower-cased) plus the project's
/// stored `dataset_links` rows.
pub struct StoredLinks {
    tables: BTreeMap<String, Dataset>,
    links: Vec<DatasetLink>,
}

impl StoredLinks {
    /// `tables` maps dataset name → decoded snapshot (keys are
    /// lower-cased here); `links` are the project's stored
    /// `dataset_links` rows, whose `parent`/`child` name the same
    /// datasets.
    pub fn new(tables: BTreeMap<String, Dataset>, links: Vec<DatasetLink>) -> StoredLinks {
        StoredLinks {
            tables: tables
                .into_iter()
                .map(|(k, v)| (k.to_ascii_lowercase(), v))
                .collect(),
            links,
        }
    }
}

impl LinkedDatasets for StoredLinks {
    fn get(&self, name: &str) -> Option<&Dataset> {
        self.tables.get(&name.to_ascii_lowercase())
    }

    /// The stored link parent→child, matched case-insensitively. A
    /// replay source carries a placeholder id rather than its store
    /// name (the wasm boundary decodes it as `source`), so `parent`
    /// also matches a link whose `parent` names no linked table —
    /// the session's source is the one project dataset absent from
    /// `tables`.
    fn link(&self, parent: &str, child: &str) -> Option<&DatasetLink> {
        self.links.iter().find(|l| {
            l.child.eq_ignore_ascii_case(child)
                && (l.parent.eq_ignore_ascii_case(parent)
                    || !self
                        .tables
                        .keys()
                        .any(|k| k.eq_ignore_ascii_case(&l.parent)))
        })
    }
}
