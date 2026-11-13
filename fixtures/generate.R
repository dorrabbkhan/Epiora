#!/usr/bin/env Rscript
# Regenerates the reference fixtures under fixtures/ (v0.2 Task 2,
# spec §6.2). Run from the repo root:  Rscript fixtures/generate.R
# Fixtures are never hand-edited (AGENTS.md rule 1); re-run this
# script instead. Sources fixtures/R/*.R in dependency order:
# helpers -> data -> generators.

# Locate this script's directory so the renv project activates
# regardless of the caller's working directory.
file_arg <- grep("^--file=", commandArgs(trailingOnly = FALSE), value = TRUE)
script_dir <- if (length(file_arg) > 0L) {
  dirname(normalizePath(sub("^--file=", "", file_arg[length(file_arg)])))
} else {
  getwd()
}
# activate.R resolves the project from getwd(); pin it to this script's
# directory so the script works from any caller working directory.
Sys.setenv(RENV_PROJECT = script_dir)
source(file.path(script_dir, "renv", "activate.R"))

FIXTURES_DIR <- script_dir

# epitools is used unqualified by the generators (riskratio,
# oddsratio, riskratio.small); epiR and DescTools are used qualified.
library(epitools)

# v1.0 Task 2: method helpers (methods/*.R) load before every
# generator that calls them; means.R and calc.R run after the
# generators they depend on; import.R stays last.
method_files <- sort(list.files(file.path(FIXTURES_DIR, "R", "methods"),
                                pattern = "\\.R$", full.names = TRUE))
for (f in c("common.R", "nchyper.R", "data.R", "special.R")) {
  source(file.path(FIXTURES_DIR, "R", f))
}
for (f in method_files) {
  source(f)
}
for (f in c("twobytwo.R", "stratified.R", "rxc.R", "freq.R",
            "means.R", "calc.R", "import.R")) {
  source(file.path(FIXTURES_DIR, "R", f))
}
