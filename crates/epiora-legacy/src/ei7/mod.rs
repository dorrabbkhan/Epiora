//! The Epi Info 7 mapper (v1.1 spec §4, Task 7): `meta*` tables →
//! `LegacyProject`, a faithful port of the independently written
//! `fixtures/legacy/ei7-project.mjs` + `ei7-grid.mjs` reference so the
//! output structurally equals the golden `project.json` files.

pub(crate) mod datetime;
pub(crate) mod emit;
pub(crate) mod fields;
#[cfg(feature = "fuzzing")]
pub(crate) mod fuzz;
pub(crate) mod meta;
pub(crate) mod page;
pub(crate) mod views;

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use epiora_core::dataset::LabelSet;

use crate::error::LegacyError;
use crate::project::{LegacyProgram, LegacyProject, SourceFormat};
use crate::report::{ReportEntry, ReportKind};
use crate::source::{LegacySource, TableData};

/// Shared mapping state: table cache, claimed tables, label sets,
/// the report, and the view-id → name index for relate links.
pub(crate) struct MapCtx<'a> {
    source: &'a mut dyn LegacySource,
    format: SourceFormat,
    /// Lowercase name → real name.
    table_real: HashMap<String, String>,
    /// Lazily decoded tables, keyed by the *spelled* name (as the
    /// golden's `tableRows` cache is).
    cache: HashMap<String, Option<Rc<TableData>>>,
    /// Lowercase names every pass claimed.
    pub(crate) claimed: HashSet<String>,
    /// Project-level label sets by id.
    pub(crate) label_sets: HashMap<String, LabelSet>,
    /// The accumulated report.
    pub(crate) report: Vec<ReportEntry>,
    /// `ViewId` → view name (relate children).
    view_names: HashMap<String, String>,
}

impl<'a> MapCtx<'a> {
    fn new(source: &'a mut dyn LegacySource, format: SourceFormat) -> Self {
        let table_real = source
            .tables()
            .into_iter()
            .map(|t| (t.to_lowercase(), t))
            .collect();
        Self {
            source,
            format,
            table_real,
            cache: HashMap::new(),
            claimed: HashSet::new(),
            label_sets: HashMap::new(),
            report: Vec::new(),
            view_names: HashMap::new(),
        }
    }

    /// Push one report entry.
    pub(crate) fn add(
        &mut self,
        kind: ReportKind,
        item: impl Into<String>,
        reason: impl Into<String>,
    ) {
        self.report.push(ReportEntry {
            kind,
            item: item.into(),
            reason: reason.into(),
        });
    }

    /// `rowsOf(t)` — a decoded table by spelled name (CI lookup, cached
    /// per spelling). `None` when the table is absent or unreadable.
    pub(crate) fn rows_of(&mut self, spelled: &str) -> Option<Rc<TableData>> {
        if !self.cache.contains_key(spelled) {
            let data = self
                .table_real
                .get(&spelled.to_lowercase())
                .cloned()
                .and_then(|real| self.source.table(&real).ok())
                .map(|t| Rc::new(TableData::from_raw(&t)));
            self.cache.insert(spelled.to_string(), data);
        }
        self.cache.get(spelled).and_then(|o| o.as_ref()).cloned()
    }

    /// `colOf(t, name)` → the physical column's real name. The table
    /// name must match the schema like the golden's `db.columns(t)`:
    /// exactly for Access (schema keys), case-insensitively for SQLite
    /// (`PRAGMA table_info` resolves CI).
    pub(crate) fn col_of(&mut self, spelled: &str, col: &str) -> Option<String> {
        let real = match self.format {
            SourceFormat::Ei7Sqlite | SourceFormat::Ei7SqlServer => {
                self.table_real.get(&spelled.to_lowercase()).cloned()
            }
            _ => self.table_real.values().find(|n| **n == spelled).cloned(),
        }?;
        let t = self.rows_of(&real)?;
        t.col_ci(col).map(|i| t.cols[i].name.clone())
    }

    /// `db.hasTable(t)` — an exact-name table presence check.
    pub(crate) fn has_table_exact(&self, spelled: &str) -> bool {
        self.table_real.values().any(|n| n == spelled)
    }

    /// The view name a `RelatedViewId` resolves to.
    pub(crate) fn view_name(&self, view_id: &str) -> Option<String> {
        self.view_names.get(view_id).cloned()
    }

    /// Decode-error counts per table name (extension tag 3) — drives
    /// the `text_decoding` report entries.
    fn decode_error_tables(&mut self) -> Vec<(String, u32)> {
        let names: Vec<String> = self.table_real.values().cloned().collect();
        let mut out = Vec::new();
        for n in names {
            if let Some(t) = self.rows_of(&n) {
                if t.decode_errors > 0 {
                    out.push((t.name.clone(), t.decode_errors));
                }
            }
        }
        out
    }

    /// Whether any table batch declared a code page (JET3 →
    /// windows-1252, `code_page_assumed`).
    fn any_code_page(&mut self) -> bool {
        let names: Vec<String> = self.table_real.values().cloned().collect();
        names
            .into_iter()
            .any(|n| self.rows_of(&n).is_some_and(|t| t.code_page.is_some()))
    }
}

/// `detect` — layout detection only: the `LegacySource` carries table
/// names, not the backend, so an EI7 layout reports `Ei7Access`; the
/// caller supplies the true backend `SourceFormat` to [`map_ei7`].
/// `view*` tables (and no `metaViews`) mean EI3; anything else is
/// [`LegacyError::LayoutUnknown`] with the plain table list (spec §10).
pub fn detect(source: &dyn LegacySource) -> Result<SourceFormat, LegacyError> {
    let tables = source.tables();
    if tables.iter().any(|t| t.eq_ignore_ascii_case("metaviews")) {
        return Ok(SourceFormat::Ei7Access);
    }
    if tables
        .iter()
        .any(|t| t.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("view")))
    {
        return Ok(SourceFormat::Ei3Access);
    }
    Err(LegacyError::LayoutUnknown {
        tables: plain_tables(&tables),
    })
}

/// The non-metadata tables an unknown-layout error offers for plain
/// import (the golden's `unknownLayoutEntries` filter).
fn plain_tables(tables: &[String]) -> Vec<String> {
    tables
        .iter()
        .filter(|t| {
            let lt = t.to_lowercase();
            !lt.starts_with("meta") && !lt.starts_with("sqlite_")
        })
        .cloned()
        .collect()
}

/// `map_ei7` — one EI7 project (Access or SQLite batches) →
/// [`LegacyProject`].
pub fn map_ei7(
    source: &mut dyn LegacySource,
    format: SourceFormat,
) -> Result<LegacyProject, LegacyError> {
    // A source without `metaViews` is not an EI7 project: the caller
    // gets the plain table list to offer per-table import (plan §7
    // Review Focus 2 — never an empty project or a crash).
    let tables = source.tables();
    if !tables.iter().any(|t| t.eq_ignore_ascii_case("metaviews")) {
        return Err(LegacyError::LayoutUnknown {
            tables: plain_tables(&tables),
        });
    }
    let mut ctx = MapCtx::new(source, format);
    // JET3 text was decoded under an assumed code page (spec §6.1):
    // any batch tag-2 entry means the reader assumed windows-1252.
    let jet3_assumed = ctx.any_code_page();
    let (name, legacy_id) = meta::read_db_info(ctx.rows_of("metaDbInfo").as_deref());
    if jet3_assumed {
        ctx.add(
            ReportKind::CodePageAssumed,
            if name.is_empty() {
                "the file"
            } else {
                name.as_str()
            },
            "JET3 text decoded as windows-1252 (MDB_JET3_CHARSET)",
        );
    }
    let meta_views = meta::read_views(ctx.rows_of("metaViews").as_deref());
    let meta_pages = meta::read_pages(ctx.rows_of("metaPages").as_deref());
    let meta_fields = meta::read_fields(ctx.rows_of("metaFields").as_deref());
    let programs = meta::read_programs(ctx.rows_of("metaPrograms").as_deref());
    let grid_cols = meta::read_grid_cols(ctx.rows_of("metaGridColumns").as_deref());
    let link_items = meta::read_link_items(ctx.rows_of("metaLinks").as_deref());
    ctx.view_names = meta_views
        .iter()
        .map(|v| (v.view_id.clone(), v.name.clone()))
        .collect();

    let mut views = Vec::new();
    let mut datasets = Vec::new();
    let mut links = Vec::new();
    let mut check_code = Vec::new();
    for v in &meta_views {
        // A view that cannot map at all (e.g. no fields → an empty
        // dataset the model cannot represent) is one `view_failed`
        // entry; the other views still map (plan §7 error contract).
        let out = match views::build_view(&mut ctx, v, &meta_fields, &meta_pages, &grid_cols) {
            Ok(out) => out,
            Err(e) => {
                ctx.add(ReportKind::ViewFailed, v.name.clone(), e.to_string());
                continue;
            }
        };
        let mut broken = false;
        for (id, cols) in out.grids {
            let (columns, metas) = emit::out_columns(cols);
            match emit::finish_dataset(&id, columns, metas, &mut ctx) {
                Ok(ds) => datasets.push(ds),
                Err(e) => {
                    ctx.add(ReportKind::ViewFailed, id, e.to_string());
                    broken = true;
                }
            }
        }
        if broken {
            continue;
        }
        links.extend(out.links);
        if let Some(c) = out.check {
            check_code.push(c);
        }
        views.push(out.view);
    }

    for item in link_items {
        ctx.add(
            ReportKind::RecordLinksNotImported,
            item,
            "metaLinks record-level link (EWE); not a schema link",
        );
    }
    if ctx.has_table_exact("Programs")
        && ctx.rows_of("Programs").is_some_and(|t| !t.rows.is_empty())
    {
        ctx.add(
            ReportKind::TableNotImported,
            "Programs",
            "EI3-era programs table; metaPrograms is the EI7 program store",
        );
        ctx.claimed.insert("programs".to_string());
    }

    // Unclaimed tables, sorted by lowercase name so Access and SQLite
    // sources emit in the same order.
    let mut names: Vec<String> = ctx.source.tables();
    names.sort_by_key(|a| a.to_lowercase());
    for t in names {
        let lt = t.to_lowercase();
        if ctx.claimed.contains(&lt) || meta::KNOWN_META.contains(&lt.as_str()) {
            continue;
        }
        let reason = if lt.starts_with("meta") {
            "unhandled metadata table"
        } else if lt.starts_with("code") {
            "unreferenced code table"
        } else if lt.starts_with("view") {
            "EI3-era view-definition table"
        } else {
            "no view, field or code table claims it"
        };
        ctx.add(ReportKind::TableNotImported, t, reason);
    }

    // Tables whose text needed substitution decoding.
    for (t, n) in ctx.decode_error_tables() {
        ctx.add(
            ReportKind::TextDecoding,
            t,
            format!("{n} value(s) decoded with replacement characters"),
        );
    }

    Ok(LegacyProject {
        name,
        legacy_id,
        format,
        views,
        datasets,
        links,
        programs: programs
            .into_iter()
            .map(|(name, text)| LegacyProgram { name, text })
            .collect(),
        check_code,
        report: ctx.report,
    })
}
