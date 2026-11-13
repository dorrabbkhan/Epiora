# SPDX-License-Identifier: MIT
# Pair-matched case-control statistics. Literature: b/c with the Taylor
# log CI sqrt(1/b + 1/c) (OpenEpi MatchCC.js / EI7 pmccTable.cs).

# Pair-matched odds ratio b/c with Taylor confidence limits.
matched_or_taylor <- function(b, c, conf) {
  z <- qnorm((1 + conf / 100) / 2)
  or <- b / c
  se <- sqrt(1 / b + 1 / c)
  c(est = or, lower = or * exp(-z * se), upper = or * exp(z * se))
}

# Discordant pair count b + c.
discordant_pairs <- function(b, c) b + c
