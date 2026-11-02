# fixtures/2x2/*.json — two_by_two fixtures (plan Step 8).
# Table layout: [[a,b],[c,d]] rows exposed/unexposed, columns ill/well.
# epitools' convention is reference row first / non-event column first,
# so rev = "both" maps this layout onto it; measure row 2 is the
# comparison row.

# Replace every leaf in `x` with a null leaf of the same class and drop
# r_default (used for all_zero: the reference says every statistic is
# undefined).
nullify <- function(x) {
  if (is.list(x) && identical(sort(names(x)[names(x) != "r_default"]),
                             c("class", "value"))) {
    return(list(value = NA, class = x$class))
  }
  if (is.list(x)) return(lapply(x, nullify))
  x
}

# CI pieces that need root-finding: returns list(cmle, fisher_ci,
# midp_ci, midp_p) — each element finite or NA — and fills r_defaults.
# `tab` is the 2x2 matrix; `id` only labels check_tight failures.
twobytwo_iterative <- function(tab, id) {
  fis <- fisher.test(tab)
  midp_r <- tryCatch(
    oddsratio(tab, rev = "both", method = "midp")$measure[2, ],
    error = function(e) c(estimate = NA, lower = NA, upper = NA))
  midp_r_p <- tryCatch(
    suppressWarnings(
      epitools:::tab2by2.test(tab, rev = "both")$p.value[2, "midp.exact"]),
    error = function(e) NA)

  h <- nch_setup(tab)
  cmle_t <- tryCatch(nch_cmle(h), error = function(e) NA)
  check_tight(cmle_t, unname(fis$estimate),
              paste0(id, " odds_ratio.cmle"))
  ci_t <- tryCatch(nch_fisher_ci(h), error = function(e) c(NA, NA))
  check_tight(ci_t[1], unname(fis$conf.int[1]),
              paste0(id, " odds_ratio.ci_95_fisher_exact.lower"))
  check_tight(ci_t[2], unname(fis$conf.int[2]),
              paste0(id, " odds_ratio.ci_95_fisher_exact.upper"))
  midp_t <- tryCatch(nch_midp_ci(h), error = function(e) c(NA, NA))
  check_tight(midp_t[1], unname(midp_r["lower"]),
              paste0(id, " odds_ratio.ci_95_midp_exact.lower"))
  check_tight(midp_t[2], unname(midp_r["upper"]),
              paste0(id, " odds_ratio.ci_95_midp_exact.upper"))
  midp_p <- nch_midp_p_two_sided(h)
  check_tight(midp_p, unname(midp_r_p), paste0(id, " midp_exact.p_two_sided"))

  list(
    cmle = leaf(cmle_t, "iterative", r_default = unname(fis$estimate)),
    fisher_ci = list(
      lower = leaf(ci_t[1], "iterative", r_default = unname(fis$conf.int[1])),
      upper = leaf(ci_t[2], "iterative", r_default = unname(fis$conf.int[2]))),
    midp_ci = list(
      lower = leaf(midp_t[1], "iterative", r_default = unname(midp_r["lower"])),
      upper = leaf(midp_t[2], "iterative", r_default = unname(midp_r["upper"]))),
    # The mid-p two-sided p is a direct pnhyper sum (no root), so it is
    # closed_form like the Fisher p-values.
    midp_p = leaf(midp_p)
  )
}

# Full expected object for a 2x2 table.
twobytwo_expected <- function(tab, id) {
  a <- tab[1, 1]; b <- tab[1, 2]; cc <- tab[2, 1]; d <- tab[2, 2]
  n1 <- a + b; n0 <- cc + d
  risk_e <- a / n1
  risk_u <- cc / n0
  rd <- risk_e - risk_u
  se_rd <- sqrt(risk_e * (1 - risk_e) / n1 + risk_u * (1 - risk_u) / n0)
  z <- qnorm(0.975)

  rr  <- riskratio(tab, rev = "both")
  rrs <- riskratio.small(tab, rev = "both")
  orw <- oddsratio(tab, rev = "both", method = "wald")
  chi_u <- suppressWarnings(chisq.test(tab, correct = FALSE))
  chi_y <- suppressWarnings(chisq.test(tab, correct = TRUE))
  # MH (N-1): stock mantelhaen.test refuses one stratum, so the
  # verbatim port in common.R supplies the same computation.
  mh <- mh_2x2xk(array(tab, dim = c(2, 2, 1)), correct = FALSE)
  # Single-table identity check on the port's MH chi-square (fix
  # round 2; skipped when the statistic is undefined).
  check_mh_single_table(tab, mh$statistic,
                        paste0(id, " chi_square.mantel_haenszel"))
  it <- twobytwo_iterative(tab, id)

  list(
    risk = list(
      exposed = leaf(risk_e),
      unexposed = leaf(risk_u),
      difference = leaf(rd),
      difference_ci_95_wald = list(
        lower = leaf(rd - z * se_rd),
        upper = leaf(rd + z * se_rd))
    ),
    risk_ratio = list(
      estimate = leaf(rr$measure[2, "estimate"]),
      ci_95_wald = list(
        lower = leaf(rr$measure[2, "lower"]),
        upper = leaf(rr$measure[2, "upper"])),
      small_sample = list(
        estimate = leaf(rrs$measure[2, "estimate"]),
        lower = leaf(rrs$measure[2, "lower"]),
        upper = leaf(rrs$measure[2, "upper"]))
    ),
    odds_ratio = list(
      estimate = leaf(orw$measure[2, "estimate"]),
      cmle = it$cmle,
      ci_95_wald = list(
        lower = leaf(orw$measure[2, "lower"]),
        upper = leaf(orw$measure[2, "upper"])),
      ci_95_fisher_exact = it$fisher_ci,
      ci_95_midp_exact = it$midp_ci
    ),
    chi_square = list(
      df = leaf(chi_u$parameter),
      uncorrected = list(
        statistic = leaf(chi_u$statistic),
        p_value = leaf(chi_u$p.value)),
      yates = list(
        statistic = leaf(chi_y$statistic),
        p_value = leaf(chi_y$p.value)),
      mantel_haenszel = list(
        statistic = leaf(mh$statistic),
        p_value = leaf(mh$p.value))
    ),
    fisher_exact = list(
      p_lower = leaf(fisher.test(tab, alternative = "less")$p.value),
      p_upper = leaf(fisher.test(tab, alternative = "greater")$p.value),
      p_two_sided = leaf(fisher.test(tab)$p.value)
    ),
    midp_exact = list(p_two_sided = it$midp_p)
  )
}

# Write one two_by_two fixture. `inputs` optionally carries
# dataset/exposure/outcome for Oswego cases.
twobytwo_write <- function(id, tab, inputs = list()) {
  inputs$table <- tab
  expected <- if (all(tab == 0)) {
    nullify(twobytwo_expected(osw_table("VANILLA"), id))
  } else {
    twobytwo_expected(tab, id)
  }
  write_fixture("2x2", id, "two_by_two", inputs, expected,
                packages = c("stats", "epitools"))
}

twobytwo_write("oswego_vanilla", osw_table("VANILLA"),
               list(dataset = "oswego", exposure = "VANILLA",
                    outcome = "ILL"))
twobytwo_write("oswego_chocolate", osw_table("CHOCOLATE"),
               list(dataset = "oswego", exposure = "CHOCOLATE",
                    outcome = "ILL"))
twobytwo_write("oswego_fruitsalad", osw_table("FRUITSALAD"),
               list(dataset = "oswego", exposure = "FRUITSALAD",
                    outcome = "ILL"))
twobytwo_write("oswego_brownbread", osw_table("BROWNBREAD"),
               list(dataset = "oswego", exposure = "BROWNBREAD",
                    outcome = "ILL"))

# One zero in each position; the other cells are 5, 7, 9.
twobytwo_write("zero_a", matrix(c(0, 5, 7, 9), 2, 2, byrow = TRUE))
twobytwo_write("zero_b", matrix(c(5, 0, 7, 9), 2, 2, byrow = TRUE))
twobytwo_write("zero_c", matrix(c(5, 7, 0, 9), 2, 2, byrow = TRUE))
twobytwo_write("zero_d", matrix(c(5, 7, 9, 0), 2, 2, byrow = TRUE))
twobytwo_write("all_zero", matrix(c(0, 0, 0, 0), 2, 2, byrow = TRUE))
twobytwo_write("large", matrix(c(1e6, 1e6, 1e6, 1e6), 2, 2, byrow = TRUE))
