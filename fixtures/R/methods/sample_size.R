# SPDX-License-Identifier: MIT
# Sample-size and power methods reimplemented from the reference
# formulas. Literature: Kelsey et al., Methods in Observational
# Epidemiology 2nd ed., Table 12-15; Fleiss, Statistical Methods for
# Rates and Proportions, formulas 3.18-3.19; Lwanga & Lemeshow (the
# population-survey formula). Formula reference: OpenEpi
# SSPropor.js/SSCC.js/SSCohort.js/SSMean.js/PowerCC.js/PowerCohort.js/
# PowerMean.js and EI7 PopulationSurvey.xaml.cs @ 4cd353c.
#
# The corpus uses exact normal quantiles (qnorm) where the reference
# tools use their own tabled/polynomial approximations; for the integer
# outputs both agree to the ceiling in all but pathological cases.
#
# percents are given as percents (e.g. 95, 80, 40).

ss_za <- function(conf) qnorm((1 + conf / 100) / 2)   # two-sided z
ss_zb <- function(power) qnorm(power / 100)           # one-sided z

# --- population survey ----------------------------------------------

# The seven confidence levels OpenEpi/EI7 display, as percent values.
ss_levels <- c("80", "90", "95", "97", "99", "999", "9999")
ss_level_pct <- c(80, 90, 95, 97, 99, 99.9, 99.99)

# OpenEpi (Lwanga & Lemeshow): ceil(DEFF*N*p(1-p) /
#   ((d^2/z^2)(N-1) + p(1-p))) per level. Folds FPC and DEFF into one
# expression; ignores clusters.
ss_prop_openepi <- function(population_size, expected_freq, margin_error,
                            design_effect) {
  p <- expected_freq / 100
  d <- margin_error / 100
  out <- vapply(ss_level_pct, function(lv) {
    za <- qnorm((1 + lv / 100) / 2)
    ceiling((design_effect * population_size * p * (1 - p)) /
              ((d * d / (za * za)) * (population_size - 1) + p * (1 - p)))
  }, numeric(1))
  setNames(out, ss_levels)
}

# EI7: per level, n0 = z^2*p(1-p)/d^2 (freq/margin entered as percents,
# which cancel); finite-pop correction n = n0/(1 + n0/N); then
# cluster_size = ceil(DEFF*round(n)/clusters) and total =
# clusters*cluster_size. Returns list of named vectors.
ss_prop_ei7 <- function(population_size, expected_freq, margin_error,
                        design_effect, clusters) {
  factor <- expected_freq * (100 - expected_freq) / (margin_error^2)
  n_raw <- vapply(ss_level_pct, function(lv) {
    za <- qnorm((1 + lv / 100) / 2)
    n0 <- za * za * factor
    round(n0 / (1 + n0 / population_size))
  }, numeric(1))
  cluster_size <- ceiling(design_effect * n_raw / clusters)
  list(
    n_ei7 = setNames(cluster_size, ss_levels),
    cluster_size = setNames(cluster_size, ss_levels),
    total = setNames(clusters * cluster_size, ss_levels)
  )
}

# --- two proportions (Kelsey / Fleiss) -------------------------------
# Shared by ss_unmatched_cc and ss_cohort; `group1`/`group2` are the
# case/exposed and control/unexposed sides respectively.

ss_pbar <- function(p1, p2, r) (p1 + r * p2) / (1 + r)

# Kelsey: n1 = (Za+Zb)^2 * pbar*qbar*(r+1) / ((p1-p2)^2 * r).
kelsey <- function(conf, power, r, p2, p1) {
  za <- ss_za(conf); zb <- ss_zb(power)
  pbar <- ss_pbar(p1, p2, r); qbar <- 1 - pbar
  vn <- (za + zb)^2 * pbar * qbar * (r + 1) / ((p1 - p2)^2 * r)
  n1 <- ceiling(vn)
  n2 <- ceiling(vn * r)
  c(group1 = n1, group2 = n2, total = n1 + n2)
}

# Fleiss: n1 = (Za*sqrt((r+1)pbar qbar) + Zb*sqrt(r*p1*q1 + p2*q2))^2 /
#   (r*(p2-p1)^2).
fleiss <- function(conf, power, r, p2, p1) {
  za <- ss_za(conf); zb <- ss_zb(power)
  pbar <- ss_pbar(p1, p2, r); qbar <- 1 - pbar
  vn <- (za * sqrt((r + 1) * pbar * qbar) +
           zb * sqrt(r * p1 * (1 - p1) + p2 * (1 - p2)))^2 /
    (r * (p2 - p1)^2)
  n1 <- ceiling(vn)
  n2 <- ceiling(vn * r)
  c(group1 = n1, group2 = n2, total = n1 + n2)
}

# Fleiss with continuity correction: same base n then
# n' = n*(1 + sqrt(1 + 2*(r+1)/(n*r*|p2-p1|)))^2/4.
fleiss_cc <- function(conf, power, r, p2, p1) {
  za <- ss_za(conf); zb <- ss_zb(power)
  pbar <- ss_pbar(p1, p2, r); qbar <- 1 - pbar
  vn <- (za * sqrt((r + 1) * pbar * qbar) +
           zb * sqrt(r * p1 * (1 - p1) + p2 * (1 - p2)))^2 /
    (r * (p2 - p1)^2)
  vn <- vn * (1 + sqrt(1 + 2 * (r + 1) / (vn * r * abs(p2 - p1))))^2 / 4
  n1 <- ceiling(vn)
  n2 <- ceiling(vn * r)
  c(group1 = n1, group2 = n2, total = n1 + n2)
}

# --- mean difference -------------------------------------------------

# z-based two-sample size: n1 = (za+zb)^2 * (s1^2 + s2^2/ratio) /
#   diff^2; n2 = ceil(n1*ratio); OpenEpi SSMean.js.
ss_means <- function(conf, power, ratio, diff, s1, s2) {
  za <- ss_za(conf); zb <- ss_zb(power)
  n1 <- ceiling((s1^2 + s2^2 / ratio) * (za + zb)^2 / diff^2)
  n2 <- ceiling(n1 * ratio)
  c(n1 = n1, n2 = n2, total = n1 + n2)
}

# --- power -----------------------------------------------------------
# OpenEpi convention: power % = (1 - Q(|z_pow|)/2) * 100 where Q is the
# normal upper tail; i.e. 50 + 50*Phi(|z_pow|), with the absolute value
# matching OE's Norm(|x|).

power_norm <- function(powerz) (1 - pnorm(abs(powerz), lower.tail = FALSE) / 2) * 100

# Unmatched case-control, normal approximation.
power_cc <- function(n_cases, n_controls, pct_cases, pct_controls, conf) {
  z <- ss_za(conf)
  p1 <- pct_cases / 100; p2 <- pct_controls / 100
  r <- n_controls / n_cases
  q1 <- 1 - p1; q2 <- 1 - p2
  d <- p1 - p2
  pbar <- (p1 + r * p2) / (1 + r); qbar <- 1 - pbar
  powerz <- (sqrt(n_cases * d * d) - z * sqrt((1 + 1 / r) * pbar * qbar)) /
    sqrt(p1 * q1 + p2 * q2 / r)
  power_norm(powerz)
}

# Same with continuity correction: n1 adjusted by (r+1)/(r*d), sign by
# whether the OR exceeds 1.
power_cc_cc <- function(n_cases, n_controls, pct_cases, pct_controls, conf) {
  z <- ss_za(conf)
  p1 <- pct_cases / 100; p2 <- pct_controls / 100
  r <- n_controls / n_cases
  q1 <- 1 - p1; q2 <- 1 - p2
  d <- p1 - p2
  or <- implied_or(pct_cases, pct_controls)
  pbar <- (p1 + r * p2) / (1 + r); qbar <- 1 - pbar
  nn1 <- if (or > 1) {
    abs(n_cases - (r + 1) / (r * d))
  } else {
    abs(n_cases + (r + 1) / (r * d))
  }
  powerz <- (sqrt(nn1 * d * d) - z * sqrt((1 + 1 / r) * pbar * qbar)) /
    sqrt(p1 * q1 + p2 * q2 / r)
  power_norm(powerz)
}

# Cohort/RCT/cross-sectional (same computation): pbar weighted by raw
# counts and the denominator uses p2q2 + p1q1/r.
power_cohort <- function(n_exposed, n_unexposed, pct_exposed,
                         pct_unexposed, conf) {
  z <- ss_za(conf)
  p1 <- pct_exposed / 100; p2 <- pct_unexposed / 100
  r <- n_unexposed / n_exposed
  q1 <- 1 - p1; q2 <- 1 - p2
  d <- p1 - p2
  pbar <- (p1 * n_exposed + p2 * n_unexposed) / (n_exposed + n_unexposed)
  qbar <- 1 - pbar
  powerz <- (sqrt(n_exposed * d * d) - z * sqrt((1 + 1 / r) * pbar * qbar)) /
    sqrt(p2 * q2 + p1 * q1 / r)
  power_norm(powerz)
}

power_cohort_cc <- function(n_exposed, n_unexposed, pct_exposed,
                            pct_unexposed, conf) {
  z <- ss_za(conf)
  p1 <- pct_exposed / 100; p2 <- pct_unexposed / 100
  r <- n_unexposed / n_exposed
  q1 <- 1 - p1; q2 <- 1 - p2
  d <- p1 - p2
  rr <- p1 / p2
  pbar <- (p1 * n_exposed + p2 * n_unexposed) / (n_exposed + n_unexposed)
  qbar <- 1 - pbar
  nn1 <- if (rr > 1) {
    abs(n_exposed - (r + 1) / (r * d))
  } else {
    abs(n_exposed + (r + 1) / (r * d))
  }
  powerz <- (sqrt(nn1 * d * d) - z * sqrt((1 + 1 / r) * pbar * qbar)) /
    sqrt(p2 * q2 + p1 * q1 / r)
  power_norm(powerz)
}

# Mean difference power: z_pow = -za + sqrt(n1)*diff /
#   sqrt(s1^2 + s2^2/ratio).
power_means <- function(n1, n2, diff, s1, s2, conf) {
  za <- ss_za(conf)
  ratio <- n2 / n1
  powerz <- -za + (sqrt(n1) * diff) / sqrt(s1^2 + s2^2 / ratio)
  power_norm(powerz)
}

# --- derived inputs ---------------------------------------------------

# Implied OR from the two percent exposures.
implied_or <- function(pct_cases, pct_controls) {
  p1 <- pct_cases / 100; p2 <- pct_controls / 100
  (p1 * (1 - p2)) / (p2 * (1 - p1))
}

# Implied RR and RD from the two percent outcomes.
implied_rr_rd <- function(pct_exposed, pct_unexposed) {
  p1 <- pct_exposed / 100; p2 <- pct_unexposed / 100
  c(rr = p1 / p2, rd = p1 - p2)
}
