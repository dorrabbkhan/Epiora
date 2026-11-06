# Zip container writers and the LibreOffice bridge for the import
# fixture corpus (v0.3 Task 2, spec §6.5). Sourced by import.R;
# expects FIXTURES_DIR and stop_if() to be defined.
#
# Byte-stability (plan Step 2):
# - write_stored_zip() writes every member stored uncompressed with
#   fixed DOS time/date, no extra fields and fixed attributes, so the
#   only bytes that can differ between environments are the member
#   contents themselves.
# - repack_zip() rebuilds a producer zip through write_stored_zip() in
#   canonical member order (the named member first, the rest sorted),
#   optionally patching member contents first.
# - lo_convert() runs LibreOffice under SOURCE_DATE_EPOCH=0 and TZ=UTC
#   (doc metadata and OLE timestamps otherwise follow wall clock and
#   the local timezone), a private profile seeded with a fixed author
#   (the OS username otherwise lands in dc:creator and the .xls Author
#   / WriteAccess records), and a fontconfig confined to the build's
#   bundled fonts (document font tables, column widths and the
#   rendered thumbnail otherwise follow the host's installed fonts).
# - normalize_ods_meta() fixes the meta.xml producer fields so the
#   file carries no LO-build or user identity.
#
# LO output still differs between releases, so the pinned TDF build is
# required: EPIORA_SOFFICE points at its program/soffice (the
# fixtures.yml workflow downloads the same tarball, verified by
# SHA-256), and the version string is checked below.

ZIP_DOS_DATE <- 23585L  # 2026-01-01 in DOS date format
ZIP_DOS_TIME <- 0L

XLSX_EPOCH_POSIX <- as.POSIXct("2026-01-01 00:00:00", tz = "UTC")

# The pinned TDF build's full --version string (git hash fingerprint,
# not a distro "NNN(Build:N)" suffix), so only that build generates.
LO_REQUIRED_VERSION <-
  "LibreOffice 26.2.6.3 8221e31b3ac356a1623c672912a3d2b492f7e3d1"
LO_AUTHOR <- "epiora"
LO_META_DATETIME <- "2026-01-01T00:00:00"

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
  # sort: member order must not depend on the producer's filesystem
  # enumeration order (differs between environments).
  write_stored_zip(path, ex, c(first, sort(setdiff(members, first))))
  invisible(path)
}

# A private LO profile whose UserProfile/Data seeds a fixed author;
# otherwise LO takes the OS username and it lands in dc:creator /
# meta:initial-creator and in the .xls Author and WriteAccess records.
lo_profile <- function() {
  dir <- file.path(tempdir(), "epiora-lo-profile")
  user <- file.path(dir, "user")
  dir.create(user, showWarnings = FALSE, recursive = TRUE)
  writeLines(c(
    '<?xml version="1.0" encoding="UTF-8"?>',
    paste0('<oor:items xmlns:oor="http://openoffice.org/2001/registry"',
           ' xmlns:xs="http://www.w3.org/2001/XMLSchema"',
           ' xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">'),
    sprintf(paste0(' <item oor:path="/org.openoffice.UserProfile/Data">',
                   '<prop oor:name="%s" oor:op="fuse"><value>%s</value>',
                   '</prop></item>'),
            "givenname", LO_AUTHOR),
    sprintf(paste0(' <item oor:path="/org.openoffice.UserProfile/Data">',
                   '<prop oor:name="%s" oor:op="fuse"><value>%s</value>',
                   '</prop></item>'),
            "sn", ""),
    sprintf(paste0(' <item oor:path="/org.openoffice.UserProfile/Data">',
                   '<prop oor:name="%s" oor:op="fuse"><value>%s</value>',
                   '</prop></item>'),
            "initials", ""),
    '</oor:items>'),
    file.path(user, "registrymodifications.xcu"))
  dir
}

# A fontconfig file that sees only the fonts bundled with the pinned
# LO build (<root>/program/soffice -> <root>/share/fonts); without it
# the host's installed fonts change the document font tables, default
# column widths and the rendered thumbnail.
lo_fonts_conf <- function(soffice) {
  lo_root <- dirname(dirname(normalizePath(soffice)))
  fonts <- file.path(lo_root, "share", "fonts")
  stop_if(!dir.exists(fonts), sprintf(
    paste0("lo_convert: no bundled fonts at %s — EPIORA_SOFFICE must ",
           "point at the pinned TDF build's program/soffice"), fonts))
  conf <- tempfile("fonts-", fileext = ".conf")
  writeLines(c(
    '<?xml version="1.0"?>',
    '<!DOCTYPE fontconfig SYSTEM "fonts.dtd">',
    '<fontconfig>',
    sprintf('  <dir>%s</dir>', fonts),
    sprintf('  <cachedir>%s</cachedir>',
            file.path(tempdir(), "epiora-fccache")),
    '</fontconfig>'), conf)
  conf
}

# The soffice binary: the pinned TDF build via EPIORA_SOFFICE, else
# PATH (whose version string is still checked against the pin).
# env -u LD_LIBRARY_PATH as in lo_convert(): R's own library path
# makes soffice.bin fail to find its libs.
lo_soffice <- function() {
  soffice <- Sys.getenv("EPIORA_SOFFICE", "soffice")
  ver <- tryCatch(system(paste("env -u LD_LIBRARY_PATH",
                               shQuote(soffice), "--version"),
                         intern = TRUE),
                  warning = function(w) conditionMessage(w),
                  error = function(e) conditionMessage(e))
  stop_if(!any(grepl(LO_REQUIRED_VERSION, ver, fixed = TRUE)), sprintf(
    paste0("lo_convert: need %s (set EPIORA_SOFFICE to the pinned ",
           "TDF build's program/soffice); got: %s"),
    LO_REQUIRED_VERSION, paste(ver, collapse = " ")))
  soffice
}

# LibreOffice conversion for .xls and the .ods date case.
# SOURCE_DATE_EPOCH=0 pins LO's embedded timestamps and TZ=UTC keeps
# them timezone-invariant; FONTCONFIG_FILE confines font discovery to
# the build's bundled fonts. Returns the output path.
lo_convert <- function(src, to, outdir) {
  dir.create(outdir, showWarnings = FALSE, recursive = TRUE)
  soffice <- lo_soffice()
  # env -u LD_LIBRARY_PATH: the LD_LIBRARY_PATH R exports for its own
  # libs (/opt/R/.../lib) makes soffice.bin fail to find libreglo.so;
  # LO's own RUNPATH resolves its libs once the variable is unset.
  # Output goes to a log file (these paths never contain spaces).
  log <- tempfile("lo-log-")
  st <- system(paste(
    "SOURCE_DATE_EPOCH=0", "TZ=UTC",
    paste0("FONTCONFIG_FILE=", lo_fonts_conf(soffice)),
    "env", "-u", "LD_LIBRARY_PATH", shQuote(soffice),
    "--headless",
    paste0("-env:UserInstallation=file://", lo_profile()),
    "--convert-to", to, "--outdir", outdir, src,
    ">", log, "2>&1"))
  dst <- file.path(outdir, paste0(tools::file_path_sans_ext(basename(src)),
                                  ".", to))
  stop_if(!file.exists(dst), sprintf(
    "soffice --convert-to %s failed for %s (status %d): %s", to, src,
    st, paste(readLines(log, warn = FALSE), collapse = "\n")))
  dst
}

# Text case: emits expected.json via write_import_case and writes
# records (physical lines, delimiters and quotes included) joined with
# `eol`, in `file_encoding` (an iconv target), with a BOM when `bom`.
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

# Workbook factory: creator is pinned (otherwise the local username
# leaks into dc:creator / lastModifiedBy) and datetime_created is
# fixed so docProps/core.xml is byte-stable.
new_wb <- function() {
  wb <- openxlsx2::wb_workbook(creator = LO_AUTHOR,
                               datetime_created = XLSX_EPOCH_POSIX)
  wb$add_worksheet("Data")
  wb
}

# Spreadsheet case: emits expected.json via write_import_case (which
# clears the case directory first), saves wb, unzips, runs `patch` (a
# function of the extracted directory) when given, then rebuilds the
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

# repack_zip patch: rewrite meta.xml's producer fields (generator,
# creators, dates) to fixed values so an LO-produced ODS carries no
# build or user identity.
normalize_ods_meta <- function(ex) {
  f <- file.path(ex, "meta.xml")
  x <- paste(readLines(f, warn = FALSE), collapse = "")
  fixed <- c("meta:generator" = "Epiora",
             "meta:initial-creator" = LO_AUTHOR,
             "dc:creator" = LO_AUTHOR,
             "meta:creation-date" = LO_META_DATETIME,
             "dc:date" = LO_META_DATETIME)
  for (tag in names(fixed)) {
    pat <- sprintf("<%s>[^<]*</%s>", tag, tag)
    stop_if(!grepl(pat, x),
            sprintf("normalize_ods_meta: <%s> missing in meta.xml", tag))
    x <- sub(pat, sprintf("<%s>%s</%s>", tag, fixed[[tag]], tag), x)
  }
  writeLines(x, f)
}
