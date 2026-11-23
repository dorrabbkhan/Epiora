//! The Epi Info 3.5.x mapper (v1.1 spec §4.2, Task 8): `view<Name>`
//! field-definition tables plus their `SOURCE` data/code references →
//! [`LegacyProject`]. A faithful port of the independently written
//! `fixtures/legacy/ei3-project.mjs` golden, so the output
//! structurally equals the golden `project.json` files.
//!
//! Method source: CDC's own upgrade code —
//! `Epi.ImportExport/Epi2000/{Project,View,Page,MetadataDbProvider,
//! CollectedDataProvider,ProjectAnalyzer}.cs` and
//! `Epi.ImportExport/ProjectUpgradeManager.cs` in
//! `Epi-Info/Epi-Info-Community-Edition` at commit
//! `4cd353c62c40b78d9d1f013b0f5a2bb685a5fca6` (Apache-2.0, D2).

pub(crate) mod fields;
pub(crate) mod meta;

use std::rc::Rc;

use crate::ei7::meta::cell;
use crate::ei7::{emit, MapCtx};
use crate::error::LegacyError;
use crate::plain_tables;
use crate::project::{CheckCode, LegacyProgram, LegacyProject, LegacyView, SourceFormat};
use crate::report::ReportKind;
use crate::source::{Cell, LegacySource, TableData};

/// The product-side D12 record (owner, Task 2): the Epi Info 3.5.4
/// tutorial samples are **redistributable**, so EI3 support is
/// validated against the committed golden corpus and project reports
/// carry no `unvalidated` entry. Had the verdict been `unavailable`
/// this would be `false` — spec §8.2 ships EI3 "unvalidated" and every
/// mapped project's report gets the entry the `ei3` tests' dormant
/// branch asserts.
pub const EI3_VALIDATED: bool = true;

/// `map_ei3` — one EI3 Access project → [`LegacyProject`].
///
/// `name` is the project name the caller supplies (the file stem): an
/// EI3 file carries no project-name table and the golden burns
/// `opts.stem` into `name` plus the `code_page_assumed` and
/// `legacy_layout_unknown` report items, so the caller passes what the
/// file cannot provide — exactly like `map_ei7`'s caller-supplied
/// `format`. (Interface deviation from the plan's `map_ei3(source)` —
/// recorded in `docs/decisions.md`.)
///
/// Views without a data table still map (`view_data_missing`); a
/// dataset the engine cannot represent becomes one `view_failed`
/// entry — the other views still map.
pub fn map_ei3(source: &mut dyn LegacySource, name: &str) -> Result<LegacyProject, LegacyError> {
    let all_tables = source.tables();
    let mut ctx = MapCtx::new(source, SourceFormat::Ei3Access);
    // `view*` tables carrying a `Type` column are the EI3 field
    // definitions (the golden `viewTables`); they are claimed before
    // anything else can be.
    let mut view_tables = Vec::new();
    for t in &all_tables {
        if t.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("view"))
            && ctx.rows_of(t).is_some_and(|d| meta::is_view_table(t, &d))
        {
            view_tables.push(t.clone());
            ctx.claimed.insert(t.to_lowercase());
        }
    }
    // JET3 text was decoded under an assumed code page (spec §6.1):
    // any batch tag-2 entry means the reader assumed windows-1252.
    if ctx.any_code_page() {
        ctx.add(
            ReportKind::CodePageAssumed,
            name,
            "JET3 text decoded as windows-1252 (MDB_JET3_CHARSET)",
        );
    }
    if !EI3_VALIDATED {
        ctx.add(
            ReportKind::Unvalidated,
            name,
            "Epi Info 3.5.x support is not validated against sample projects (D12)",
        );
    }
    if view_tables.is_empty() {
        // `unknownLayoutEntries`: the marker plus one plain-table
        // entry per non-metadata table.
        ctx.add(
            ReportKind::LayoutUnknown,
            name,
            "no recognized view-definition tables",
        );
        let mut names = plain_tables(&all_tables);
        names.sort_by_key(|t| t.to_lowercase());
        for t in names {
            ctx.add(
                ReportKind::TableNotImported,
                t,
                "layout_unknown: plain data table",
            );
        }
        return Ok(LegacyProject {
            name: name.to_string(),
            legacy_id: String::new(),
            format: SourceFormat::Ei3Access,
            views: Vec::new(),
            datasets: Vec::new(),
            links: Vec::new(),
            programs: Vec::new(),
            check_code: Vec::new(),
            report: ctx.report,
        });
    }

    let mut views = Vec::new();
    let mut check_code = Vec::new();
    for vt in &view_tables {
        let Some(def_t) = ctx.rows_of(vt) else {
            continue;
        };
        let def = meta::read_view_def(vt, &def_t);
        ctx.claimed.insert(def.data_table.to_lowercase());
        let data = ctx.rows_of(&def.data_table);
        if data.is_none() {
            ctx.add(
                ReportKind::ViewDataMissing,
                &def.view_name,
                format!("data table {} is not in the file", def.data_table),
            );
        }
        let data = data.unwrap_or_else(|| Rc::new(TableData::empty(&def.data_table)));
        let cols = fields::build_columns(&mut ctx, &def, &data);
        let rec_col = ctx.col_of(&def.data_table, "RECSTATUS");
        let deleted = fields::deleted_count(&data, rec_col.as_deref());
        let (columns, metas) = emit::out_columns(cols);
        match emit::finish_dataset(&def.view_name, columns, metas, &mut ctx) {
            Ok(ds) => views.push(LegacyView {
                name: def.view_name.clone(),
                dataset: ds,
                key_column: "UniqueKey".to_string(),
                deleted_records: deleted,
            }),
            Err(e) => ctx.add(ReportKind::ViewFailed, def.view_name.clone(), e.to_string()),
        }
        let texts = fields::checkcode_texts(&def);
        if !texts.is_empty() {
            check_code.push(CheckCode {
                view: def.view_name.clone(),
                text: texts.join("\n\n"),
                blocks: texts.len() as u32,
            });
        }
    }

    // `Programs` → saved programs (`Name`/`ProgramName` ×
    // `Code`/`Program`/`Content`); claimed whether or not rows carry
    // a name.
    let mut programs = Vec::new();
    if ctx.has_table_exact("Programs") {
        ctx.claimed.insert("programs".to_string());
        if let Some(t) = ctx.rows_of("Programs") {
            for r in 0..t.rows.len() {
                let pname = cell(&t, r, "Name")
                    .or_else(|| cell(&t, r, "ProgramName"))
                    .map_or_else(String::new, Cell::as_string);
                if pname.is_empty() {
                    continue;
                }
                let text = cell(&t, r, "Code")
                    .or_else(|| cell(&t, r, "Program"))
                    .or_else(|| cell(&t, r, "Content"))
                    .map_or_else(String::new, Cell::as_string);
                programs.push(LegacyProgram { name: pname, text });
            }
        }
    }

    // Unclaimed tables, sorted by lowercase name; `code*` tables (and
    // any table a label set is named after) get the code-table reason.
    let mut names = all_tables;
    names.sort_by_key(|a| a.to_lowercase());
    for t in names {
        let lt = t.to_lowercase();
        if ctx.claimed.contains(&lt) || lt.starts_with("meta") {
            continue;
        }
        let reason = if lt.starts_with("code") || ctx.label_sets.contains_key(&t) {
            "unreferenced code table"
        } else {
            "no view or code table claims it"
        };
        ctx.add(ReportKind::TableNotImported, t, reason);
    }

    Ok(LegacyProject {
        name: name.to_string(),
        legacy_id: String::new(),
        format: SourceFormat::Ei3Access,
        views,
        datasets: Vec::new(),
        links: Vec::new(),
        programs,
        check_code,
        report: ctx.report,
    })
}
