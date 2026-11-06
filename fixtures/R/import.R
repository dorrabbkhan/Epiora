# Import fixture corpus (v0.3 Task 2, spec §6.5). Sourced last by
# fixtures/generate.R; expects FIXTURES_DIR to be set.
#
# Each case's values are defined once in R and written both into
# fixtures/import/<case>/input.<ext> (in the case's encoding, delimiter
# or spreadsheet format) and into expected.json (schema
# fixtures/schema/import.schema.json), so the R definitions are the
# oracle (plan Interfaces).
#
# Byte-stability (plan Step 2): openxlsx2 is called with a pinned
# datetime_created; every zip container (xlsx/ods) has its entry DOS
# timestamps rewritten to a fixed date by normalize_zip(); LibreOffice
# conversions run under SOURCE_DATE_EPOCH=0.

XLSX_EPOCH_POSIX <- as.POSIXct("2026-01-01 00:00:00", tz = "UTC")
ZIP_DOS_DATE <- 23585L  # 2026-01-01 in DOS date format
ZIP_DOS_TIME <- 0L

# An empty settings object ({} in JSON, not []).
NO_SETTINGS <- setNames(list(), character(0))

imp_col <- function(name, type, values) {
  # I(): a one-row column must still serialise as a JSON array ([1],
  # not 1) under auto_unbox.
  list(name = name, type = type, values = I(values))
}

# detection block; encoding/delimiter stay NA (-> JSON null) for
# spreadsheets. header_renames is a list of length-2 character vectors.
imp_detection <- function(container, encoding = NA, delimiter = NA,
                          decimal = "dot", header_renames = list(),
                          sheets = character(0)) {
  list(container = container, encoding = encoding, delimiter = delimiter,
       decimal = decimal, header_renames = header_renames,
       # I(): a one-sheet workbook still serialises as an array.
       sheets = I(sheets))
}

# Named zero coercion counts for every column.
no_coercions <- function(columns) {
  setNames(as.list(rep(0L, length(columns))),
           vapply(columns, function(c) c$name, character(1)))
}

# Emit expected.json under fixtures/import/<case>/ and return the dir.
write_import_case <- function(case, file, settings, expect = NULL,
                              expect_error = NULL) {
  dir <- file.path(FIXTURES_DIR, "import", case)
  dir.create(dir, showWarnings = FALSE, recursive = TRUE)
  obj <- list(case = case, file = file, settings = settings)
  obj$expect <- expect
  obj$expect_error <- expect_error
  obj <- obj[!vapply(obj, is.null, logical(1))]
  writeLines(
    jsonlite::toJSON(obj, auto_unbox = TRUE, pretty = TRUE,
                     digits = I(17), na = "null"),
    file.path(dir, "expected.json"))
  invisible(dir)
}

# Text case: emits expected.json and writes records (physical lines,
# delimiters and quotes included) joined with `eol`, in `file_encoding`
# (an iconv target), with a BOM when `bom`.
text_case <- function(case, columns, records, delimiter = ",",
                      encoding = "utf8", decimal = "dot",
                      settings = NO_SETTINGS, header_renames = list(),
                      coercions = NULL, file = "input.csv",
                      file_encoding = "UTF-8", bom = FALSE, eol = "\n",
                      trailing = TRUE) {
  dir <- write_import_case(
    case, file, settings,
    expect = list(
      detection = imp_detection("text", encoding, delimiter, decimal,
                                header_renames),
      columns = columns,
      coercions = if (is.null(coercions)) no_coercions(columns)
                  else coercions))
  text <- paste0(paste(records, collapse = eol),
                 if (trailing) eol else "")
  raw <- iconv(text, from = "UTF-8", to = file_encoding,
               toRaw = TRUE)[[1]]
  stop_if(is.null(raw), sprintf("iconv to %s failed", file_encoding))
  marks <- list("UTF-16LE" = as.raw(c(0xff, 0xfe)),
                "UTF-16BE" = as.raw(c(0xfe, 0xff)),
                "UTF-8" = as.raw(c(0xef, 0xbb, 0xbf)))
  if (bom) {
    stop_if(!file_encoding %in% names(marks),
            sprintf("no BOM defined for %s", file_encoding))
    raw <- c(marks[[file_encoding]], raw)
  }
  writeBin(raw, file.path(dir, file))
  invisible(dir)
}

# Spreadsheet case: emits expected.json, saves wb, unzips, runs `patch`
# (a function of the extracted directory) when given, then rebuilds the
# container byte-stably.
xlsx_case <- function(case, wb, columns, settings = NO_SETTINGS,
                      sheets = "Data", coercions = NULL, patch = NULL,
                      container = "xlsx") {
  dir <- write_import_case(
    case, paste0("input.", container), settings,
    expect = list(
      detection = imp_detection(container, sheets = sheets),
      columns = columns,
      coercions = if (is.null(coercions)) no_coercions(columns)
                  else coercions))
  path <- file.path(dir, paste0("input.", container))
  wb$save(path)
  repack_zip(path, "[Content_Types].xml", patch)
  invisible(path)
}

# Write `members` (files under exdir) as a zip in which every entry is
# stored uncompressed with fixed metadata: no compression (so deflate
# implementations cannot differ between machines), no extra fields,
# fixed DOS time/date, UTF-8-name flag, mode-0644 external attributes.
# The only bytes that can ever differ between environments are the
# member contents themselves.
write_stored_zip <- function(path, exdir, members) {
  u32raw <- function(v) {
    v <- as.double(v)
    as.raw(c(v %% 256, (v %/% 256) %% 256, (v %/% 65536) %% 256,
             (v %/% 16777216) %% 256))
  }
  hex_to_raw <- function(h) {
    as.raw(strtoi(substring(h, seq(1L, 7L, 2L), seq(2L, 8L, 2L)), 16L))
  }
  con <- file(path, "wb")
  on.exit(close(con))
  w <- function(...) for (x in list(...)) writeBin(x, con)
  cds <- list()
  for (m in members) {
    f <- file.path(exdir, m)
    d <- readBin(f, "raw", file.info(f)$size)
    crc <- rev(hex_to_raw(digest::digest(d, algo = "crc32",
                                         serialize = FALSE)))
    name <- charToRaw(m)
    offset <- as.double(seek(con, where = NA))
    w(charToRaw("PK\x03\x04"), u32raw(20)[1:2],
      u32raw(0x0800)[1:2], u32raw(0)[1:2],
      u32raw(ZIP_DOS_TIME)[1:2], u32raw(ZIP_DOS_DATE)[1:2],
      crc, u32raw(length(d)), u32raw(length(d)),
      u32raw(length(name))[1:2], u32raw(0)[1:2],
      name, d)
    cds[[length(cds) + 1L]] <- list(name = name, crc = crc,
                                   size = length(d), offset = offset)
  }
  cd_start <- as.double(seek(con, where = NA))
  attr644 <- u32raw(0x81a40000)
  for (e in cds) {
    w(charToRaw("PK\x01\x02"), u32raw(20)[1:2], u32raw(20)[1:2],
      u32raw(0x0800)[1:2], u32raw(0)[1:2],
      u32raw(ZIP_DOS_TIME)[1:2], u32raw(ZIP_DOS_DATE)[1:2],
      e$crc, u32raw(e$size), u32raw(e$size),
      u32raw(length(e$name))[1:2], u32raw(0)[1:2], u32raw(0)[1:2],
      u32raw(0)[1:2], u32raw(0)[1:2], attr644,
      u32raw(e$offset), e$name)
  }
  cd_size <- as.double(seek(con, where = NA)) - cd_start
  w(charToRaw("PK\x05\x06"), u32raw(0)[1:2], u32raw(0)[1:2],
    u32raw(length(cds))[1:2], u32raw(length(cds))[1:2],
    u32raw(cd_size), u32raw(cd_start), u32raw(0)[1:2])
  invisible(path)
}

# Rebuild a zip fixture byte-stably: unzip, run fn(exdir) for content
# patches, then rewrite the container stored+ordered with `first` at
# the head ([Content_Types].xml for OOXML, mimetype for ODF).
repack_zip <- function(path, first, fn = NULL) {
  members <- utils::unzip(path, list = TRUE)$Name
  members <- members[!grepl("/$", members)]
  stop_if(!first %in% members,
          sprintf("repack_zip: %s missing in %s", first, path))
  ex <- tempfile("zipex")
  dir.create(ex)
  utils::unzip(path, exdir = ex)
  if (!is.null(fn)) fn(ex)
  write_stored_zip(path, ex, c(first, setdiff(members, first)))
  invisible(path)
}

# LibreOffice conversion for .xls and the .ods date case.
# SOURCE_DATE_EPOCH=0 pins LO's embedded timestamps; a private profile
# dir keeps runs hermetic. Returns the output path.
lo_convert <- function(src, to, outdir) {
  dir.create(outdir, showWarnings = FALSE, recursive = TRUE)
  # env -u LD_LIBRARY_PATH: the LD_LIBRARY_PATH R exports for its own
  # libs (/opt/R/.../lib) makes soffice.bin fail to find libreglo.so;
  # LO's own RUNPATH resolves its libs once the variable is unset.
  # Output goes to a log file (these paths never contain spaces).
  log <- tempfile("lo-log-")
  st <- system(paste(
    "SOURCE_DATE_EPOCH=0", "env", "-u", "LD_LIBRARY_PATH", "soffice",
    "--headless",
    paste0("-env:UserInstallation=file://",
           file.path(tempdir(), "epiora-lo-profile")),
    "--convert-to", to, "--outdir", outdir, src,
    ">", log, "2>&1"))
  dst <- file.path(outdir, paste0(tools::file_path_sans_ext(basename(src)),
                                  ".", to))
  stop_if(!file.exists(dst), sprintf(
    "soffice --convert-to %s failed for %s (status %d): %s", to, src,
    st, paste(readLines(log, warn = FALSE), collapse = "\n")))
  dst
}

# --- text cases ----------------------------------------------------------

simple_records <- c("AGE,NAME", "20,alice", ",bob")
simple_cols <- function() {
  list(imp_col("AGE", "integer", c(20L, NA_integer_)),
       imp_col("NAME", "text", c("alice", "bob")))
}

text_case("utf8_comma", simple_cols(), simple_records)
text_case("utf8_bom_comma", simple_cols(), simple_records, bom = TRUE)
text_case("utf8_crlf_no_trailing_newline", simple_cols(), simple_records,
          eol = "\r\n", trailing = FALSE)

text_case("semicolon_decimal_comma",
          list(imp_col("N", "integer", c(7L, 8L)),
               imp_col("X", "real", c(1.5, 3.75)),
               imp_col("Y", "real", c(2.25, 0.5))),
          c("N;X;Y", "7;1,5;2,25", "8;3,75;0,5"),
          delimiter = ";", decimal = "comma")

ab_cols <- list(imp_col("A", "integer", c(1L, 2L)),
                imp_col("B", "text", c("x", "y")))
text_case("tab_delimited", ab_cols, c("A\tB", "1\tx", "2\ty"),
          delimiter = "\t")
text_case("pipe_delimited", ab_cols, c("A|B", "1|x", "2|y"),
          delimiter = "|")
# .tsv extension is only a hint: comma content wins (spec §6.1.3).
text_case("tsv_extension_with_commas", simple_cols(), simple_records,
          file = "input.tsv")

text_case("utf16le_bom_tab", ab_cols, c("A\tB", "1\tx", "2\ty"),
          delimiter = "\t", encoding = "utf16le",
          file_encoding = "UTF-16LE", bom = TRUE)
text_case("utf16be_bom_comma", ab_cols, c("A,B", "1,x", "2,y"),
          encoding = "utf16be", file_encoding = "UTF-16BE", bom = TRUE)

# é, ü, €, ' are Windows-1252 bytes 0xE9, 0xFC, 0x80, 0x92 — all
# invalid as UTF-8, so detection lands on windows1252 (spec §6.1.2).
text_case("windows1252_accents",
          list(imp_col("NAME", "text",
                       c("café", "grün", "€5", "it’s"))),
          c("NAME", "café", "grün", "€5", "it’s"),
          encoding = "windows1252", file_encoding = "WINDOWS-1252")

text_case("quoted_delimiters_and_newlines",
          list(imp_col("NAME", "text", c("a,b", "c")),
               imp_col("NOTE", "text", c("line1\nline2", "plain"))),
          c("NAME,NOTE", "\"a,b\",\"line1", "line2\"", "c,plain"))

# x,x,,x -> x, x_2, column_3, x_3 (spec §6.1.4); renames are reported.
text_case("duplicate_and_empty_headers",
          list(imp_col("x", "integer", 1L), imp_col("x_2", "integer", 2L),
               imp_col("column_3", "integer", 3L),
               imp_col("x_3", "integer", 4L)),
          c("x,x,,x", "1,2,3,4"),
          header_renames = list(c("x", "x_2"), c("", "column_3"),
                                c("x", "x_3")))

text_case("no_header",
          list(imp_col("column_1", "integer", c(1L, 2L)),
               imp_col("column_2", "text", c("a", "b"))),
          c("1,a", "2,b"), settings = list(header = FALSE))

# Zero rows: every candidate type vacuously fits, so the first
# (Boolean) wins (spec §6.2 candidate order). Same rule makes the
# all-missing ALLMISS column Boolean in missing_codes_default.
text_case("header_only",
          list(imp_col("A", "boolean", logical(0)),
               imp_col("B", "boolean", logical(0))),
          "A,B")

# Default missing codes "", ".", "NA" (spec §6.2).
text_case("missing_codes_default",
          list(imp_col("VAL", "integer", c(5L, NA, NA, NA, 7L)),
               imp_col("TAG", "text", c("a", "b", "c", "d", "e")),
               imp_col("ALLMISS", "boolean", rep(NA, 5))),
          c("VAL,TAG,ALLMISS", "5,a,", ".,b,.", "NA,c,NA", ",d,",
            "7,e,."))

# missing_codes replaces the whole default list: with ["-99"] only
# empty cells and "-99" are missing; "NA" and "." stay literal values.
text_case("missing_codes_custom",
          list(imp_col("CODE", "integer", c(1L, NA, 3L)),
               imp_col("NOTES", "text", c("ok", "NA", "."))),
          c("CODE,NOTES", "1,ok", "-99,NA", "3,."),
          settings = list(missing_codes = list("-99")))

# The three accepted spellings (spec §6.2); 0/1 is NOT boolean.
text_case("booleans",
          list(imp_col("TF", "boolean", c(TRUE, FALSE)),
               imp_col("TFU", "boolean", c(TRUE, FALSE)),
               imp_col("YN", "boolean", c(TRUE, FALSE))),
          c("TF,TFU,YN", "true,TRUE,yes", "false,FALSE,no"))

text_case("zero_one_is_integer",
          list(imp_col("Z", "integer", c(0L, 1L, 1L, 0L))),
          c("Z", "0", "1", "1", "0"))

# 2^63 overflows i64, so the whole column falls to Real (spec §6.2).
text_case("i64_overflow_is_real",
          list(imp_col("BIG", "real", c(5, 2^63))),
          c("BIG", "5", "9223372036854775808"))

text_case("iso_dates",
          list(imp_col("D", "date", c("2026-04-03", "2020-06-15"))),
          c("D", "2026-04-03", "2020-06-15"))
text_case("dmy_dates",
          list(imp_col("D", "date", c("2026-04-03", "2020-06-15"))),
          c("D", "03/04/2026", "15/06/2020"),
          settings = list(date_format = "dmy"))
text_case("mdy_dates",
          list(imp_col("D", "date", c("2026-03-04", "2020-12-25"))),
          c("D", "03/04/2026", "12/25/2020"),
          settings = list(date_format = "mdy"))
# DD/MM/YYYY is never guessed: under defaults the column is Text.
text_case("ambiguous_dates_default_text",
          list(imp_col("D", "text", c("03/04/2026", "15/06/2020"))),
          c("D", "03/04/2026", "15/06/2020"))

text_case("override_coerces_to_missing",
          list(imp_col("AGE", "integer", c(20L, NA, 31L))),
          c("AGE", "20", "abc", "31"),
          settings = list(column_types = list(AGE = "integer")),
          coercions = setNames(list(1L), "AGE"))

write_import_case("ragged_row_error", "input.csv", NO_SETTINGS,
                  expect_error = list(code = "ragged_row", line = 4L))
dir <- file.path(FIXTURES_DIR, "import", "ragged_row_error")
writeBin(charToRaw("A,B,C\n1,2,3\n4,5,6\n7,8\n9,10,11\n"),
         file.path(dir, "input.csv"))

write_import_case("empty_file_error", "input.csv", NO_SETTINGS,
                  expect_error = list(code = "empty_file"))
file.create(file.path(FIXTURES_DIR, "import", "empty_file_error",
                      "input.csv"))

# --- spreadsheet cases ---------------------------------------------------

xb_df <- data.frame(
  R = c(1.5, 2.25, 0.5), I = c(1L, 2L, 3L), T = c("a", "b", NA),
  B = c(TRUE, FALSE, TRUE),
  D = as.Date(c("2020-01-02", "2021-03-04", "2022-05-06")),
  check.names = FALSE, stringsAsFactors = FALSE)
xb_cols <- list(
  imp_col("R", "real", c(1.5, 2.25, 0.5)),
  imp_col("I", "integer", c(1L, 2L, 3L)),
  imp_col("T", "text", c("a", "b", NA)),
  imp_col("B", "boolean", c(TRUE, FALSE, TRUE)),
  imp_col("D", "date", c("2020-01-02", "2021-03-04", "2022-05-06")))

new_wb <- function() {
  wb <- openxlsx2::wb_workbook(creator = "epiora",
                               datetime_created = XLSX_EPOCH_POSIX)
  wb$add_worksheet("Data")
  wb
}

# The NA in T is an empty cell, not an error cell: openxlsx2 writes R
# NA as t="e" #N/A; the patch removes the element so this case keeps a
# plain missing (error cells have their own case, xlsx_cell_error).
xlsx_case("xlsx_basic", new_wb()$add_data(x = xb_df), xb_cols,
          patch = function(ex) {
            f <- file.path(ex, "xl", "worksheets", "sheet1.xml")
            x <- paste(readLines(f, warn = FALSE), collapse = "")
            stop_if(!grepl('<c r="C4" t="e"><v>#N/A</v></c>', x,
                           fixed = TRUE),
                    "xlsx_basic: NA cell not where expected")
            writeLines(sub('<c r="C4" t="e"><v>#N/A</v></c>', "",
                           x, fixed = TRUE), f)
          })

# 1904 date system: serials written with the 1904 epoch and the
# workbookPr flag patched in (openxlsx2 has no public 1904 switch).
wb <- new_wb()
wb$add_data(x = data.frame(
  D = unclass(openxlsx2:::conv_to_excel_date(
    as.Date(c("2020-01-02", "2021-03-04")), date1904 = TRUE)),
  check.names = FALSE))
wb$add_numfmt(dims = "A2:A3", numfmt = "yyyy-mm-dd")
xlsx_case("xlsx_1904_dates", wb,
          list(imp_col("D", "date", c("2020-01-02", "2021-03-04"))),
          patch = function(ex) {
            f <- file.path(ex, "xl", "workbook.xml")
            x <- paste(readLines(f, warn = FALSE), collapse = "")
            stop_if(!grepl('<workbookPr date1904="false"/>', x,
                           fixed = TRUE),
                    "xlsx_1904_dates: unexpected workbook.xml")
            writeLines(sub('<workbookPr date1904="false"/>',
                           '<workbookPr date1904="1"/>',
                           x, fixed = TRUE), f)
          })

wb <- openxlsx2::wb_workbook(creator = "epiora",
                             datetime_created = XLSX_EPOCH_POSIX)
wb$add_worksheet("First")
wb$add_data(x = data.frame(X = 9L))
wb$add_worksheet("Second")
wb$add_data(x = data.frame(A = 1:2, B = c("p", "q"),
                           check.names = FALSE))
xlsx_case("xlsx_two_sheets", wb,
          list(imp_col("A", "integer", c(1L, 2L)),
               imp_col("B", "text", c("p", "q"))),
          settings = list(sheet = "Second"),
          sheets = c("First", "Second"))

# Formula cells with cached results: openxlsx2 writes <f> without <v>,
# so the cached <v> is patched into the sheet XML.
wb <- new_wb()
wb$add_data(x = data.frame(A = 1:2, B = 3:4))
wb$add_formula(x = "=A2+B2", dims = "C2")
wb$add_formula(x = "=A3+B3", dims = "C3")
xlsx_case("xlsx_formula_cached", wb,
          list(imp_col("A", "integer", c(1L, 2L)),
               imp_col("B", "integer", c(3L, 4L)),
               imp_col("C", "integer", c(4L, 6L))),
          patch = function(ex) {
            f <- file.path(ex, "xl", "worksheets", "sheet1.xml")
            x <- paste(readLines(f, warn = FALSE), collapse = "")
            x <- sub('<c r="C2" t="str"><f>=A2+B2</f></c>',
                     '<c r="C2"><f>=A2+B2</f><v>4</v></c>', x,
                     fixed = TRUE)
            x <- sub('<c r="C3" t="str"><f>=A3+B3</f></c>',
                     '<c r="C3"><f>=A3+B3</f><v>6</v></c>', x,
                     fixed = TRUE)
            stop_if(!grepl("<v>4</v>", x) || !grepl("<v>6</v>", x),
                    "xlsx_formula_cached: formula cells not patched")
            writeLines(x, f)
          })

# Merged cells keep their value at the top-left anchor only; the
# merged-away B3 is empty (openxlsx2's #N/A error is patched out, same
# empty-cell convention as xlsx_basic).
wb <- new_wb()
wb$add_data(x = data.frame(NAME = c("a", "b"), NOTE = c("shared", NA),
                           check.names = FALSE))
wb$merge_cells(dims = "B2:B3")
xlsx_case("xlsx_merged_cells", wb,
          list(imp_col("NAME", "text", c("a", "b")),
               imp_col("NOTE", "text", c("shared", NA))),
          patch = function(ex) {
            f <- file.path(ex, "xl", "worksheets", "sheet1.xml")
            x <- paste(readLines(f, warn = FALSE), collapse = "")
            stop_if(!grepl('<c r="B3" t="e"><v>#N/A</v></c>', x,
                           fixed = TRUE),
                    "xlsx_merged_cells: NA cell not where expected")
            writeLines(sub('<c r="B3" t="e"><v>#N/A</v></c>',
                           '<c r="B3"/>', x, fixed = TRUE), f)
          })

# A real error cell (t="e", #N/A) becomes missing and is counted in
# coercions (plan Task 2).
wb <- new_wb()
wb$add_data(x = data.frame(SCORE = c(10L, 0L, 30L)))
wb$add_formula(x = "=NA()", dims = "A3")
xlsx_case("xlsx_cell_error", wb,
          list(imp_col("SCORE", "integer", c(10L, NA, 30L))),
          coercions = setNames(list(1L), "SCORE"),
          patch = function(ex) {
            f <- file.path(ex, "xl", "worksheets", "sheet1.xml")
            x <- paste(readLines(f, warn = FALSE), collapse = "")
            x <- sub('<c r="A3" t="str"><f>=NA()</f></c>',
                     '<c r="A3" t="e"><f>=NA()</f><v>#N/A</v></c>', x,
                     fixed = TRUE)
            stop_if(!grepl('t="e"', x),
                    "xlsx_cell_error: error cell not patched")
            writeLines(x, f)
          })

# xlsx_basic's workbook converted to .xls by LibreOffice (plan Task 2).
xls_dir <- file.path(FIXTURES_DIR, "import", "xls_basic")
dir.create(xls_dir, showWarnings = FALSE, recursive = TRUE)
file.copy(lo_convert(file.path(FIXTURES_DIR, "import", "xlsx_basic",
                               "input.xlsx"),
                     "xls", tempfile("xlsout")),
          file.path(xls_dir, "input.xls"))
write_import_case("xls_basic", "input.xls", NO_SETTINGS,
                  expect = list(
                    detection = imp_detection("xls", sheets = "Data"),
                    columns = xb_cols,
                    coercions = no_coercions(xb_cols)))

# ods_basic via readODS (write_ods stores dates and booleans as
# strings, so this case sticks to numeric/text cells).
ob_dir <- file.path(FIXTURES_DIR, "import", "ods_basic")
dir.create(ob_dir, showWarnings = FALSE, recursive = TRUE)
readODS::write_ods(
  data.frame(N = c(1.5, 2.5), I = c(3L, 4L), T = c("u", "v"),
             check.names = FALSE, stringsAsFactors = FALSE),
  file.path(ob_dir, "input.ods"))
repack_zip(file.path(ob_dir, "input.ods"), "mimetype")
ob_cols <- list(
  imp_col("N", "real", c(1.5, 2.5)),
  imp_col("I", "integer", c(3L, 4L)),
  imp_col("T", "text", c("u", "v")))
write_import_case("ods_basic", "input.ods", NO_SETTINGS,
                  expect = list(
                    detection = imp_detection("ods", sheets = "Sheet1"),
                    columns = ob_cols,
                    coercions = no_coercions(ob_cols)))

# ods_dates needs genuine ODS date cells (office:date-value); write_ods
# writes strings, so the file is produced by LibreOffice from an xlsx
# with real date cells.
od_dir <- file.path(FIXTURES_DIR, "import", "ods_dates")
dir.create(od_dir, showWarnings = FALSE, recursive = TRUE)
src <- tempfile(fileext = ".xlsx")
wb <- new_wb()
wb$add_data(x = data.frame(D = as.Date(c("2020-01-02", "2021-03-04")),
                           N = c(1L, 2L), check.names = FALSE))
wb$save(src)
ods_out <- lo_convert(src, "ods", tempfile("odsout"))
repack_zip(ods_out, "mimetype")
file.copy(ods_out, file.path(od_dir, "input.ods"))
od_cols <- list(
  imp_col("D", "date", c("2020-01-02", "2021-03-04")),
  imp_col("N", "integer", c(1L, 2L)))
write_import_case("ods_dates", "input.ods", NO_SETTINGS,
                  expect = list(
                    detection = imp_detection("ods", sheets = "Data"),
                    columns = od_cols,
                    coercions = no_coercions(od_cols)))
