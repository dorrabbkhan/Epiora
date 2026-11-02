# fixtures/stratified/*.json — stratified_two_by_two fixtures (plan
# Step 9). Strata are 2x2 matrices [[a,b],[c,d]] laid out like
# twobytwo.R. All-zero strata are excluded before the references run
# and their 0-based positions are recorded in expected.excluded_strata
# (review focus 1: listed, never silently dropped).

# Full expected object for a stratified 2x2 case.
stratified_expected <- function(strata, labels, id) {
  stop_if(length(strata) != length(labels),
          sprintf("%s: strata/labels length mismatch", id))
  keep <- vapply(strata, function(s) sum(s) > 0, logical(1))
  kept <- strata[keep]
  excluded <- which(!keep) - 1L
  n <- length(kept)
  stop_if(n < 1L, sprintf("%s: no non-empty strata", id))

  crude <- twobytwo_expected(Reduce(`+`, kept), paste0(id, " crude"))
  arr <- array(unlist(kept), dim = c(2, 2, n))

  # MH odds ratio, its RBG CI and the MH chi-squares: mantelhaen.test
  # for >= 2 strata (with a bitwise port check); the verbatim port in
  # common.R for a single stratum, which stock refuses. The common-OR
  # estimate and CI do not depend on the continuity correction, so
  # they are taken from the uncorrected run.
  if (n >= 2L) {
    check_mh_port(arr, id)
    mh_u <- mantelhaen.test(arr, correct = FALSE)
    mh_c <- mantelhaen.test(arr, correct = TRUE)
  } else {
    mh_u <- mh_2x2xk(arr, correct = FALSE)
    mh_c <- mh_2x2xk(arr, correct = TRUE)
  }

  # MH risk ratio with Greenland-Robins CI: epiR cohort.count. For a
  # single stratum epiR's array path errors, and the pooled MH estimate
  # is the stratum's own, so the per-stratum Wald result is used.
  rr <- if (n == 1L) {
    epiR::epi.2by2(kept[[1]], method = "cohort.count",
                   outcome = "as.columns")$massoc.detail$RR.strata.wald
  } else {
    epiR::epi.2by2(arr, method = "cohort.count",
                   outcome = "as.columns")$massoc.detail$RR.mh.wald
  }

  # Breslow-Day-Tarone: undefined with fewer than 2 kept strata
  # (DescTools errors; the leaves record it as undefined).
  bd <- tryCatch(
    DescTools::BreslowDayTest(arr, correct = TRUE),
    error = function(e) NULL)

  list(
    crude = crude,
    mantel_haenszel = list(
      odds_ratio = list(
        estimate = leaf(unname(mh_u$estimate)),
        ci_95 = list(
          lower = leaf(unname(mh_u$conf.int[1])),
          upper = leaf(unname(mh_u$conf.int[2])))),
      risk_ratio = list(
        estimate = leaf(rr$est),
        ci_95 = list(
          lower = leaf(rr$lower),
          upper = leaf(rr$upper))),
      chi_square = list(
        uncorrected = list(
          statistic = leaf(unname(mh_u$statistic)),
          p_value = leaf(mh_u$p.value)),
        corrected = list(
          statistic = leaf(unname(mh_c$statistic)),
          p_value = leaf(mh_c$p.value)))),
    breslow_day_tarone = list(
      statistic = leaf(if (is.null(bd)) NA else unname(bd$statistic)),
      df = leaf(if (is.null(bd)) NA else unname(bd$parameter)),
      p_value = leaf(if (is.null(bd)) NA else bd$p.value)),
    excluded_strata = lapply(excluded, function(i) leaf(i)))
}

# Write one stratified fixture. `inputs` optionally carries
# dataset/exposure/outcome/stratify_by for Oswego cases.
stratified_write <- function(id, strata, labels, inputs = list()) {
  inputs$strata <- strata
  # I() keeps a length-1 labels vector a JSON array under auto_unbox.
  inputs$labels <- I(labels)
  write_fixture("stratified", id, "stratified_two_by_two", inputs,
                stratified_expected(strata, labels, id),
                packages = c("stats", "epitools", "epiR", "DescTools"))
}

# Oswego VANILLA x ILL by SEX.
osw_sex <- osw_strata("VANILLA", "SEX")
stratified_write("oswego_vanilla_by_sex", osw_sex$strata, osw_sex$labels,
                 list(dataset = "oswego", exposure = "VANILLA",
                      outcome = "ILL", stratify_by = "SEX"))

# One stratum: MH values are the stratum's own; Breslow-Day is
# undefined (no degrees of freedom).
stratified_write("one_stratum",
                 list(matrix(c(10, 5, 3, 12), 2, 2, byrow = TRUE)),
                 "a")

# Three strata with the middle one all zero: the reference is computed
# on strata 1 and 3 and stratum index 1 is listed as excluded.
stratified_write("empty_stratum",
                 list(matrix(c(12, 4, 3, 9), 2, 2, byrow = TRUE),
                      matrix(c(0, 0, 0, 0), 2, 2, byrow = TRUE),
                      matrix(c(8, 6, 4, 10), 2, 2, byrow = TRUE)),
                 c("a", "b", "c"))

# Heterogeneous ORs (about 0.2 and 10): the plan asserts Breslow-Day
# rejects homogeneity at p < 0.01.
het_strata <- list(matrix(c(18, 36, 36, 16), 2, 2, byrow = TRUE),
                   matrix(c(32, 10, 10, 32), 2, 2, byrow = TRUE))
het_bd <- DescTools::BreslowDayTest(
  array(unlist(het_strata), dim = c(2, 2, 2)), correct = TRUE)
stop_if(het_bd$p.value >= 0.01,
        sprintf("heterogeneous: Breslow-Day p %.4g is not < 0.01",
                het_bd$p.value))
stratified_write("heterogeneous", het_strata, c("a", "b"))
