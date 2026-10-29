#!/usr/bin/env Rscript
# Regenerates the reference fixtures under fixtures/ (v0.1 Task 5, spec §12.1).
# Run from the repo root:  Rscript fixtures/generate.R
# Fixtures are never hand-edited (AGENTS.md rule 1); re-run this script instead.

# Locate this script's directory so the renv project activates regardless of
# the caller's working directory.
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

library(epitools)
library(jsonlite)

## Oswego outbreak, 2x2 table: ate vanilla ice cream x ill (plan Task 5).
##                 ill  well
##   ate            43    11   attack rate 79.6%
##   did not eat     3    18   attack rate 14.3%
## epitools' table convention is reference row first / non-event column
## first, so rev = "both" maps this conventional layout (exposed first
## row, event first column) onto it.
tab <- matrix(c(43, 11, 3, 18), nrow = 2, byrow = TRUE,
              dimnames = list(vanilla = c("ate", "did_not_eat"),
                              outcome = c("ill", "well")))

rr  <- riskratio(tab, rev = "both")
rrs <- riskratio.small(tab, rev = "both")
orw <- oddsratio(tab, rev = "both", method = "wald")
orm <- oddsratio(tab, rev = "both", method = "midp")
orf <- oddsratio(tab, rev = "both", method = "fisher")
chi <- chisq.test(tab, correct = FALSE)
fis <- fisher.test(tab)

## Row 2 of $measure is the comparison row (ate vs did_not_eat); row 1 is
## the reference. Plan Step 4: stop if the risk ratio rounded to 2 decimals
## is not 5.57.
rr_est <- rr$measure[2, "estimate"]
if (round(rr_est, 2) != 5.57) {
  stop("sanity check failed: risk ratio rounded to 2 decimals is ",
       round(rr_est, 2), ", expected 5.57")
}

fixture <- list(
  id = "oswego_vanilla",
  procedure = "2x2",
  inputs = list(
    exposure = "ate_vanilla_ice_cream",
    outcome = "ill",
    exposed_ill = 43L,
    exposed_well = 11L,
    unexposed_ill = 3L,
    unexposed_well = 18L
  ),
  expected = list(
    attack_rate = list(
      exposed = unname(tab[1, 1] / sum(tab[1, ])),
      unexposed = unname(tab[2, 1] / sum(tab[2, ]))
    ),
    risk_ratio = list(
      estimate = unname(rr_est),
      ci_95_wald = list(
        lower = unname(rr$measure[2, "lower"]),
        upper = unname(rr$measure[2, "upper"])
      ),
      ci_95_small_sample = list(
        lower = unname(rrs$measure[2, "lower"]),
        upper = unname(rrs$measure[2, "upper"])
      )
    ),
    odds_ratio = list(
      estimate = unname(orw$measure[2, "estimate"]),
      ci_95_wald = list(
        lower = unname(orw$measure[2, "lower"]),
        upper = unname(orw$measure[2, "upper"])
      ),
      ci_95_midp_exact = list(
        lower = unname(orm$measure[2, "lower"]),
        upper = unname(orm$measure[2, "upper"])
      ),
      ci_95_fisher_exact = list(
        lower = unname(orf$measure[2, "lower"]),
        upper = unname(orf$measure[2, "upper"])
      )
    ),
    chi_square_uncorrected = list(
      statistic = unname(chi$statistic),
      df = unname(chi$parameter),
      p_value = unname(chi$p.value)
    ),
    fisher_exact_test = list(
      p_value = unname(fis$p.value),
      odds_ratio_estimate = unname(fis$estimate),
      ci_95 = list(
        lower = unname(fis$conf.int[1]),
        upper = unname(fis$conf.int[2])
      )
    )
  ),
  tolerance = 1e-9,
  source = list(
    package = "epitools",
    version = utils::packageDescription("epitools")$Version,
    r_version = R.version.string
  )
)

out_dir <- file.path(script_dir, "2x2")
dir.create(out_dir, showWarnings = FALSE, recursive = TRUE)
out_file <- file.path(out_dir, "oswego_vanilla.json")
# digits = I(17): 17 significant digits, so every double round-trips
# exactly and the 1e-9 tolerance stays meaningful.
writeLines(toJSON(fixture, auto_unbox = TRUE, pretty = TRUE, digits = I(17)),
           out_file)
message("wrote ", out_file)
