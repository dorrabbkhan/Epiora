# SPDX-License-Identifier: MIT
# Extended Mantel-Haenszel chi-square for linear trend over ordered
# exposure levels. Literature: Schlesselman, Case-Control Studies
# (1982) p.203 (uncorrected form, EI7 GetChiSquareForTrend); Rosner,
# Fundamentals of Biostatistics 5th ed. p.606 (continuity correction:
# (|V| - 0.5)^2, floored at 0 — a correction shrinks |V| toward 0 and
# never raises the statistic, so it applies to |V|, not signed V).
#
# `levels` is a list of strata; each stratum is a data.frame with one
# row per exposure level (baseline first) and columns
# score/cases/controls. `correct` picks the Rosner (TRUE) or
# Schlesselman (FALSE) numerator.

xmh_trend <- function(strata, correct = TRUE) {
  vsum <- 0
  v1sum <- 0
  for (s in strata) {
    a <- s$cases      # cases per level
    b <- s$controls   # controls per level
    x <- s$score      # numeric dose score per level
    m <- a + b
    t1 <- sum(a * x)
    t2 <- sum(m * x)
    t3 <- sum(m * x * x)
    n1 <- sum(a)
    n2 <- sum(b)
    n <- n1 + n2
    vsum <- vsum + (n1 * n2 * (n * t3 - t2 * t2)) / (n * n * (n - 1))
    v1sum <- v1sum + t1 - (n1 / n) * t2
  }
  num <- if (correct) max(abs(v1sum) - 0.5, 0) else v1sum
  chisq <- num * num / vsum
  list(value = chisq, p = pchisq(chisq, 1, lower.tail = FALSE))
}

# Mantel-Haenszel summary OR of level i vs the baseline row, pooled
# over strata (per-level 2x2xk table). `strata` as above; returns one
# value per non-baseline level.
dr_mh_or <- function(strata) {
  nlev <- nrow(strata[[1]])
  out <- numeric(nlev - 1)
  for (i in 2:nlev) {
    num <- 0
    den <- 0
    for (s in strata) {
      a <- s$cases[i]; b <- s$controls[i]
      abase <- s$cases[1]; bbase <- s$controls[1]
      ttot <- a + bbase + b + abase
      num <- num + a * bbase / ttot
      den <- den + b * abase / ttot
    }
    out[i - 1] <- num / den
  }
  out
}
