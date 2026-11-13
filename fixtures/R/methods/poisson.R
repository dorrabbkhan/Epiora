# SPDX-License-Identifier: MIT
# Poisson/SMR statistics not available as plain R calls.
# Literature: Rothman & Boice, Epidemiologic Analysis with a
# Programmable Calculator, 1979 (Byar); Rothman & Greenland, Modern
# Epidemiology 2e (log method); Ury & Wiggins, Am J Epidemiol
# 1985;122:197-8; Vandenbroucke, Am J Epidemiol 1982;115:303-4.
# Formula reference: OpenEpi SMR.js.

# Byar's approximation to the Poisson chi-square test. `v` is the
# continuity-adjusted count: observed when O > E, else O + 1. Returns
# the signed normal deviate squared (the chi-square statistic) and the
# two-sided p, matching OpenEpi's Norm(x2) = 2 * Q(|x2|).
byar_stat <- function(o, e) {
  v <- if (o > e) o else o + 1
  x2 <- sqrt(9 * v) * (1 - 1 / (9 * v) - (e / v)^(1 / 3))
  list(chisq = x2^2, p = pchisq(x2^2, 1, lower.tail = FALSE))
}

# Rothman/Greenland (Boice-Monson) log method for the SMR CI.
rg_ci <- function(o, e, conf) {
  z <- qnorm((1 + conf / 100) / 2)
  lower <- exp(log(o / e) - z / sqrt(o))
  upper <- exp(log(o / e) + z / sqrt(o))
  c(lower = lower, upper = upper)
}

# Ury & Wiggins shortcut CI; defined only at 90/95/99 (NA elsewhere).
ury_wiggins <- function(o, e, conf) {
  z <- qnorm((1 + conf / 100) / 2)
  add <- if (o > 50) 0 else 0.1
  if (conf == 99) {
    c(lower = (o - z * sqrt(o) + 2) / e,
      upper = (o + z * sqrt(o) + 3 + add) / e)
  } else if (conf == 95) {
    c(lower = (o - z * sqrt(o) + 1) / e,
      upper = (o + z * sqrt(o) + 2 + add) / e)
  } else if (conf == 90) {
    c(lower = (o - z * sqrt(o) + 0.65) / e,
      upper = (o + z * sqrt(o) + 1.65 + add) / e)
  } else {
    c(lower = NA_real_, upper = NA_real_)
  }
}

# Vandenbroucke 95% shortcut; NA at any other level.
vandenbroucke <- function(o, e, conf) {
  if (conf != 95) return(c(lower = NA_real_, upper = NA_real_))
  z <- qnorm(0.975)
  c(lower = (sqrt(o) - z * 0.5)^2 / e,
    upper = (sqrt(o) + z * 0.5)^2 / e)
}
