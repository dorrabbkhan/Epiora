//! The `Project` type (spec §3.2): a name plus references to datasets,
//! so receipts can name a project. No storage — the type only.

use serde::{Deserialize, Serialize};

/// A project: an id, a display name, and the datasets it references.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// Stable project id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// References to the project's datasets.
    pub datasets: Vec<DatasetRef>,
}

/// A pointer to one version of a dataset.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatasetRef {
    /// The dataset's id.
    pub id: String,
    /// The pinned version.
    pub version: u64,
}
