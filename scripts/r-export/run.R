# scripts/r-export/run.R — runs every `analysis.R` under the
# export-corpus output directory (v1.0 Task 17, spec §8.3). Each
# script's stdout goes to `<case>/out.txt` for
# `scripts/r-export/compare.mjs`; a non-zero script exit fails CI.
#
# Usage: Rscript scripts/r-export/run.R <out-dir>

args <- commandArgs(trailingOnly = TRUE)
stopifnot(length(args) == 1)
root <- normalizePath(args[1], mustWork = TRUE)

# The generated scripts call `library(pkg)` on the renv-pinned
# packages; the per-case `Rscript` children inherit this process's
# environment, so point `R_LIBS_USER` at the restored renv library
# (renv/library/<platform>/<R-ver>/<arch>) when it exists — a no-op
# when the lockfile env is absent and dev machines set `R_LIBS`
# themselves.
lib_root <- "fixtures/renv/library"
if (dir.exists(lib_root)) {
  libs <- list.dirs(lib_root, recursive = TRUE, full.names = TRUE)
  renv_lib <- libs[file.exists(file.path(libs, "renv"))]
  r_ver <- sprintf("/R-%s.%s/", R.version$major, sub("\\..*", "", R.version$minor))
  renv_lib <- renv_lib[grepl(r_ver, paste0(renv_lib, "/"), fixed = TRUE)]
  if (length(renv_lib) > 0) Sys.setenv(R_LIBS_USER = normalizePath(renv_lib[[1]]))
}

cases <- sort(list.dirs(root, recursive = TRUE, full.names = TRUE))
cases <- cases[file.exists(file.path(cases, "analysis.R"))]
stopifnot(length(cases) > 0)

failed <- character(0)
for (dir in cases) {
  out <- file.path(dir, "out.txt")
  # run inside `dir` so `read.csv("data.csv")` resolves there.
  owd <- setwd(dir)
  status <- system2("Rscript",
                    c("--vanilla", shQuote("analysis.R")),
                    stdout = out, stderr = out)
  setwd(owd)
  if (status != 0) {
    failed <- c(failed, sub(paste0("^", root, "/?"), "", dir))
  }
}
if (length(failed) > 0) {
  cat("r-export: scripts failed:\n")
  cat(paste0("  ", failed, "\n"), sep = "")
  quit(status = 1)
}
cat(sprintf("r-export: ran %d scripts\n", length(cases)))
