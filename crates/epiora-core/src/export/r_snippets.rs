//! `r_snippets` — the R source the generated scripts embed
//! (v1.0 Task 17, spec §8.1). The shared `fixtures/R/methods/*.R`
//! files are MIT-licensed and emitted **verbatim, whole file** — the
//! SPDX line and top-level constants (`ss_levels`, `ss_level_pct`)
//! come along, and the corpus and the exports cannot drift:
//! `export-corpus` + `r-export.yml` re-run the same text the
//! generator ran. From the non-methods corpus sources (`common.R`,
//! `calc.R`, …) the script embeds only the `name <- function(...)`
//! definitions in the driver's transitive call closure.
//!
//! `snippet_set(&[seed names])` returns, in source order: the methods
//! files the closure touches (whole), then the extracted function
//! definitions. The generator entry points (`write_fixture`,
//! `calc_write`, `pkg_versions`, `write_dataset`) are shimmed in
//! `r_emit`'s PRELUDE and never extracted.

use crate::EngineError;
use std::collections::{BTreeMap, BTreeSet};

/// The MIT-licensed shared methods — emitted whole file when the
/// closure uses any of their functions.
const METHODS: &[&str] = &[
    include_str!("../../../../fixtures/R/methods/dose_response.R"),
    include_str!("../../../../fixtures/R/methods/matched.R"),
    include_str!("../../../../fixtures/R/methods/means.R"),
    include_str!("../../../../fixtures/R/methods/poisson.R"),
    include_str!("../../../../fixtures/R/methods/proportion.R"),
    include_str!("../../../../fixtures/R/methods/rates.R"),
    include_str!("../../../../fixtures/R/methods/sample_size.R"),
    include_str!("../../../../fixtures/R/methods/screening.R"),
    include_str!("../../../../fixtures/R/methods/tables.R"),
];

/// The corpus drivers/helpers the closure extracts `name <-
/// function` definitions from. Generation scripts that need
/// `DATASETS`/`FIXTURES_DIR` (`data.R`, `import.R`, `import_zip.R`)
/// are deliberately absent — exported analyses never read the corpus
/// datasets through them.
const DEFS_SOURCES: &[&str] = &[
    include_str!("../../../../fixtures/R/common.R"),
    include_str!("../../../../fixtures/R/nchyper.R"),
    include_str!("../../../../fixtures/R/twobytwo.R"),
    include_str!("../../../../fixtures/R/stratified.R"),
    include_str!("../../../../fixtures/R/rxc.R"),
    include_str!("../../../../fixtures/R/freq.R"),
    include_str!("../../../../fixtures/R/means.R"),
    include_str!("../../../../fixtures/R/special.R"),
    include_str!("../../../../fixtures/R/calc.R"),
];

/// Generator entry points the script replaces with a printer shim.
/// `special_fixture` is NOT shimmed — the verbatim version builds
/// `inputs`/`expected` and hands them to the shimmed `write_fixture`.
const SHIMMED: &[&str] = &[
    "write_fixture",
    "calc_write",
    "pkg_versions",
    "write_dataset",
];

fn invalid(reason: impl Into<String>) -> EngineError {
    EngineError::InvalidOptions {
        reason: reason.into(),
    }
}

/// `true` for an R identifier start character.
fn ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'.'
}

/// A `name <- function(args)` definition's source text, from the
/// `name` at `start` to the end of the body. Signatures span lines, so
/// the body's `{` is the first one at paren depth 0 after the args
/// list (strings and comments skipped); a bare-expression body
/// (`f <- function(x) qnorm(x)`) ends at the end of its line.
fn fn_text(src: &str, start: usize) -> Option<(String, String)> {
    let rest = src.get(start..)?;
    let nl = rest.find('\n').map(|i| i + 1).unwrap_or(rest.len());
    let first = &rest[..nl];
    let (name, _) = first.split_once("<- function")?;
    let name = name.trim();
    if !name
        .bytes()
        .all(|b| ident_start(b) || b.is_ascii_digit() || b == b'_')
    {
        return None;
    }
    let fpos = rest.find("<- function")?;
    let bytes = src.as_bytes();
    let mut i = start + fpos + "<- function".len();
    let mut depth = 0_i32;
    let mut saw_paren = false;
    let mut in_str: Option<u8> = None;
    let mut escaped = false;
    let mut in_comment = false;
    let mut open = usize::MAX;
    while i < bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
        } else if let Some(q) = in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                in_str = None;
            }
        } else if b == b'#' {
            in_comment = true;
        } else if b == b'"' || b == b'\'' {
            in_str = Some(b);
        } else if b == b'(' {
            depth += 1;
            saw_paren = true;
        } else if b == b')' {
            depth -= 1;
        } else if b == b'{' {
            if saw_paren && depth == 0 {
                open = i;
                break;
            }
        } else if saw_paren && depth == 0 && !matches!(b, b' ' | b'\t' | b'\r' | b'\n') {
            let end = src[i..].find('\n').map_or(src.len(), |k| i + k);
            return Some((name.to_string(), src[start..end].to_string()));
        }
        i += 1;
    }
    if open == usize::MAX {
        return None;
    }
    let open = open;
    let bytes = src.as_bytes();
    let mut i = open;
    let mut depth = 0_i32;
    let mut in_str: Option<u8> = None;
    let mut escaped = false;
    let mut in_comment = false;
    while i < bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
        } else if let Some(q) = in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                in_str = None;
            }
        } else if b == b'#' {
            in_comment = true;
        } else if b == b'"' || b == b'\'' {
            in_str = Some(b);
        } else if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                i += 1;
                break;
            }
        }
        i += 1;
    }
    if depth != 0 {
        return None;
    }
    Some((name.to_string(), src[start..i].to_string()))
}

/// All top-level `name <- function` definitions of `src`, in source
/// order. A name a nested function reuses is only collected at column
/// 0, so local helpers never shadow globals.
fn collect(src: &str, out: &mut Vec<(String, String)>) {
    let mut pos = 0_usize;
    let bytes = src.as_bytes();
    while pos < bytes.len() {
        if pos == 0 || bytes[pos - 1] == b'\n' {
            let mut end = pos;
            while end < bytes.len()
                && (ident_start(bytes[end]) || bytes[end].is_ascii_digit() || bytes[end] == b'_')
            {
                end += 1;
            }
            if src[pos..end]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && src[end..].trim_start().starts_with("<- function")
            {
                if let Some((name, text)) = fn_text(src, pos) {
                    out.push((name, text));
                }
            }
        }
        pos += 1;
    }
}

/// Every identifier in `text` (code only — strings and comments are
/// blanked first). The caller intersects this with the def map, so
/// `lapply(x, leaf)`-style references and `name(...)` calls both
/// resolve; identifiers that match no def are ignored.
fn referenced_names(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut masked = bytes.to_vec();
    let mut i = 0;
    let mut in_str: Option<u8> = None;
    let mut escaped = false;
    let mut in_comment = false;
    while i < bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            } else {
                masked[i] = b' ';
            }
        } else if let Some(q) = in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == q {
                in_str = None;
            }
            masked[i] = b' ';
        } else if b == b'#' {
            in_comment = true;
            masked[i] = b' ';
        } else if b == b'"' || b == b'\'' {
            in_str = Some(b);
        }
        i += 1;
    }
    let masked = String::from_utf8_lossy(&masked);
    let mut out = Vec::new();
    let mb = masked.as_bytes();
    let mut p = 0;
    while p < mb.len() {
        if ident_start(mb[p]) {
            let mut end = p + 1;
            while end < mb.len()
                && (ident_start(mb[end]) || mb[end].is_ascii_digit() || mb[end] == b'_')
            {
                end += 1;
            }
            out.push(masked[p..end].to_string());
            p = end;
        } else {
            p += 1;
        }
    }
    out
}

/// The name → (methods-file index | defs-source index, position,
/// text) map over every source. `true` in the flag marks a
/// methods-file function (its file is emitted whole when the
/// closure reaches it).
fn map() -> BTreeMap<String, (bool, usize, usize, String)> {
    let mut out = BTreeMap::new();
    for (si, src) in METHODS.iter().enumerate() {
        let mut defs = Vec::new();
        collect(src, &mut defs);
        for (pi, (name, text)) in defs.into_iter().enumerate() {
            out.entry(name).or_insert((true, si, pi, text));
        }
    }
    for (si, src) in DEFS_SOURCES.iter().enumerate() {
        let mut defs = Vec::new();
        collect(src, &mut defs);
        for (pi, (name, text)) in defs.into_iter().enumerate() {
            if SHIMMED.contains(&name.as_str()) {
                continue;
            }
            out.entry(name).or_insert((false, si, pi, text));
        }
    }
    out
}

/// The verbatim R for everything in `seeds`' transitive call
/// closure: whole methods files first (in `METHODS` order), then the
/// extracted definitions in `DEFS_SOURCES` order. `InvalidOptions`
/// when a seed is unknown.
pub fn snippet_set(seeds: &[&str]) -> Result<String, EngineError> {
    let map = map();
    let mut have: BTreeSet<String> = BTreeSet::new();
    let mut queue: Vec<String> = Vec::new();
    for seed in seeds {
        if !map.contains_key(*seed) {
            return Err(invalid(format!("R source has no function {seed:?}")));
        }
        if have.insert((*seed).to_string()) {
            queue.push((*seed).to_string());
        }
    }
    // The closure can pull methods-file functions; scanning a whole
    // emitted file would re-add the same names, so only the collected
    // defs' bodies feed the search. Methods-file top-level constants
    // (ss_levels &c.) ship with their files.
    while let Some(name) = queue.pop() {
        let (_, _, _, text) = &map[&name];
        for call in referenced_names(text) {
            if map.contains_key(&call) && have.insert(call.clone()) {
                queue.push(call);
            }
        }
    }
    let mut methods_files: BTreeSet<usize> = BTreeSet::new();
    let mut defs: Vec<(usize, usize, String)> = Vec::new();
    for name in &have {
        let (is_methods, si, pi, text) = &map[name];
        if *is_methods {
            methods_files.insert(*si);
        } else {
            defs.push((*si, *pi, text.clone()));
        }
    }
    defs.sort();
    let mut parts: Vec<String> = methods_files
        .iter()
        .map(|si| METHODS[*si].trim().to_string())
        .collect();
    parts.extend(defs.into_iter().map(|(_, _, text)| text));
    Ok(parts.join("\n\n"))
}
