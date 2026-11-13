# SPDX-License-Identifier: MIT
# Continuous-summary statistics not available as plain R calls.
# Literature: Cochran, Sampling Techniques 1977 §2.8 (mean CI with
# finite-population correction, as in OpenEpi CIMean.js); Conover,
# Practical Nonparametric Statistics (normal-approximation CI for a
# percentile as order-statistic ranks, as in OpenEpi CIMedian.js);
# one-way ANOVA and Bartlett 1937 from summary data (OpenEpi ANOVA.js).

# z-based CI for a mean; applies the FPC sqrt((N-n)/(N-1)) when `pop`
# is finite (a finite population size, not the "infinite" sentinel).
mean_ci <- function(mean, sd, n, conf, pop) {
  z <- qnorm((1 + conf / 100) / 2)
  se <- sd / sqrt(n)
  if (is.finite(pop)) se <- se * sqrt((pop - n) / (pop - 1))
  c(lower = mean - z * se, upper = mean + z * se)
}

# Normal-approximation CI for a percentile expressed as order-statistic
# ranks: np +/- z*sqrt(n p q), where p is the percentile fraction.
percentile_ci <- function(n, percentile, conf) {
  z <- qnorm((1 + conf / 100) / 2)
  p <- percentile / 100
  mid <- n * p
  half <- z * sqrt(n * p * (1 - p))
  c(lower_rank = mid - half, upper_rank = mid + half)
}

# One-way ANOVA from summary data. `tab` is a k x 3 matrix/data.frame
# with columns n, mean, sd.
anova_from_summary <- function(tab) {
  n <- tab[, 1]; m <- tab[, 2]; s <- tab[, 3]
  k <- nrow(tab)
  n_tot <- sum(n)
  grand <- sum(n * m) / n_tot
  ss_b <- sum(n * (m - grand)^2)
  ss_w <- sum((n - 1) * s^2)
  df_b <- k - 1; df_w <- n_tot - k
  ms_b <- ss_b / df_b; ms_w <- ss_w / df_w
  f <- ms_b / ms_w
  list(ss_between = ss_b, df_between = df_b, ms_between = ms_b,
       ss_within = ss_w, df_within = df_w, ms_within = ms_w,
       f = f, p = pf(f, df_b, df_w, lower.tail = FALSE),
       ss_total = ss_b + ss_w, df_total = n_tot - 1)
}

# Bartlett's test for equality of variance from summary data (same
# algebra as stats::bartlett.test on raw data).
bartlett_from_summary <- function(tab) {
  n <- tab[, 1]; s <- tab[, 3]
  k <- nrow(tab)
  v <- n - 1
  n_tot <- sum(n)
  sp2 <- sum(v * s^2) / (n_tot - k)
  chisq <- ((n_tot - k) * log(sp2) - sum(v * log(s^2))) /
    (1 + (sum(1 / v) - 1 / (n_tot - k)) / (3 * (k - 1)))
  list(chisq = chisq, df = k - 1,
       p = pchisq(chisq, k - 1, lower.tail = FALSE))
}
