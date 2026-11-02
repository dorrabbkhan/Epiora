# Corpus dataset JSON (spec §3.3) written under fixtures/data/.
# Sourced by generate.R; produces *.dataset.json and parses oswego.csv
# into OSWEGO for the downstream generators.

# Serialise one dataset: columns = named list of list(type, values).
# The column spec is also kept in DATASETS so freq.R can recompute
# categories from the same values that went into the dataset JSON.
DATASETS <- new.env(parent = emptyenv())
write_dataset <- function(id, columns) {
  assign(id, columns, envir = DATASETS)
  ds <- list(
    id = id,
    version = 1L,
    columns = unname(Map(function(name, col) {
      list(name = name, type = col$type, values = col$values)
    }, names(columns), columns))
  )
  out_dir <- file.path(FIXTURES_DIR, "data")
  dir.create(out_dir, showWarnings = FALSE, recursive = TRUE)
  out_file <- file.path(out_dir, paste0(id, ".dataset.json"))
  writeLines(
    jsonlite::toJSON(ds, auto_unbox = TRUE, pretty = TRUE,
                     digits = I(17), na = "null"),
    out_file
  )
  message("wrote ", out_file)
  invisible(out_file)
}

# --- oswego ------------------------------------------------------------

oswego_csv <- read.csv(file.path(FIXTURES_DIR, "data", "oswego.csv"),
                       colClasses = "character", na.strings = "",
                       check.names = FALSE)

OSWEGO_INT <- c("AGE", "ILL", "BAKEDHAM", "SPINACH", "MASHEDPOTA",
                "CABBAGESAL", "JELLO", "ROLLS", "BROWNBREAD", "MILK",
                "COFFEE", "WATER", "CAKES", "VANILLA", "CHOCOLATE",
                "FRUITSALAD")
OSWEGO_TEXT <- c("Name", "SEX", "TIMESUPPER", "DATEONSET",
                 "GlobalRecordId", "CODE_RW")

# Parsed copy for the generators: integer vectors for 0/1 columns and
# AGE, character elsewhere; empty cells are NA.
OSWEGO <- as.data.frame(lapply(names(oswego_csv), function(n) {
  if (n %in% OSWEGO_INT) as.integer(oswego_csv[[n]]) else oswego_csv[[n]]
}), stringsAsFactors = FALSE, check.names = FALSE)
names(OSWEGO) <- names(oswego_csv)

# Sanity check (plan Step 6): VANILLA x ILL must be [[43,11],[3,18]].
osw_table <- function(food) {
  ok <- !is.na(OSWEGO[[food]]) & !is.na(OSWEGO[["ILL"]])
  e <- OSWEGO[[food]][ok] == 1L
  i <- OSWEGO[["ILL"]][ok] == 1L
  matrix(c(sum(e & i), sum(e & !i), sum(!e & i), sum(!e & !i)),
         nrow = 2, byrow = TRUE)
}
stop_if(!identical(osw_table("VANILLA"),
                   matrix(c(43L, 11L, 3L, 18L), nrow = 2, byrow = TRUE)),
        "sanity check failed: VANILLA x ILL is not [[43,11],[3,18]]")

# Per-stratum 2x2 tables of `food` x ILL, ordered by text stratum level
# (spec §3.3 category order: ascending code points).
osw_strata <- function(food, by) {
  ok <- !is.na(OSWEGO[[food]]) & !is.na(OSWEGO[["ILL"]]) &
    !is.na(OSWEGO[[by]])
  levels <- sort(unique(OSWEGO[[by]][ok]), method = "radix")
  strata <- lapply(levels, function(lv) {
    sel <- ok & OSWEGO[[by]] == lv
    e <- OSWEGO[[food]][sel] == 1L
    i <- OSWEGO[["ILL"]][sel] == 1L
    matrix(c(sum(e & i), sum(e & !i), sum(!e & i), sum(!e & !i)),
           nrow = 2, byrow = TRUE)
  })
  list(labels = levels, strata = strata)
}

write_dataset("oswego", lapply(setNames(nm = names(oswego_csv)), function(n) {
  if (n %in% OSWEGO_INT) {
    list(type = "integer", values = as.integer(oswego_csv[[n]]))
  } else {
    list(type = "text", values = oswego_csv[[n]])
  }
}))

# --- synthetic sets (deterministic, no RNG) ------------------------------

# missing: 24 rows; EXPOSED and ILL boolean with missing values, GROUP
# text with missing values. The missing patterns differ per variable so
# per-variable exclusion counts are exercised.
ms_exposed <- rep(c(TRUE, FALSE), 12)
ms_exposed[c(5, 15, 23)] <- NA
ms_ill <- rep(c(FALSE, TRUE), 12)
ms_ill[c(1, 10, 20, 24)] <- NA
ms_group <- rep(c("a", "b", "c"), 8)
ms_group[c(2, 8, 14, 19)] <- NA
write_dataset("missing", list(
  EXPOSED = list(type = "boolean", values = as.logical(ms_exposed)),
  ILL     = list(type = "boolean", values = as.logical(ms_ill)),
  GROUP   = list(type = "text",    values = ms_group)
))

# coding: 24 records coded as boolean E_BOOL/I_BOOL and as integer
# 0/1 E_INT/I_INT; encodings agree cell for cell.
cd_e <- c(rep(TRUE, 10), rep(FALSE, 10), rep(NA, 4))
cd_i <- c(rep(c(TRUE, FALSE), 9), rep(TRUE, 2), rep(NA, 4))
write_dataset("coding", list(
  E_BOOL = list(type = "boolean", values = as.logical(cd_e)),
  I_BOOL = list(type = "boolean", values = as.logical(cd_i)),
  E_INT  = list(type = "integer",
                values = ifelse(is.na(cd_e), NA, as.integer(cd_e))),
  I_INT  = list(type = "integer",
                values = ifelse(is.na(cd_i), NA, as.integer(cd_i)))
))

# text_levels: 24 rows; two-level text T2, three-level text T3, boolean
# ILL. One T2 and one T3 cell are missing.
tl_t2 <- rep(c("a", "b"), 12)
tl_t2[7] <- NA
tl_t3 <- rep(c("x", "y", "z"), 8)
tl_t3[13] <- NA
tl_ill <- rep(c(TRUE, FALSE), 12)
write_dataset("text_levels", list(
  T2  = list(type = "text",    values = tl_t2),
  T3  = list(type = "text",    values = tl_t3),
  ILL = list(type = "boolean", values = as.logical(tl_ill))
))

# strata: 24 rows; boolean exposure/outcome with a three-level text
# stratum S.
st_e <- rep(c(TRUE, FALSE), 12)
st_i <- c(rep(TRUE, 5), rep(FALSE, 5), rep(TRUE, 3), rep(FALSE, 3),
          rep(TRUE, 4), rep(FALSE, 4))
st_s <- rep(c("x", "y", "z"), each = 8)
write_dataset("strata", list(
  EXPOSED = list(type = "boolean", values = as.logical(st_e)),
  ILL     = list(type = "boolean", values = as.logical(st_i)),
  S       = list(type = "text",    values = st_s)
))
