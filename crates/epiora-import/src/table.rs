//! The raw string table between csv reading and typed columns, plus
//! header renaming (spec §6.1.4): empty or duplicate names become
//! unique — `x,x,,x` → `x`, `x_2`, `column_3`, `x_3` — and every change
//! is reported in `header_renames`.

use std::collections::HashSet;

/// A parsed text table: unique column names and uniform-width rows.
pub(crate) struct Table {
    /// Final column names (unique, non-empty).
    pub names: Vec<String>,
    /// `(original, renamed)` for each header cell whose name changed.
    pub renames: Vec<(String, String)>,
    /// Data rows, each `names.len()` wide.
    pub rows: Vec<Vec<String>>,
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
