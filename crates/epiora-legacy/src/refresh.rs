//! The re-import plan (v1.1 spec §7.3): `refresh_plan` diffs each
//! remapped view against the committed dataset by the record-key
//! column — `GlobalRecordId` for EI7 — and classifies every view so
//! the caller can show the preview before applying anything
//! (`unchanged` diffs plan nothing).

use std::collections::{HashMap, HashSet};

use epiora_core::dataset::Dataset;
use epiora_import::Diff;
use serde::{Deserialize, Serialize};

use crate::error::LegacyError;
use crate::project::LegacyProject;

/// What a remapped view means for the committed data.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefreshOutcome {
    /// The re-import produced no record or schema change.
    Unchanged,
    /// Records added/removed/changed and/or schema changes.
    Changed(Diff),
    /// A previously imported view is gone from the refreshed project
    /// — the committed dataset is kept untouched.
    Missing,
    /// A view the committed project does not have.
    New,
    /// The view has no record-key column (`GlobalRecordId` or
    /// `UniqueKey`), so no diff is possible.
    NoKey,
}

/// One view's refresh decision.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewRefresh {
    /// The view name.
    pub name: String,
    /// What the re-import does to it.
    pub outcome: RefreshOutcome,
}

/// The refresh plan for a whole re-imported project.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RefreshPlan {
    /// Every view's decision: `next` views in project order, then
    /// `previous`-only views (`Missing`) in committed order.
    pub views: Vec<ViewRefresh>,
}

impl RefreshPlan {
    /// `true` when nothing would change (no `Changed`/`New` views).
    pub fn is_unchanged(&self) -> bool {
        self.views
            .iter()
            .all(|v| !matches!(v.outcome, RefreshOutcome::Changed(_) | RefreshOutcome::New))
    }
}

/// Diffs each view of `next` against `previous` (the committed
/// `(name, dataset)` pairs) by the view's `key_column`
/// (`GlobalRecordId` or `UniqueKey`). Key-column problems surface as
/// the import diff's typed errors (`DuplicateKey`, `MissingKey`, …).
pub fn refresh_plan(
    previous: &[(String, Dataset)],
    next: &LegacyProject,
) -> Result<RefreshPlan, LegacyError> {
    let prev: HashMap<&str, &Dataset> = previous.iter().map(|(n, d)| (n.as_str(), d)).collect();
    let mut views = Vec::new();
    for nv in &next.views {
        let outcome = match prev.get(nv.name.as_str()) {
            None => RefreshOutcome::New,
            Some(_) if nv.key_column.is_empty() => RefreshOutcome::NoKey,
            Some(p) => {
                let d = epiora_import::diff_datasets(p, &nv.dataset, &nv.key_column)?;
                if d.unchanged {
                    RefreshOutcome::Unchanged
                } else {
                    RefreshOutcome::Changed(d)
                }
            }
        };
        views.push(ViewRefresh {
            name: nv.name.clone(),
            outcome,
        });
    }
    let next_names: HashSet<&str> = next.views.iter().map(|v| v.name.as_str()).collect();
    for (name, _) in previous {
        if !next_names.contains(name.as_str()) {
            views.push(ViewRefresh {
                name: name.clone(),
                outcome: RefreshOutcome::Missing,
            });
        }
    }
    Ok(RefreshPlan { views })
}
