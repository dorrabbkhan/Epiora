//! The raw string table between csv reading and typed columns, plus
//! header renaming (spec §6.1.4): empty or duplicate names become
//! unique — `x,x,,x` → `x`, `x_2`, `column_3`, `x_3` — and every change
//! is reported in `header_renames`.

use std::collections::HashSet;

/// A parsed table: unique column names and uniform-width rows.
pub(crate) struct Table {
    /// Final column names (unique, non-empty).
    pub names: Vec<String>,
    /// `(original, renamed)` for each header cell whose name changed.
    pub renames: Vec<(String, String)>,
    /// Data rows, each `names.len()` wide.
    pub rows: Vec<Vec<String>>,
    /// `(row, col)` cells forced to missing and counted in `coercions`
    /// — spreadsheet error cells and unconvertible date serials; their
    /// row strings are placeholders that inference and coercion skip.
    /// Empty for text files.
    pub forced_coerced: HashSet<(usize, usize)>,
}

/// Maps raw header cells to unique names. An empty cell becomes
/// `column_<1-based position>`; any name already taken gets the
/// smallest free `<name>_<k>` suffix (k ≥ 2), giving positional
/// occurrence numbering like the fixtures.
pub(crate) fn unique_names(header: &[String]) -> (Vec<String>, Vec<(String, String)>) {
    let mut used: HashSet<String> = HashSet::with_capacity(header.len());
    let mut names = Vec::with_capacity(header.len());
    let mut renames = Vec::new();
    for (i, cell) in header.iter().enumerate() {
        let base = if cell.is_empty() {
            format!("column_{}", i + 1)
        } else {
            cell.clone()
        };
        let mut name = base.clone();
        let mut k = 2u32;
        while !used.insert(name.clone()) {
            name = format!("{base}_{k}");
            k += 1;
        }
        if name != *cell {
            renames.push((cell.clone(), name.clone()));
        }
        names.push(name);
    }
    (names, renames)
}

/// Maps raw spreadsheet header cells to unique names. An empty cell
/// takes the Excel column letter (`A`, `B`, …, `AA`) — the column's
/// own name — which is not a rename; non-empty names uniquify and
/// report exactly like [`unique_names`] (plan Task 4 /
/// `xlsx_formula_cached`: a missing `C1` header is column `C`).
pub(crate) fn sheet_names(header: &[String]) -> (Vec<String>, Vec<(String, String)>) {
    let mut used: HashSet<String> = HashSet::with_capacity(header.len());
    let mut names = Vec::with_capacity(header.len());
    let mut renames = Vec::new();
    for (i, cell) in header.iter().enumerate() {
        let base = if cell.is_empty() {
            column_letter(i)
        } else {
            cell.clone()
        };
        let mut name = base.clone();
        let mut k = 2u32;
        while !used.insert(name.clone()) {
            name = format!("{base}_{k}");
            k += 1;
        }
        if !cell.is_empty() && name != *cell {
            renames.push((cell.clone(), name.clone()));
        }
        names.push(name);
    }
    (names, renames)
}

/// The Excel column letter of the 0-based index: 0 → `A`, 25 → `Z`,
/// 26 → `AA`.
pub(crate) fn column_letter(i: usize) -> String {
    let mut n = i + 1;
    let mut letters = Vec::new();
    while n > 0 {
        n -= 1;
        letters.push(b'A' + (n % 26) as u8);
        n /= 26;
    }
    letters.reverse();
    String::from_utf8(letters).unwrap_or_default()
}
