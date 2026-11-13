# SPDX-License-Identifier: MIT
# One-sample proportion intervals not available as plain R calls.
# Literature: Wilson 1927 (score interval); Fleiss, Statistical Methods
# for Rates and Proportions (score with continuity correction); Cochran,
# Sampling Techniques 1977 (finite-population correction). Formula
# reference: OpenEpi Proportion.js.

# Two-sided normal deviate for a confidence level given in percent.
# (OpenEpi uses sqrt of the chi-square(1) critical; identical.)
oe_z <- function(conf) qnorm((1 + conf / 100) / 2)

# Wilson score CI with finite-population correction. OE applies the FPC
# sqrt((N - n)/(N - 1)) to the score half-width only when 2 < N < 1e6;
# callers emit this CI only under that condition.
wilson_fpc_ci <- function(x, n, conf, pop) {
  z <- oe_z(conf)
  zsq <- z * z
  p <- x / n
  fpc <- sqrt((pop - n) / (pop - 1))
  r <- sqrt(x * (n - x) / (n^3) + zsq / (4 * n^2))
  lower <- if (p == 0) 0 else
    (n / (n + zsq)) * ((p + zsq / (2 * n)) - fpc * z * r)
  upper <- if (p == 1) 1 else
    (n / (n + zsq)) * ((p + zsq / (2 * n)) + fpc * z * r)
  c(lower = lower, upper = upper)
}

# Score CI with continuity correction, Fleiss quadratic method, as in
# OpenEpi Proportion.js.
score_cc_ci <- function(x, n, conf) {
  z <- oe_z(conf)
  zsq <- z * z
  p <- x / n
  lower <- if (p == 0) 0 else
    ((2 * n * p + zsq - 1) -
       z * sqrt(zsq - (2 + 1 / n) + 4 * p * (n * (1 - p) + 1))) /
    (2 * (n + zsq))
  upper <- if (p == 1) 1 else
    ((2 * n * p + zsq + 1) +
       z * sqrt(zsq + (2 - 1 / n) + 4 * p * (n * (1 - p) - 1))) /
    (2 * (n + zsq))
  c(lower = lower, upper = upper)
}
