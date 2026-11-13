# SPDX-License-Identifier: MIT
# 2x2 and stratified 2x2 statistics not available as plain R calls.
# Literature: Rothman & Greenland direct pooling (inverse-variance
# weights) and Kahn/Sempos etiologic fractions (OpenEpi TwobyTwo.js);
# Breslow & Day 1980 homogeneity tests (the Woolf form computed by
# OpenEpi); Martin & Austin, Epidemiology 1991;2:359-62 (stratified
# conditional-MLE OR and exact limits via convolution of hypergeometric
# coefficient polynomials, as in OpenEpi js/MartinStats.js).
#
# `strata` is a list of 2x2 matrices [[a,b],[c,d]], rows
# exposed/unexposed, columns ill/well (the corpus layout). OpenEpi's
# (a,b,c,d) cells are ill-and-exposed, ill-and-unexposed, well-and-
# exposed, well-and-unexposed = (a, c, b, d) of this layout; every
# formula below is written directly in the corpus layout.

# --- direct pooling ------------------------------------------------

# Directly adjusted OR ("or"), RR ("rr") or RD ("rd") over strata with
# inverse-variance weights; Taylor CIs.
direct_adjust <- function(strata, conf, measure = c("or", "rr", "rd")) {
  measure <- match.arg(measure)
  z <- qnorm((1 + conf / 100) / 2)
  ests <- w <- numeric(length(strata))
  for (i in seq_along(strata)) {
    a <- strata[[i]][1, 1]; b <- strata[[i]][1, 2]
    cc <- strata[[i]][2, 1]; d <- strata[[i]][2, 2]
    n1 <- a + b; n0 <- cc + d
    if (measure == "or") {
      ests[i] <- log((a * d) / (b * cc))
      w[i] <- 1 / (1 / a + 1 / b + 1 / cc + 1 / d)
    } else if (measure == "rr") {
      ests[i] <- log((a / n1) / (cc / n0))
      w[i] <- 1 / (b / (a * n1) + d / (cc * n0))
    } else {
      ests[i] <- (a / n1) - (cc / n0)
      w[i] <- 1 / ((a * b) / n1^3 + (cc * d) / n0^3)
    }
  }
  sw <- sum(w)
  adj <- sum(w * ests) / sw
  half <- z / sqrt(sw)
  if (measure == "rd") {
    c(est = adj, lower = adj - half, upper = adj + half)
  } else {
    c(est = exp(adj), lower = exp(adj - half), upper = exp(adj + half))
  }
}

# Breslow-Day (Woolf) homogeneity statistics over strata: chi-square
# and p (df = strata - 1) for OR, RR and RD around the directly
# adjusted estimate.
breslow_day <- function(strata, conf) {
  k <- length(strata)
  out <- list()
  for (measure in c("or", "rr", "rd")) {
    ests <- w <- numeric(k)
    for (i in seq_along(strata)) {
      a <- strata[[i]][1, 1]; b <- strata[[i]][1, 2]
      cc <- strata[[i]][2, 1]; d <- strata[[i]][2, 2]
      n1 <- a + b; n0 <- cc + d
      if (measure == "or") {
        ests[i] <- log((a * d) / (b * cc))
        w[i] <- 1 / (1 / a + 1 / b + 1 / cc + 1 / d)
      } else if (measure == "rr") {
        ests[i] <- log((a / n1) / (cc / n0))
        w[i] <- 1 / (b / (a * n1) + d / (cc * n0))
      } else {
        ests[i] <- (a / n1) - (cc / n0)
        w[i] <- 1 / ((a * b) / n1^3 + (cc * d) / n0^3)
      }
    }
    adj <- sum(w * ests) / sum(w)
    chi <- sum(w * (ests - adj)^2)
    out[[paste0(measure, "_chisq")]] <- chi
    out[[paste0(measure, "_p")]] <- pchisq(chi, k - 1, lower.tail = FALSE)
  }
  out
}

# --- OR-based etiologic/prevented fractions -------------------------

# Etiologic fraction in population/exposed and prevented fraction in
# population/exposed derived from the odds ratio, with CIs, as in
# OpenEpi TwobyTwo.js (Kahn/Sempos for EFp; transformed OR limits for
# EFe; PEPI for the prevented forms).
ef_or <- function(a, b, cc, d, conf) {
  z <- qnorm((1 + conf / 100) / 2)
  r1 <- a + cc          # ill total
  r2 <- b + d           # well total
  od <- (a * d) / (b * cc)
  od_lo <- od * exp(-z * sqrt(1 / a + 1 / b + 1 / cc + 1 / d))
  od_hi <- od * exp(z * sqrt(1 / a + 1 / b + 1 / cc + 1 / d))
  clamp <- function(x) max(-1, min(1, x))

  share <- b / r2       # exposed share among the well
  efpor <- clamp((share * (od - 1)) / (share * (od - 1) + 1))
  efeor <- clamp((od - 1) / od)
  pfpor <- share * (1 - od)
  pfeor <- 1 - od

  var_efpor <- ((cc * r2) / (d * r1))^2 *
    (a / (cc * r1) + b / (d * r2))
  efpor_lo <- max(-1, efpor - z * sqrt(var_efpor))
  efpor_hi <- min(1, efpor + z * sqrt(var_efpor))
  efeor_lo <- clamp((od_lo - 1) / od_lo)
  efeor_hi <- clamp((od_hi - 1) / od_hi)
  pfpor_lo <- 1 - 1 / (1 - efpor_hi)
  pfpor_hi <- 1 - 1 / (1 - efpor_lo)
  pfeor_lo <- 1 - od_hi
  pfeor_hi <- 1 - od_lo

  list(
    efp_or = c(est = efpor, lower = efpor_lo, upper = efpor_hi),
    efe_or = c(est = efeor, lower = efeor_lo, upper = efeor_hi),
    pfp_or = c(est = pfpor, lower = pfpor_lo, upper = pfpor_hi),
    pfe_or = c(est = pfeor, lower = pfeor_lo, upper = pfeor_hi)
  )
}

# --- risk-based etiologic/prevented fractions -------------------------
# epiR::epi.2by2 emits no prevented-fraction fields in epiR 3.0.0 and
# its PAF estimator differs from OpenEpi's, so the corpus computes the
# OpenEpi TwobyTwo.js formulas directly (Kahn/Sempos CI for EFp,
# transformed RR limits for EFe/PFe, transformed EFp limits for PFp).

# Taylor CI for a single proportion.
risk_ci <- function(x, n, conf) {
  z <- qnorm((1 + conf / 100) / 2)
  p <- x / n
  se <- sqrt(p * (1 - p) / n)
  c(est = p, lower = p - z * se, upper = p + z * se)
}

ef_risk <- function(a, b, cc, d, conf) {
  z <- qnorm((1 + conf / 100) / 2)
  n1 <- a + b; n0 <- cc + d
  r1 <- a + cc          # ill total
  t <- n1 + n0
  risk_e <- a / n1; risk_u <- cc / n0; risk_o <- r1 / t
  rr <- risk_e / risk_u
  rr_se <- sqrt((1 - a / n1) / (n1 * a / n1) +
                  (1 - cc / n0) / (n0 * cc / n0))
  rr_lo <- rr * exp(-z * rr_se)
  rr_hi <- rr * exp(z * rr_se)
  clamp <- function(x) max(-1, min(1, x))

  efp <- clamp((risk_o - risk_u) / risk_o)
  efe <- clamp((rr - 1) / rr)
  pfp <- (risk_u - risk_o) / risk_u
  pfe <- (risk_u - risk_e) / risk_u

  var_efp <- cc * t * (a * d * (t - cc) + b * cc^2) / (r1^3 * n0^3)
  efp_lo <- max(-1, efp - z * sqrt(var_efp))
  efp_hi <- min(1, efp + z * sqrt(var_efp))
  efe_lo <- clamp((rr_lo - 1) / rr_lo)
  efe_hi <- clamp((rr_hi - 1) / rr_hi)
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

# --- stratified conditional MLE odds ratio --------------------------
# Port of the Martin & Austin polynomial machinery in OpenEpi
# js/MartinStats.js: the conditional distribution of the sum of the
# "a" cells over strata is the convolution of per-stratum noncentral
# hypergeometrics; the CMLE and exact/mid-p limits are roots of tail
# probabilities of that distribution. Roots are found with
# uniroot(tol = 1e-14) (E5); the default-tolerance values are returned
# in `r_default` for the tight-root check.

# Per-stratum coefficient polynomial at odds 1 (coefficients of the
# generating function of the a-cell's conditional distribution).
# Returns list(poly = numeric[deg+1], min_a, max_a, informative).
oe_cells_cc <- function(tab) {
  # corpus [[a,b],[c,d]] (exp/unexp x ill/well) -> the a-cell is
  # exp&ill; margins m1 = ill, n1 = exposed, n0 = unexposed.
  a <- tab[1, 1]; b <- tab[1, 2]; cc <- tab[2, 1]; d <- tab[2, 2]
  list(a = a, m1 = a + cc, n1 = a + b, n0 = cc + d,
       informative = (a * d != 0) || (b * cc != 0))
}

poly_strat_cc <- function(t) {
  if (!t$informative) return(list(poly = 1, deg = 0, min_a = 0, max_a = 0))
  min_a <- max(0, t$m1 - t$n0)
  max_a <- min(t$m1, t$n1)
  deg <- max_a - min_a
  poly <- numeric(deg + 1)
  poly[1] <- 1
  aa <- min_a; bb <- t$m1 - min_a + 1; cc <- t$n1 - min_a + 1
  dd <- t$n0 - t$m1 + min_a
  for (i in seq_len(deg)) {
    poly[i + 1] <- poly[i] * ((bb - i) / (aa + i)) * ((cc - i) / (dd + i))
  }
  list(poly = poly, deg = deg, min_a = min_a, max_a = max_a)
}

# Person-time stratum: [[cases_exp, cases_unexp],[pt_exp, pt_unexp]];
# coefficient polynomial is binomial C(m1,i)*(n0/n1)^(m1-i).
oe_cells_pt <- function(tab) {
  a <- tab[1, 1]; b <- tab[1, 2]; n1 <- tab[2, 1]; n0 <- tab[2, 2]
  list(a = a, m1 = a + b, n1 = n1, n0 = n0,
       informative = (a * n0 != 0) || (b * n1 != 0))
}

poly_strat_pt <- function(t) {
  if (!t$informative) return(list(poly = 1, deg = 0, min_a = 0, max_a = 0))
  m1 <- round(t$m1)
  poly <- numeric(m1 + 1)
  poly[m1 + 1] <- 1
  c0 <- t$n0 / t$n1
  for (i in seq(m1 - 1, 0, by = -1)) {
    poly[i + 1] <- poly[i + 2] * c0 * (i + 1) / (m1 - i)
  }
  list(poly = poly, deg = m1, min_a = 0, max_a = m1)
}

mult_poly <- function(p1, p2) {
  out <- numeric(length(p1) + length(p2) - 1)
  for (i in seq_along(p1)) {
    for (j in seq_along(p2)) {
      out[i + j - 1] <- out[i + j - 1] + p1[i] * p2[j]
    }
  }
  out
}

# EvalPoly port: avoids overflow for r > 1.
eval_poly <- function(c, r) {
  deg_c <- length(c) - 1
  if (r == 0) return(c[1])
  if (r <= 1) {
    y <- c[deg_c + 1]
    if (r < 1) {
      for (i in seq(deg_c, 1)) y <- y * r + c[i]
    } else {
      for (i in seq(deg_c, 1)) y <- y + c[i]
    }
    y
  } else {
    y <- c[1]
    rr <- 1 / r
    for (i in 2:(deg_c + 1)) y <- y * rr + c[i]
    y
  }
}

# Shared convolved-polynomial solver. `cells_fn`/`poly_fn` select the
# case-control or person-time variant. `tol` is the uniroot tolerance
# (1e-14 for the recorded value; the caller also runs the R default for
# r_default).
martin_setup <- function(strata, cells_fn, poly_fn) {
  sum_a <- 0; min_sum <- 0; max_sum <- 0
  poly_d <- 1
  for (tab in strata) {
    t <- cells_fn(tab)
    if (t$informative) {
      sum_a <- sum_a + t$a
      p <- poly_fn(t)
      min_sum <- min_sum + p$min_a
      max_sum <- max_sum + p$max_a
      poly_d <- mult_poly(poly_d, p$poly)
    }
  }
  list(poly_d = poly_d, sum_a = sum_a, min_sum = min_sum, max_sum = max_sum)
}

martin_func <- function(poly_n, poly_d, r, value) {
  numer <- eval_poly(poly_n, r)
  denom <- eval_poly(poly_d, r)
  deg_diff <- (length(poly_d) - 1) - (length(poly_n) - 1)
  if (r <= 1) numer / denom - value
  else numer / r^deg_diff / denom - value
}

martin_root <- function(poly_n, poly_d, value, approx, tol) {
  f <- function(r) martin_func(poly_n, poly_d, r, value)
  x0 <- 0; x1 <- max(0.5, approx); iter <- 0
  f0 <- f(x0); f1 <- f(x1)
  while (f1 * f0 > 0 && iter < 10000) {
    iter <- iter + 1
    x0 <- x1; f0 <- f1
    x1 <- x1 * 1.5 * iter
    f1 <- f(x1)
  }
  if (f1 * f0 > 0) return(NA_real_)
  uniroot(f, c(x0, x1), tol = tol)$root
}

# diff = sum_a - min_sum index into poly_d/poly_n (0-based).
martin_cmle <- function(s, approx, tol) {
  if (s$min_sum == s$max_sum) return(NA_real_)
  if (s$sum_a == s$min_sum) return(0)
  if (s$sum_a == s$max_sum) return(Inf)
  poly_n <- ((s$min_sum + seq(0, length(s$poly_d) - 1)) * s$poly_d)
  martin_root(poly_n, s$poly_d, s$sum_a, approx, tol)
}

# Exact limit: lower = TRUE solves for the lower bound, fisher = TRUE
# for the Fisher (central) limit, FALSE for mid-p.
martin_lim <- function(s, lower, fisher, conf, approx, tol) {
  diff <- s$sum_a - s$min_sum
  value <- if (lower) 0.5 * (1 + conf) else 0.5 * (1 - conf)
  deg_n <- if (lower && fisher) diff - 1 else diff
  if (deg_n < 0) return(0) # cannot form numerator; limit is the boundary
  if (lower && s$sum_a == s$min_sum) return(0)
  if (!lower && s$sum_a == s$max_sum) return(Inf)
  poly_n <- s$poly_d[seq_len(deg_n + 1)]
  if (!fisher) poly_n[length(poly_n)] <- 0.5 * poly_n[length(poly_n)]
  martin_root(poly_n, s$poly_d, value, approx, tol)
}

# Stratified CMLE odds ratio over 2x2 strata: est plus mid-p and Fisher
# limits at `conf` (a fraction, e.g. 0.95). `tol` selects the uniroot
# tolerance; see stratified_cmle below for the E5 pair.
stratified_cmle_one <- function(strata, conf, tol) {
  s <- martin_setup(strata, oe_cells_cc, poly_strat_cc)
  crude <- do.call(rbind, strata)
  crude <- matrix(colSums(crude), 2, 2, byrow = TRUE)
  approx <- max(1e-8, crude[1, 1] * crude[2, 2] / max(1e-8, crude[1, 2] * crude[2, 1]))
  list(
    est = martin_cmle(s, approx, tol),
    midp_lower = martin_lim(s, TRUE, FALSE, conf, approx, tol),
    midp_upper = martin_lim(s, FALSE, FALSE, conf, approx, tol),
    fisher_lower = martin_lim(s, TRUE, TRUE, conf, approx, tol),
    fisher_upper = martin_lim(s, FALSE, TRUE, conf, approx, tol)
  )
}

# Entry point used by calc.R: tight roots (1e-14) plus the R-default
# values for the E5 check.
stratified_cmle <- function(strata, conf) {
  tight <- stratified_cmle_one(strata, conf, 1e-14)
  loose <- stratified_cmle_one(strata, conf, .Machine$double.eps^0.25)
  list(tight = tight, r_default = loose)
}

# Person-time CMLE rate ratio (Martin & Austin 1996). Same machinery;
# strata are [[cases_exp, cases_unexp],[pt_exp, pt_unexp]].
stratified_cmle_pt_one <- function(strata, conf, tol) {
  s <- martin_setup(strata, oe_cells_pt, poly_strat_pt)
  a_sum <- sum(vapply(strata, function(t) t[1, 1], numeric(1)))
  b_sum <- sum(vapply(strata, function(t) t[1, 2], numeric(1)))
  n1_sum <- sum(vapply(strata, function(t) t[2, 1], numeric(1)))
  n0_sum <- sum(vapply(strata, function(t) t[2, 2], numeric(1)))
  approx <- max(1e-8, (a_sum / n1_sum) / (b_sum / n0_sum))
  list(
    est = martin_cmle(s, approx, tol),
    midp_lower = martin_lim(s, TRUE, FALSE, conf, approx, tol),
    midp_upper = martin_lim(s, FALSE, FALSE, conf, approx, tol),
    fisher_lower = martin_lim(s, TRUE, TRUE, conf, approx, tol),
    fisher_upper = martin_lim(s, FALSE, TRUE, conf, approx, tol)
  )
}
