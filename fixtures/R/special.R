# fixtures/special/*.json — reference grids for the `special` maths
# module (plan Step 7). inputs.points is an array of argument arrays;
# expected.values is the matching array of leaves (or, for nc_hypergeom,
# of {pmf, mean, lower_tail, upper_tail} objects). All closed_form.

special_fixture <- function(fn, points, values) {
  stop_if(length(points) != length(values),
          sprintf("special.%s: points/values length mismatch", fn))
  write_fixture("special", fn, paste0("special.", fn),
                inputs = list(points = points),
                expected = list(values = values),
                packages = c("stats", "jsonlite"))
}

# ln_gamma: lgamma
special_fixture("ln_gamma",
  lapply(c(1e-300, 1e-8, 0.5, 1, 1.5, 2, 3, 10.5, 100, 1e5, 1e15),
         function(x) list(x)),
  lapply(c(1e-300, 1e-8, 0.5, 1, 1.5, 2, 3, 10.5, 100, 1e5, 1e15),
         function(x) leaf(lgamma(x))))

# normal_cdf / normal_sf: pnorm
norm_xs <- c(-38, -10, -5, -1.96, -1, 0, 0.5, 1.96, 5, 8.29, 10)
special_fixture("normal_cdf",
  lapply(norm_xs, function(x) list(x)),
  lapply(norm_xs, function(x) leaf(pnorm(x))))
special_fixture("normal_sf",
  lapply(norm_xs, function(x) list(x)),
  lapply(norm_xs, function(x) leaf(pnorm(x, lower.tail = FALSE))))

# normal_quantile: qnorm
norm_ps <- c(1e-300, 1e-10, 0.001, 0.025, 0.5, 0.975, 0.999, 1 - 1e-10)
special_fixture("normal_quantile",
  lapply(norm_ps, function(p) list(p)),
  lapply(norm_ps, function(p) leaf(qnorm(p))))

# chisq_sf: pchisq(x, df, lower.tail = FALSE)
chisq_pts <- expand.grid(x = c(0, 1e-6, 0.5, 3.84, 27.22, 100, 1000),
                         df = c(1, 2, 3, 6, 50))
special_fixture("chisq_sf",
  lapply(seq_len(nrow(chisq_pts)),
         function(i) list(chisq_pts$x[i], chisq_pts$df[i])),
  lapply(seq_len(nrow(chisq_pts)), function(i) {
    leaf(pchisq(chisq_pts$x[i], chisq_pts$df[i], lower.tail = FALSE))
  }))

# hypergeom_pmf: dhyper(x, m, n, k)
hg_pts <- list(
  c(5, 10, 8, 6), c(0, 4, 6, 3), c(3, 5, 7, 5), c(7, 12, 9, 10),
  c(1, 2, 3, 2), c(10, 20, 15, 12), c(8, 13, 17, 11),
  c(43, 46, 29, 54),          # oswego VANILLA cell and margins
  c(0, 5, 16, 5), c(4, 9, 12, 8), c(6, 15, 15, 10),
  c(2e6, 2e6, 2e6, 3e6),      # m = n = 2e6
  c(999999, 2e6, 2e6, 2e6), c(50, 100, 60, 80), c(3, 7, 5, 4)
)
special_fixture("hypergeom_pmf",
  hg_pts,
  lapply(hg_pts, function(p) {
    leaf(dhyper(p[1], p[2], p[3], p[4]))
  }))

# binomial_tail: pbinom(x, n, p, lower.tail)
bn_pts <- list(
  c(5, 10, 0.5, 1), c(5, 10, 0.5, 0), c(0, 10, 0.3, 1),
  c(10, 10, 0.7, 0), c(3, 20, 0.25, 1), c(17, 20, 0.75, 0),
  c(8, 15, 0.5, 1), c(43, 54, 0.5, 1), c(2, 12, 0.1, 0),
  c(9, 30, 0.4, 1), c(1, 5, 0.5, 0), c(49, 50, 0.9, 0),
  c(0, 1, 0.5, 1), c(6, 8, 0.875, 1), c(13, 25, 0.6, 0)
)
special_fixture("binomial_tail",
  bn_pts,
  lapply(bn_pts, function(p) {
    leaf(pbinom(p[1], p[2], p[3], lower.tail = (p[4] == 1)))
  }))

# beta_quantile: qbeta(p, a, b)
bq_pts <- expand.grid(p = c(0.025, 0.5, 0.975),
                      a = c(0.5, 1, 3, 43, 1e4),
                      b = c(0.5, 1, 3, 43, 1e4))
special_fixture("beta_quantile",
  lapply(seq_len(nrow(bq_pts)),
         function(i) list(bq_pts$p[i], bq_pts$a[i], bq_pts$b[i])),
  lapply(seq_len(nrow(bq_pts)), function(i) {
    leaf(qbeta(bq_pts$p[i], bq_pts$a[i], bq_pts$b[i]))
  }))

# nc_hypergeom: from nchyper.R — points [m, n, k, x, psi]; each value is
# {pmf, mean, lower_tail, upper_tail}.
nch_pts <- list(
  c(46, 29, 54, 43, 1),            # oswego vanilla margins at psi = 1
  c(46, 29, 54, 43, 23.45),
  c(46, 29, 54, 43, 1e-8),
  c(46, 29, 54, 43, 1e8),
  c(10, 8, 6, 5, 2.5),
  c(12, 9, 10, 7, 0.4),
  c(20, 15, 12, 10, 5),
  c(9, 12, 8, 4, 0.1),
  c(15, 15, 10, 6, 1),
  c(7, 5, 4, 3, 50),
  c(30, 25, 20, 18, 0.02),
  c(100, 60, 80, 50, 3)
)
nch_vals <- lapply(nch_pts, function(p) {
  h <- nch_problem(m = p[1], n = p[2], k = p[3], x = p[4])
  psi <- p[5]
  list(
    pmf = leaf(nch_dnhyper(h, psi)[h$x - h$lo + 1]),
    mean = leaf(nch_mnhyper(h, psi)),
    lower_tail = leaf(nch_pnhyper(h, h$x, psi)),
    upper_tail = leaf(nch_pnhyper(h, h$x, psi, upper.tail = TRUE))
  )
})
special_fixture("nc_hypergeom", nch_pts, nch_vals)
