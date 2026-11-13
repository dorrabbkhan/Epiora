# SPDX-License-Identifier: MIT
# Screening-test statistics. Literature: OpenEpi DiagnosticTest.js
# formulas (log-method LRs per Simel et al. 1991; Cohen's kappa handled
# by epiR::epi.kappa); entropy reduction and bias index as implemented
# in OpenEpi.
#
# The 2x2 cut uses tp/fp/fn/tn counts; `D` = diseased column total,
# `nD` = non-diseased column total.

# Diagnostic odds ratio (tp*tn)/(fp*fn) with the log-method CI.
diagnostic_odds <- function(tp, fp, fn, tn, conf) {
  z <- qnorm((1 + conf / 100) / 2)
  odds <- (tp * tn) / (fp * fn)
  se <- sqrt(1 / tp + 1 / fp + 1 / fn + 1 / tn)
  c(est = odds, lower = odds * exp(-z * se), upper = odds * exp(z * se))
}

# Entropy reduction after a positive/negative test. `prev` = D/N;
# `post` = posterior probability (PPV for a positive test, 1 - NPV for
# a negative test, per DiagnosticTest.js which uses ppv and npv
# directly).
entropy_reduction <- function(prev, ppv, npv) {
  h <- function(p) -(p * log(p) + (1 - p) * log(1 - p))
  enth <- h(prev)
  list(positive = enth - h(ppv), negative = enth - h(npv))
}

# Bias index (fp - fn)/N: apparent minus true prevalence.
bias_index <- function(fp, fn, n) (fp - fn) / n

# Level-specific likelihood ratio for a multi-level test row: ld/nD
# counts in that level, column totals D and nD. CI per DiagnosticTest.js
# log method: lr * exp(-/+ z * ((1-p1)/(D*p1) + (1-p2)/(nD*p2))).
level_lr <- function(ld, ln, big_d, big_nd, conf) {
  z <- qnorm((1 + conf / 100) / 2)
  p1 <- ld / big_d
  p2 <- ln / big_nd
  lr <- p1 / p2
  half <- z * ((1 - p1) / (big_d * p1) + (1 - p2) / (big_nd * p2))
  c(est = lr, lower = lr * exp(-half), upper = lr * exp(half))
}
