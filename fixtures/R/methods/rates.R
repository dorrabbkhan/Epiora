# SPDX-License-Identifier: MIT
# Portions ported from OpenEpi (js/MartinStats.js), Copyright (c) 2003,
# 2008 Andrew G. Dean and Kevin M. Sullivan, MIT License.
#
# Permission is hereby granted, free of charge, to any person obtaining
# a copy of this software and associated documentation files (the
# "Software"), to deal in the Software without restriction, including
# without limitation the rights to use, copy, modify, merge, publish,
# distribute, sublicense, and/or sell copies of the Software, and to
# permit persons to whom the Software is furnished to do so, subject to
# the following conditions: The above copyright notice and this
# permission notice shall be included in all copies or substantial
# portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
# EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
# MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
# NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS
# BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
# ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
# CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
# SOFTWARE.
#
# Person-time (two-rate) statistics not available as plain R calls.
# Literature: Rothman & Boice 1979 / Rothman & Greenland (OpenEpi
# PersonTime2.js formulas); Martin & Austin, Epidemiology 1996;7:29-33
# (exact rate ratio via the same polynomial machinery as
# methods/tables.R); Abramson & Gahlinger, PEPI (etiologic fractions);
# Breslow & Day 1987 person-time homogeneity tests.
#
# `strata` is a list of 2x2 matrices [[cases_exp, cases_unexp],
# [pt_exp, pt_unexp]] (the corpus layout). In OpenEpi terms a = E0D0,
# b = E0D1, n1 = E1D0, n0 = E1D1.

# Rate difference with the Taylor CI (labelled "Byar" family in
# OpenEpi's PersonTime2 output).
rd_byar <- function(a, b, n1, n0, conf) {
  z <- qnorm((1 + conf / 100) / 2)
  dp <- a / n1 - b / n0
  se <- sqrt(a / n1^2 + b / n0^2)
  c(est = dp, lower = dp - z * se, upper = dp + z * se)
}

# Etiologic/prevented fractions on rates, PEPI formulas as in OpenEpi
# PersonTime2.js. EFp/EFe are clamped to [-1,1]; prevented fractions
# are not.
ef_person_time <- function(a, b, n1, n0, conf) {
  z <- qnorm((1 + conf / 100) / 2)
  rate_e <- a / n1; rate_u <- b / n0
  rate_o <- (a + b) / (n1 + n0)
  rr <- rate_e / rate_u
  rr_lo <- rr * exp(-z * sqrt(1 / a + 1 / b))
  rr_hi <- rr * exp(z * sqrt(1 / a + 1 / b))
  clamp <- function(x) max(-1, min(1, x))

  efp <- clamp((rate_o - rate_u) / rate_o)
  efe <- clamp((rr - 1) / rr)
  pfp <- (rate_u - rate_o) / rate_u
  pfe <- (rate_u - rate_e) / rate_u

  share <- n1 / (n1 + n0) # exposed share of person-time
  efp_lo <- (share * (rr_lo - 1)) / (share * (rr_lo - 1) + 1)
  efp_hi <- (share * (rr_hi - 1)) / (share * (rr_hi - 1) + 1)
  efe_lo <- (rr_lo - 1) / rr_lo
  efe_hi <- (rr_hi - 1) / rr_hi
  pfp_lo <- 1 - 1 / (1 - efp_hi)
  pfp_hi <- 1 - 1 / (1 - efp_lo)
  pfe_lo <- 1 - rr_hi
  pfe_hi <- 1 - rr_lo

  list(
    efp = c(est = efp, lower = efp_lo, upper = efp_hi),
    efe = c(est = efe, lower = efe_lo, upper = efe_hi),
    pfp = c(est = pfp, lower = pfp_lo, upper = pfp_hi),
    pfe = c(est = pfe, lower = pfe_lo, upper = pfe_hi)
  )
}

# CMLE rate ratio over person-time strata (Martin & Austin 1996), mid-p
# and Fisher limits. E5 pair: tight root at 1e-14, R-default in
# r_default.
cmle_rr <- function(strata, conf) {
  tight <- stratified_cmle_pt_one(strata, conf, 1e-14)
  loose <- stratified_cmle_pt_one(strata, conf, .Machine$double.eps^0.25)
  list(tight = tight, r_default = loose)
}

# Adjusted estimates over person-time strata: directly adjusted RR
# (weights a*b/m1), Mantel-Haenszel RR (SE per Rothman & Greenland),
# directly adjusted RD (weights 1/(a/n1^2 + b/n0^2)). Taylor CIs. Also
# the pooled z-score OpenEpi reports for stratified person-time data.
adjust <- function(strata, conf) {
  z <- qnorm((1 + conf / 100) / 2)
  k <- length(strata)
  a <- vapply(strata, function(t) t[1, 1], numeric(1))
  b <- vapply(strata, function(t) t[1, 2], numeric(1))
  n1 <- vapply(strata, function(t) t[2, 1], numeric(1))
  n0 <- vapply(strata, function(t) t[2, 2], numeric(1))
  m1 <- a + b; n <- n1 + n0
  rr <- (a / n1) / (b / n0)
  rd <- (a / n1) - (b / n0)

  # direct RR
  w_rr <- (a * b) / m1
  adj_rr <- exp(sum(w_rr * log(rr)) / sum(w_rr))
  # MH RR
  mh_num <- sum(a * n0 / n); mh_den <- sum(b * n1 / n)
  adj_rr_mh <- mh_num / mh_den
  se_mh <- sqrt(sum(m1 * n1 * n0 / n^2) / (mh_num * mh_den))
  # direct RD
  w_rd <- 1 / (a / n1^2 + b / n0^2)
  adj_rd <- sum(w_rd * rd) / sum(w_rd)
  # pooled z-score
  zscore <- (sum(a) - sum(m1 * n1 / n)) / sqrt(sum(m1 * n1 * n0 / n^2))

  list(
    direct_rr = c(est = adj_rr,
                  lower = adj_rr * exp(-z / sqrt(sum(w_rr))),
                  upper = adj_rr * exp(z / sqrt(sum(w_rr)))),
    mh_rr = c(est = adj_rr_mh,
              lower = adj_rr_mh * exp(-z * se_mh),
              upper = adj_rr_mh * exp(z * se_mh)),
    direct_rd = c(est = adj_rd,
                  lower = adj_rd - z / sqrt(sum(w_rd)),
                  upper = adj_rd + z / sqrt(sum(w_rd))),
    zscore = c(value = zscore, p = 2 * pnorm(-abs(zscore)))
  )
}

# Breslow-Day homogeneity tests for rates (person-time form): RR
# chi-square around the directly adjusted RR with weights a*b/m1;
# RD chi-square with weights 1/(a/n1^2 + b/n0^2); df = strata - 1.
breslow_day_rates <- function(strata, conf) {
  k <- length(strata)
  a <- vapply(strata, function(t) t[1, 1], numeric(1))
  b <- vapply(strata, function(t) t[1, 2], numeric(1))
  n1 <- vapply(strata, function(t) t[2, 1], numeric(1))
  n0 <- vapply(strata, function(t) t[2, 2], numeric(1))
  m1 <- a + b
  ln_rr <- log((a / n1) / (b / n0))
  rd <- (a / n1) - (b / n0)
  w_rr <- (a * b) / m1
  w_rd <- 1 / (a / n1^2 + b / n0^2)
  adj_ln_rr <- sum(w_rr * ln_rr) / sum(w_rr)
  adj_rd <- sum(w_rd * rd) / sum(w_rd)
  chi_rr <- sum(w_rr * (ln_rr - adj_ln_rr)^2)
  chi_rd <- sum(w_rd * (rd - adj_rd)^2)
  list(
    rr_chisq = chi_rr, rr_p = pchisq(chi_rr, k - 1, lower.tail = FALSE),
    rd_chisq = chi_rd, rd_p = pchisq(chi_rd, k - 1, lower.tail = FALSE)
  )
}
