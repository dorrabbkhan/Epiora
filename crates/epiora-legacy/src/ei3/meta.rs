//! The EI3 `view<Name>` definition-table readers — ports of the
//! golden's `isPseudoRow`, `parseEi3List` and the SOURCE/`Datatable`
//! resolution in `buildEi3Project` (`fixtures/legacy/ei3-project.mjs`).

use std::collections::HashMap;

use crate::ei7::meta::{cell, js_truthy, n_cell};
use crate::source::{Cell, TableData};

/// `String(ci(r, col) || '')` — raw cell text; absent column, missing
/// cell and every JS-falsy value give `""`.
pub(crate) fn s_or(t: &TableData, r: usize, col: &str) -> String {
    let c = cell(t, r, col);
    if js_truthy(c) {
        c.map_or_else(String::new, Cell::as_string)
    } else {
        String::new()
    }
}

/// `String(ci(r, col) || '').trim()`.
fn s_trim(t: &TableData, r: usize, col: &str) -> String {
    s_or(t, r, col).trim().to_string()
}

/// `/^view/i` prefix + a `Type` column (the golden `viewTables` rule).
pub(crate) fn is_view_table(name: &str, t: &TableData) -> bool {
    name.get(..4)
        .is_some_and(|p| p.eq_ignore_ascii_case("view"))
        && t.col_ci("Type").is_some()
}

/// A SOURCE row: `String(ci(r,'Type')||'').toUpperCase() === 'SOURCE'`.
/// Note the golden uppercases *here* but compares `type === 'SOURCE'`
/// raw in `isPseudoRow` — a lowercase `source` row is a source row
/// that is also a field row (→ `field_not_imported`).
fn is_source(t: &TableData, r: usize) -> bool {
    s_or(t, r, "Type").to_uppercase() == "SOURCE"
}

/// `/^data\d*$/i` on the (already trimmed) SOURCE `Name`.
fn is_data_name(name: &str) -> bool {
    name.get(..4)
        .is_some_and(|p| p.eq_ignore_ascii_case("data"))
        && name[4..].bytes().all(|b| b.is_ascii_digit())
}

/// `isPseudoRow` — the rows that are not data fields: `Type` `''`,
/// `SOURCE`, `RECORD` or `Background`; `Name` `RECORD`/`VIEW`/
/// `GENERAL`; or `PageNumber <= 0`.
pub(crate) fn is_pseudo_row(t: &TableData, r: usize) -> bool {
    let ty = s_trim(t, r, "Type");
    let name = s_trim(t, r, "Name");
    let page = n_cell(t, r, "PageNumber");
    ty.is_empty()
        || ty == "SOURCE"
        || ty == "RECORD"
        || ty.eq_ignore_ascii_case("Background")
        || matches!(name.to_uppercase().as_str(), "RECORD" | "VIEW" | "GENERAL")
        || page.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
}

/// `parseEi3List` — `"X(SRC,stored*display[,s2*d2])"` → the `SRC`
/// source name and the stored*display pairs (`b || a` without `*`).
pub(crate) struct Ei3List {
    /// The `SRC` token — a SOURCE row name (`CODE*`/`DATA*`) or a
    /// table name.
    pub source: String,
    /// `stored*display` pairs.
    pub cols: Vec<(String, String)>,
}

/// `/^\s*([A-Z])\s*\(\s*([^,)]+)\s*,\s*([^)]+)\)/i`.
pub(crate) fn parse_ei3_list(list: &str) -> Option<Ei3List> {
    let s = list.trim_start();
    let letter = s.chars().next()?;
    if !letter.is_ascii_alphabetic() {
        return None;
    }
    let s = s[letter.len_utf8()..].trim_start().strip_prefix('(')?;
    let split = s.find([',', ')'])?;
    if s.as_bytes()[split] != b',' || split == 0 {
        return None;
    }
    let source = s[..split].trim().to_string();
    if source.is_empty() {
        return None;
    }
    let rest = s[split + 1..].trim_start();
    let close = rest.find(')')?;
    let cols = rest[..close]
        .split(',')
        .map(|p| {
            let mut it = p.split('*');
            let a = it.next().unwrap_or("").trim().to_string();
            let b = it
                .next()
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .unwrap_or_else(|| a.clone());
            (a, b)
        })
        .collect();
    Some(Ei3List { source, cols })
}

/// One field row of a `view*` table (every row `isPseudoRow` keeps).
pub(crate) struct Ei3Field {
    /// `Name` trimmed — the dataset column name.
    pub name: String,
    /// `Name` raw — the mirror-source find compares the *untrimmed*
    /// lowercased text.
    pub name_raw: String,
    /// `Type` trimmed and uppercased — the `EI3_TYPE` key.
    pub ty: String,
    /// `PromptText` when present and truthy (`|| undefined`).
    pub prompt: Option<String>,
    /// `Lists ?? List` — the raw spec text.
    pub list_expr: Option<String>,
    /// `Formatstring` raw.
    pub formatstring: String,
    /// `Number(Decimals)` — NaN when absent.
    pub decimals: f64,
    /// `Checkcode` raw (`|| ''`).
    pub checkcode: String,
}

/// A `view<Name>` definition table parsed for the field loop.
pub(crate) struct ViewDef {
    /// The view name — the `view` prefix stripped.
    pub view_name: String,
    /// The data table — the `DATA\d*` SOURCE row's `Datatable`, or the
    /// first SOURCE whose Datatable is not `code*`, or the view name.
    pub data_table: String,
    /// SOURCE `Name.toUpperCase()` → `Datatable`, for `X(SRC,…)` specs.
    pub source_map: HashMap<String, String>,
    /// The non-pseudo rows, in file order.
    pub fields: Vec<Ei3Field>,
}

/// Parse a `view*` definition table into a [`ViewDef`].
pub(crate) fn read_view_def(table_name: &str, t: &TableData) -> ViewDef {
    let view_name = table_name.get(4..).unwrap_or(table_name).to_string();
    // The view's data table: the `DATA\d*` SOURCE row's `Datatable`,
    // else the first SOURCE whose Datatable is not `code*`.
    let src = (0..t.rows.len())
        .find(|&r| is_source(t, r) && is_data_name(&s_trim(t, r, "Name")))
        .or_else(|| {
            (0..t.rows.len()).find(|&r| {
                is_source(t, r)
                    && !s_or(t, r, "Datatable")
                        .get(..4)
                        .is_some_and(|p| p.eq_ignore_ascii_case("code"))
            })
        });
    let data_table = match src {
        Some(r) => {
            let c = cell(t, r, "Datatable");
            // `String(ci(src,'Datatable') || viewName)` — falsy cells
            // and absent columns fall back to the view name.
            if js_truthy(c) {
                c.map_or_else(|| view_name.clone(), Cell::as_string)
            } else {
                view_name.clone()
            }
        }
        None => view_name.clone(),
    };
    let source_map = (0..t.rows.len())
        .filter(|&r| is_source(t, r))
        .map(|r| (s_trim(t, r, "Name").to_uppercase(), s_or(t, r, "Datatable")))
        .collect();
    let fields = (0..t.rows.len())
        .filter(|&r| !is_pseudo_row(t, r))
        .map(|r| {
            let prompt = {
                let c = cell(t, r, "PromptText");
                js_truthy(c).then(|| c.map_or_else(String::new, Cell::as_string))
            };
            Ei3Field {
                name: s_trim(t, r, "Name"),
                name_raw: s_or(t, r, "Name"),
                ty: s_trim(t, r, "Type").to_uppercase(),
                prompt,
                list_expr: cell(t, r, "Lists")
                    .or_else(|| cell(t, r, "List"))
                    .map(Cell::as_string),
                formatstring: s_or(t, r, "Formatstring"),
                decimals: n_cell(t, r, "Decimals"),
                checkcode: s_or(t, r, "Checkcode"),
            }
        })
        .collect();
    ViewDef {
        view_name,
        data_table,
        source_map,
        fields,
    }
}
