# Tight noncentral-hypergeometric machinery (E5, spec §6.2).
#
# dnhyper/mnhyper/pnhyper are ported verbatim from the 2x2 branch of
# R 4.5.2's stats::fisher.test (src/library/stats/R/fisher.test.R). The
# CMLE (mle) and the Fisher exact CI limits (ncp.L / ncp.U) are the same
# code with every uniroot() call tightened to tol = 1e-14. The mid-p
# estimate/CI and the mid-p two-sided p port epitools 0.5-10.1's
# or.midp / ormidp.test, which reach the same pnhyper through
# fisher.test(x, or = ncp); the port swaps those calls for the functions
# below and tightens uniroot the same way.
#
# Hypergeometric layout: fisher.test on a 2x2 table [[a,b],[c,d]] uses
# m = a + c, n = b + d, k = a + b, x = a — i.e. X | margins is the count
# in cell [1,1]. The distribution is invariant under transpose, so this
# file works on our table layout directly.

# Build the hypergeometric problem from margins: X | margins is the
# count in cell [1,1] of a 2x2 table with column totals m, n, row-1
# total k and observed x.
nch_problem <- function(m, n, k, x) {
  lo <- max(0L, k - n)
  hi <- min(k, m)
  support <- lo:hi
  logdc <- dhyper(support, m, n, k, log = TRUE)
  list(m = m, n = n, k = k, x = x, lo = lo, hi = hi,
       support = support, logdc = logdc)
}

# Same, from a 2x2 table [[a,b],[c,d]] (rows exposed/unexposed, columns
# ill/well): fisher.test's m = a + c, n = b + d, k = a + b, x = a.
nch_setup <- function(tab) {
  nch_problem(sum(tab[, 1L]), sum(tab[, 2L]), sum(tab[1L, ]), tab[1L, 1L])
}

# --- verbatim ports of fisher.test's internals ------------------------

nch_dnhyper <- function(h, ncp) {
  d <- h$logdc + log(ncp) * h$support
  d <- exp(d - max(d))
  d / sum(d)
}

nch_mnhyper <- function(h, ncp) {
  if (ncp == 0) return(h$lo)
  if (ncp == Inf) return(h$hi)
  sum(h$support * nch_dnhyper(h, ncp))
}

# fisher.test's pnhyper uses the observed cell, not `q`, in the ncp == 1
# shortcut; kept identical here.
nch_pnhyper <- function(h, q, ncp = 1, upper.tail = FALSE) {
  if (ncp == 1) {
    return(if (upper.tail) {
      phyper(h$x - 1, h$m, h$n, h$k, lower.tail = FALSE)
    } else {
      phyper(h$x, h$m, h$n, h$k)
    })
  }
  if (ncp == 0) {
    return(as.numeric(if (upper.tail) q <= h$lo else q >= h$lo))
  }
  if (ncp == Inf) {
    return(as.numeric(if (upper.tail) q <= h$hi else q >= h$hi))
  }
  sum(nch_dnhyper(h, ncp)[if (upper.tail) h$support >= q else h$support <= q])
}

# Conditional MLE odds ratio — fisher.test's `mle` with tol = 1e-14.
nch_cmle <- function(h) {
  x <- h$x
  if (x == h$lo) return(0)
  if (x == h$hi) return(Inf)
  mu <- nch_mnhyper(h, 1)
  if (mu > x) {
    uniroot(function(t) nch_mnhyper(h, t) - x, c(0, 1),
            tol = 1e-14)$root
  } else if (mu < x) {
    1 / uniroot(function(t) nch_mnhyper(h, 1 / t) - x,
                c(.Machine$double.eps, 1), tol = 1e-14)$root
  } else {
    1
  }
}

# fisher.test's ncp.U / ncp.L with tol = 1e-14.
nch_ncp_U <- function(h, x, alpha) {
  if (x == h$hi) return(Inf)
  p <- nch_pnhyper(h, x, 1)
  if (p < alpha) {
    uniroot(function(t) nch_pnhyper(h, x, t) - alpha, c(0, 1),
            tol = 1e-14)$root
  } else if (p > alpha) {
    1 / uniroot(function(t) nch_pnhyper(h, x, 1 / t) - alpha,
                c(.Machine$double.eps, 1), tol = 1e-14)$root
  } else {
    1
  }
}

nch_ncp_L <- function(h, x, alpha) {
  if (x == h$lo) return(0)
  p <- nch_pnhyper(h, x, 1, upper.tail = TRUE)
  if (p > alpha) {
    uniroot(function(t) nch_pnhyper(h, x, t, upper.tail = TRUE) - alpha,
            c(0, 1), tol = 1e-14)$root
  } else if (p < alpha) {
    1 / uniroot(function(t) nch_pnhyper(h, x, 1 / t, upper.tail = TRUE) -
                  alpha, c(.Machine$double.eps, 1), tol = 1e-14)$root
  } else {
    1
  }
}

# Fisher exact two-sided CI at conf.level — fisher.test's CINT branch.
nch_fisher_ci <- function(h, conf.level = 0.95) {
  alpha <- (1 - conf.level) / 2
  c(nch_ncp_L(h, h$x, alpha), nch_ncp_U(h, h$x, alpha))
}

# --- mid-p ports (epitools or.midp / ormidp.test) ---------------------

# or.midp's `mue`: P(X <= x | ncp) - P(X >= x | ncp).
nch_mue <- function(h, ncp) {
  nch_pnhyper(h, h$x, ncp) - nch_pnhyper(h, h$x, ncp, upper.tail = TRUE)
}

# ormidp.test's `midp` at a given non-centrality.
nch_midp <- function(h, ncp) {
  0.5 * (nch_pnhyper(h, h$x, ncp) - nch_pnhyper(h, h$x, ncp,
                                               upper.tail = TRUE) + 1)
}

# Median-unbiased OR estimate — or.midp's EST root.
nch_midp_estimate <- function(h, interval = c(0, 1000)) {
  uniroot(function(or) nch_mue(h, or), interval = interval,
          tol = 1e-14)$root
}

# Mid-p exact CI — or.midp's LCL/UCL roots.
nch_midp_ci <- function(h, conf.level = 0.95, interval = c(0, 1000)) {
  alpha <- 1 - conf.level
  lcl <- uniroot(function(or) 1 - nch_midp(h, or) - alpha / 2,
                 interval = interval, tol = 1e-14)$root
  ucl <- 1 / uniroot(function(or) nch_midp(h, 1 / or) - alpha / 2,
                     interval = interval, tol = 1e-14)$root
  c(lcl, ucl)
}

# Mid-p two-sided p-value at ncp = 1 — ormidp.test.
nch_midp_p_two_sided <- function(h) {
  pval1 <- 0.5 * (nch_pnhyper(h, h$x, 1) -
                    nch_pnhyper(h, h$x, 1, upper.tail = TRUE) + 1)
  one.sided <- min(pval1, 1 - pval1)
  2 * one.sided
}
