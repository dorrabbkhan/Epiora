//! The recipe (v1.1 spec §5.4): the canonical command sequence of a
//! session — the as-of instant plus every cleaning command since the
//! last READ. `canonical_text` is the stable, re-parseable spelling a
//! receipt's `working_view` stamp hashes; `working_view_stamp` is the
//! stamp itself (`None` while the recipe has no steps — a run on a
//! view that never saw a cleaning command is a plain v1/v2 receipt).

use crate::expr::eval::AsOf;
use crate::functions::calendar::civil_from_days;
use crate::manage::Command;
use crate::pgm::print::write_command;
use crate::view::WorkingView;
use epiora_core::dataset::Dataset;
use epiora_core::receipt::{DatasetStamp, WorkingViewStamp};
use sha2::Digest;

/// A session's recorded commands with its as-of instant.
#[derive(Clone, Debug, PartialEq)]
pub struct Recipe {
    /// The session's as-of instant (`SYSTEMDATE`/`SYSTEMTIME`).
    pub as_of: AsOf,
    /// Commands since the last READ.
    pub steps: Vec<Command>,
}

impl Recipe {
    /// The canonical, re-parseable command text: one `AS-OF` header
    /// then one block per step, `Expr` printed in its fully
    /// parenthesized form so `parse(print(e)) = e`.
    pub fn canonical_text(&self) -> String {
        let mut out = String::new();
        let (y, m, d) = civil_from_days(self.as_of.date);
        out.push_str(&format!(
            "AS-OF {y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}\n",
            self.as_of.seconds / 3600,
            self.as_of.seconds % 3600 / 60,
            self.as_of.seconds % 60
        ));
        for (i, step) in self.steps.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            write_command(&mut out, step);
        }
        out.trim_end_matches('\n').to_string()
    }

    /// SHA-256 hex of the canonical text — `recipe_sha256` in the
    /// receipt's working-view stamp.
    pub fn sha256_hex(&self) -> String {
        let digest = sha2::Sha256::digest(self.canonical_text().as_bytes());
        let mut hex = String::with_capacity(digest.len() * 2);
        for byte in digest {
            hex.push(char::from_digit((byte >> 4) as u32, 16).unwrap_or('0'));
            hex.push(char::from_digit((byte & 0xf) as u32, 16).unwrap_or('0'));
        }
        hex
    }
}

/// The `working_view` stamp a session worker adds to `RunContext`
/// once a recipe has steps (spec §6.1): the source dataset's stamp,
/// the recipe text and hash, and the materialized view's content
/// hash. `None` for a step-less recipe — receipt v4 exists only when
/// a working view exists.
pub fn working_view_stamp(
    source: &Dataset,
    view: &Dataset,
    recipe: &Recipe,
) -> Option<WorkingViewStamp> {
    if recipe.steps.is_empty() {
        return None;
    }
    Some(WorkingViewStamp {
        source: DatasetStamp {
            id: source.id().to_string(),
            version: source.version(),
            sha256: source.content_hash_hex(),
        },
        recipe_text: recipe.canonical_text(),
        recipe_sha256: recipe.sha256_hex(),
        view_sha256: view.content_hash_hex(),
    })
}

/// Replays the recipe's steps over `source` — deterministic: the same
/// recipe always yields the same view hash or the same typed error.
pub fn replay(
    source: &Dataset,
    linked: &dyn crate::view::LinkedDatasets,
    recipe: &Recipe,
) -> Result<WorkingView, crate::LangError> {
    let mut view = WorkingView::new(source.clone());
    let mut env = crate::expr::eval::EvalEnv {
        as_of: recipe.as_of,
        globals: std::collections::BTreeMap::new(),
    };
    for step in &recipe.steps {
        crate::manage::apply(&mut view, linked, &mut env, step)?;
    }
    Ok(view)
}
