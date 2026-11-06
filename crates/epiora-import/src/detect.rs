//! Deterministic format detection over the first 64 KiB plus file
//! magic (spec §6.1). Delimiters are scored by consistency: the number
//! of sample lines whose quote-aware field count equals the modal
//! count, modal count ≥ 2; ties break in the order `,` `;` tab `|`;
//! no qualifying candidate defaults to `,`. The file extension is a
//! hint only — content decides.

use crate::infer::ascii_trim;
use crate::settings::{Container, DecimalSep, Encoding};
use std::collections::BTreeMap;

/// Detection sample size (spec §6.1: first 64 KiB).
const SAMPLE_BYTES: usize = 64 * 1024;

/// Candidate delimiters in tie-break order.
const DELIMITERS: [char; 4] = [',', ';', '\t', '|'];

const ZIP_MAGIC: &[u8] = b"PK\x03\x04";
const OLE_MAGIC: &[u8] = &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const ODS_MIMETYPE: &[u8] = b"application/vnd.oasis.opendocument.spreadsheet";

/// Container by file magic (spec §6.1.1): an ODF zip is told apart by
/// its first member — an uncompressed `mimetype` entry whose content
/// is the document-type string; any other zip is `Xlsx` (a zip that
/// is neither fails later inside calamine as `spreadsheet`).
pub(crate) fn container(bytes: &[u8]) -> Container {
    if bytes.starts_with(OLE_MAGIC) {
        Container::Xls
    } else if bytes.starts_with(ZIP_MAGIC) {
        if is_ods(bytes) {
            Container::Ods
        } else {
            Container::Xlsx
        }
    } else {
        Container::Text
    }
}

/// Whether `bytes` (already known to start with the zip local-header
/// magic) is an ODF container: the first member's name is `mimetype`
/// and its stored content is the ODS spreadsheet type string. Reads
/// only the first local file header — `PK\x03\x04` sig(4) …
/// compressed-size u32 at 18, name-len u16 at 26, extra-len u16 at 28,
/// name at 30, then the member's stored bytes.
fn is_ods(bytes: &[u8]) -> bool {
    if bytes.len() < 30 {
        return false;
    }
    let name_len = u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
    let extra_len = u16::from_le_bytes([bytes[28], bytes[29]]) as usize;
    let comp_size = u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]) as usize;
    let name_end = 30 + name_len;
    if bytes.get(30..name_end) != Some(b"mimetype".as_slice()) {
        return false;
    }
    let data_start = name_end + extra_len;
    bytes.get(data_start..data_start + comp_size) == Some(ODS_MIMETYPE)
}

/// Encoding by BOM, then UTF-8 validity, then Windows-1252 (spec §6.1.2).
pub(crate) fn encoding(bytes: &[u8]) -> Encoding {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        Encoding::Utf8
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        Encoding::Utf16le
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        Encoding::Utf16be
    } else if std::str::from_utf8(bytes).is_ok() {
        Encoding::Utf8
    } else {
        Encoding::Windows1252
    }
}

/// The first 64 KiB of decoded text, cut on a char boundary.
pub(crate) fn sample(text: &str) -> &str {
    let mut end = text.len().min(SAMPLE_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Scans one physical line, tracking the quote state across calls
/// (`in_quotes` persists: a quoted field may contain newlines).
/// Returns the logical fields, or `None` for a line that is empty or a
/// continuation of a quoted field — neither carries delimiter evidence.
fn scan_line(line: &str, delim: char, in_quotes: &mut bool) -> Option<Vec<String>> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let started = *in_quotes;
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut at_field_start = !started;
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        if *in_quotes {
            if c == '"' {
                if it.peek() == Some(&'"') {
                    it.next();
                    cur.push('"');
                } else {
                    *in_quotes = false;
                }
            } else {
                cur.push(c);
            }
        } else if c == '"' && at_field_start {
            *in_quotes = true;
            at_field_start = false;
        } else if c == delim {
            fields.push(std::mem::take(&mut cur));
            at_field_start = true;
        } else {
            cur.push(c);
            at_field_start = false;
        }
    }
    fields.push(cur);
    if started || line.is_empty() {
        None
    } else {
        Some(fields)
    }
}

/// Per-line quote-aware field counts for `delim` over the sample.
fn field_counts(sample: &str, delim: char) -> Vec<usize> {
    let mut in_quotes = false;
    sample
        .split('\n')
        .filter_map(|line| scan_line(line, delim, &mut in_quotes).map(|f| f.len()))
        .collect()
}

/// The modal field count and its frequency; freq ties break to the
/// larger count.
fn modal_freq(counts: &[usize]) -> (usize, usize) {
    let mut freq: BTreeMap<usize, usize> = BTreeMap::new();
    for c in counts {
        *freq.entry(*c).or_insert(0) += 1;
    }
    freq.into_iter()
        .max_by(|a, b| (a.1, a.0).cmp(&(b.1, b.0)))
        .unwrap_or((0, 0))
}

/// The most consistent delimiter over the sample (spec §6.1.3).
pub(crate) fn delimiter(sample: &str) -> char {
    let mut best = ',';
    let mut best_score = 0usize;
    for d in DELIMITERS {
        let counts = field_counts(sample, d);
        if counts.is_empty() {
            continue;
        }
        let (modal, score) = modal_freq(&counts);
        if modal >= 2 && score > best_score {
            best_score = score;
            best = d;
        }
    }
    best
}

/// `^[+-]?\d+<sep>\d+$` — one fixed-decimal literal.
fn is_fixed_dec(s: &str, sep: char) -> bool {
    let s = s
        .strip_prefix('+')
        .or_else(|| s.strip_prefix('-'))
        .unwrap_or(s);
    let mut parts = s.split(sep);
    match (parts.next(), parts.next(), parts.next()) {
        (Some(a), Some(b), None) => {
            !a.is_empty()
                && a.bytes().all(|c| c.is_ascii_digit())
                && !b.is_empty()
                && b.bytes().all(|c| c.is_ascii_digit())
        }
        _ => false,
    }
}

/// Decimal is `Comma` iff the delimiter is `;`, some sampled field
/// matches `^[+-]?\d+,\d+$` and none matches `^[+-]?\d+\.\d+$`
/// (spec §6.1/Task 3 Step 4).
pub(crate) fn decimal(sample: &str, delimiter: char) -> DecimalSep {
    if delimiter != ';' {
        return DecimalSep::Dot;
    }
    let mut in_quotes = false;
    let mut comma = false;
    let mut dot = false;
    for line in sample.split('\n') {
        if let Some(fields) = scan_line(line, ';', &mut in_quotes) {
            for field in &fields {
                let f = ascii_trim(field);
                if is_fixed_dec(f, ',') {
                    comma = true;
                }
                if is_fixed_dec(f, '.') {
                    dot = true;
                }
            }
        }
    }
    if comma && !dot {
        DecimalSep::Comma
    } else {
        DecimalSep::Dot
    }
}
