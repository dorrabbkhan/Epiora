//! Per-column metadata and dataset links (v1.1 spec §4.3). Meta lives
//! beside the value data — a prompt (the form's question text), the
//! legacy field type and name the mapper saw, and the column's
//! value-label set id. Only `label_set` enters the dataset hash (labels
//! change what results display); prompts and legacy names never do.
//! Links relate datasets inside a project; they are stored per project
//! in the store's `dataset_links` table, never inside `Dataset`.

use serde::{Deserialize, Serialize};

/// The non-value information a legacy import keeps for one column.
/// Every field is optional; an all-`None` meta is the default a plain
/// `Dataset::new` column carries.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ColumnMeta {
    /// The question text the legacy form showed for the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// The legacy field type (Epi Info's type name), for the report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_type: Option<String>,
    /// The field's name in the legacy file, when it differs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_name: Option<String>,
    /// The id of the `LabelSet` whose values label this column.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_set: Option<String>,
}

/// A link between two datasets of one project: `child_key` on the
/// child dataset points at `parent_key` on the parent (Epi Info's
/// `FKEY` → `GlobalRecordId`). Stored in the project, never hashed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DatasetLink {
    /// The parent dataset's name.
    pub parent: String,
    /// The child dataset's name.
    pub child: String,
    /// The child's foreign-key column.
    pub child_key: String,
    /// The parent's key column.
    pub parent_key: String,
    /// The kind of link the legacy project declared.
    pub kind: LinkKind,
}

/// How two datasets are linked (v1.1 spec §4.2): an Epi Info Relate
/// field (one child row's `FKEY` points at a parent record) or a grid
/// (a child form page's rows belong to a parent record).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum LinkKind {
    /// An Epi Info Relate field.
    Relate,
    /// An Epi Info grid page.
    Grid,
}
