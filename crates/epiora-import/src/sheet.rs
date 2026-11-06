//! Spreadsheet reading via `calamine` (v0.3 spec §6.1.5): the chosen
//! sheet's used range becomes a [`Table`] of strings plus a set of
//! cells forced to missing-and-coerced. Date-formatted cells go
//! through [`serial_to_days`] and emit `YYYY-MM-DD` text, error cells
//! and unconvertible serials force a coercion, and everything else
//! reaches §6.2 inference as text — merged and formula cells carry
//! calamine's cached values.

use crate::error::ImportError;
use crate::excel_date::{days_to_iso, serial_to_days};
use crate::settings::{Container, ImportSettings};
use crate::table::{self, Table};
use crate::{MAX_COLUMNS, MAX_ROWS};
use calamine::{Data, Ods, Range, Reader, Xls, Xlsx};
use std::io::{BufReader, Cursor, Read, Seek, Write};

/// A sheet's raw table plus the workbook audit trail.
pub(crate) struct SheetTable {
    /// Column names, header renames and string cells (uniform width).
    pub table: Table,
    /// Sheet names in workbook order.
    pub sheets: Vec<String>,
    /// The sheet actually read.
    pub used: String,
}

/// One cell's fate: candidate text for inference, or a forced
/// coerce-to-missing (counted in `coercions`, never in `missing`).
enum Cell {
    /// String form handed to §6.2 inference/coercion.
    Text(String),
    /// Becomes `None` and counts as a coercion under any chosen type.
    Forced,
}

/// Reads the chosen sheet of `bytes` (a `container` workbook) into a
/// [`SheetTable`]; every calamine failure is a `spreadsheet` error.
pub(crate) fn read(
    bytes: &[u8],
    container: Container,
    s: &ImportSettings,
) -> Result<SheetTable, ImportError> {
    let cursor = Cursor::new(bytes);
    match container {
        Container::Xlsx => {
            let mut wb = Xlsx::new(cursor).map_err(spreadsheet)?;
            let is_1904 = wb.has_1904_epoch();
            read_workbook(&mut wb, is_1904, s)
        }
        Container::Xls => {
            let mut wb = Xls::new(cursor).map_err(spreadsheet)?;
            let is_1904 = wb.has_1904_epoch();
            read_workbook(&mut wb, is_1904, s)
        }
        Container::Ods => {
            let repacked = ods_bytes(bytes)?;
            let mut wb = Ods::new(Cursor::new(repacked)).map_err(spreadsheet)?;
            read_workbook(&mut wb, false, s)
        }
        Container::Text => Err(ImportError::Spreadsheet {
            reason: "text bytes reached the spreadsheet reader".to_string(),
        }),
    }
}

/// Sheets, the chosen sheet's range and its [`Table`] — shared by all
/// three workbook readers.
fn read_workbook<RS: Read + Seek, R: Reader<RS>>(
    wb: &mut R,
    is_1904: bool,
    s: &ImportSettings,
) -> Result<SheetTable, ImportError>
where
    R::Error: std::fmt::Display,
{
    let sheets = wb.sheet_names();
    let used = match &s.sheet {
        Some(name) if sheets.contains(name) => name.clone(),
        Some(name) => return Err(ImportError::SheetNotFound { name: name.clone() }),
        None => match sheets.first() {
            Some(name) => name.clone(),
            None => {
                return Err(ImportError::Spreadsheet {
                    reason: "workbook has no sheets".to_string(),
                });
            }
        },
    };
    let range = wb.worksheet_range(&used).map_err(spreadsheet)?;
    let table = table_of(&range, is_1904, s.header)?;
    Ok(SheetTable {
        table,
        sheets,
        used,
    })
}

/// The used range as a [`Table`]: row 0 is the header when `header`,
/// every cell maps through [`cell_text`] and forced cells land in
/// `forced_coerced` at their data position.
fn table_of(range: &Range<Data>, is_1904: bool, header: bool) -> Result<Table, ImportError> {
    let width = range.width();
    if width == 0 {
        return Err(ImportError::NoColumns);
    }
    if width > MAX_COLUMNS {
        return Err(ImportError::TooManyColumns);
    }
    let first_data = usize::from(header);
    if range.height() < first_data || (range.height() - first_data) as u64 > MAX_ROWS {
        return Err(ImportError::TooManyRows);
    }
    let cell_or_empty = |r: usize, c: usize| -> &Data { range.get((r, c)).unwrap_or(&Data::Empty) };
    let (names, renames) = if header {
        let header_cells: Vec<String> = (0..width)
            .map(|c| match cell_text(cell_or_empty(0, c), is_1904) {
                Cell::Text(t) => t,
                Cell::Forced => String::new(),
            })
            .collect();
        table::sheet_names(&header_cells)
    } else {
        // Without a header row the columns take their sheet letters —
        // the spreadsheet's own names (Task 4 convention).
        ((0..width).map(table::column_letter).collect(), Vec::new())
    };
    let mut rows = Vec::with_capacity(range.height() - first_data);
    let mut forced_coerced = std::collections::HashSet::new();
    for r in first_data..range.height() {
        let mut row = Vec::with_capacity(width);
        for c in 0..width {
            match cell_text(cell_or_empty(r, c), is_1904) {
                Cell::Text(t) => row.push(t),
                Cell::Forced => {
                    forced_coerced.insert((rows.len(), c));
                    row.push(String::new());
                }
            }
        }
        rows.push(row);
    }
    Ok(Table {
        names,
        renames,
        rows,
        forced_coerced,
    })
}

/// One cell's candidate text or `Forced` (plan Task 4 Step 3): an
/// integral number within `i64` is an integer candidate, any other
/// finite number a real candidate; booleans keep the §6.2 spellings;
/// date-formatted cells become ISO text or `Forced`; error cells and
/// non-finite floats are `Forced`; strings and ISO date/duration
/// literals pass through.
fn cell_text(cell: &Data, is_1904: bool) -> Cell {
    match cell {
        Data::Empty => Cell::Text(String::new()),
        Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => Cell::Text(s.clone()),
        Data::Int(i) => Cell::Text(i.to_string()),
        Data::Float(f) => float_text(*f),
        Data::Bool(b) => Cell::Text(if *b { "true" } else { "false" }.to_string()),
        Data::DateTime(dt) if dt.is_datetime() => match serial_to_days(dt.as_f64(), is_1904) {
            Some(days) => Cell::Text(days_to_iso(days)),
            None => Cell::Forced,
        },
        Data::DateTime(dt) => float_text(dt.as_f64()),
        Data::Error(_) => Cell::Forced,
    }
}

/// The candidate text of a numeric value; non-finite is `Forced`.
fn float_text(f: f64) -> Cell {
    if !f.is_finite() {
        return Cell::Forced;
    }
    // ±2^63 are exactly representable in f64; `as i64` saturates, so
    // the upper bound must be strict.
    if f.fract() == 0.0 && (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&f)
    {
        Cell::Text((f as i64).to_string())
    } else {
        Cell::Text(f.to_string())
    }
}

/// Repacks an ODS zip with `content.xml` passed through
/// [`strip_row_cell_whitespace`]. calamine's ODS `read_row` rejects
/// anything but cell elements inside a `table:table-row`, so the
/// pretty-printed XML that readODS (and hand-edited files) write
/// between cells must be normalized before it is parsed. Members other
/// than `content.xml` are copied compressed (their bytes and method are
/// preserved, so `mimetype` stays first and stored); `content.xml` is
/// streamed event by event into its new entry, so peak memory scales
/// with the compressed archive rather than the decompressed document.
/// The repack feeds calamine only; `source_sha256` still covers the raw
/// input.
fn ods_bytes(bytes: &[u8]) -> Result<Vec<u8>, ImportError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(spreadsheet)?;
    let mut out = Vec::with_capacity(bytes.len());
    let mut writer = zip::ZipWriter::new(Cursor::new(&mut out));
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(spreadsheet)?;
        let name = file.name().map_err(spreadsheet)?.into_owned();
        if name == "content.xml" {
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            writer.start_file(name, options).map_err(spreadsheet)?;
            strip_row_cell_whitespace(file, &mut writer)?;
        } else {
            writer.raw_copy_file(file).map_err(spreadsheet)?;
        }
    }
    writer.finish().map_err(spreadsheet)?;
    Ok(out)
}

/// Streams `input` (the decompressed `content.xml` member) to `output`
/// (its replacement zip entry), dropping text that is all ASCII
/// whitespace while it is a direct child of `table:table-row`,
/// `table:table-cell` or `table:covered-table-cell`. Whitespace inside
/// `text:p` (and every deeper element) is cell content and is
/// preserved.
fn strip_row_cell_whitespace<R: Read, W: Write>(input: R, output: W) -> Result<(), ImportError> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_reader(BufReader::new(input));
    let mut writer = quick_xml::Writer::new(output);
    // `true` where the open element is a row or cell element.
    let mut stack: Vec<bool> = Vec::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).map_err(spreadsheet)? {
            Event::Start(e) => {
                stack.push(matches!(
                    e.name().as_ref(),
                    "table:table-row" | "table:table-cell" | "table:covered-table-cell"
                ));
                writer.write_event(Event::Start(e)).map_err(spreadsheet)?;
            }
            Event::End(e) => {
                stack.pop();
                writer.write_event(Event::End(e)).map_err(spreadsheet)?;
            }
            Event::Text(t)
                if stack.last() == Some(&true) && t.bytes().all(|b| b.is_ascii_whitespace()) => {}
            Event::Eof => break,
            e => writer.write_event(e).map_err(spreadsheet)?,
        }
        buf.clear();
    }
    Ok(())
}

/// Any calamine reader failure is a `spreadsheet` error.
fn spreadsheet(e: impl std::fmt::Display) -> ImportError {
    ImportError::Spreadsheet {
        reason: e.to_string(),
    }
}
