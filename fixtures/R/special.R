# fixtures/special/*.json — reference grids for the `special` maths
# module (plan Step 7). inputs.points is an array of argument arrays;
# expected.values is the matching array of leaves (or, for nc_hypergeom,
# of {pmf, mean, lower_tail, upper_tail} objects). All closed_form.

special_fixture <- function(fn, points, values, key = "points") {
  stop_if(length(points) != length(values),
          sprintf("special.%s: points/values length mismatch", fn))
  inputs <- list()
  inputs[[key]] <- points
  write_fixture("special", fn, paste0("special.", fn),
                inputs = inputs,
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

# ---------------------------------------------------------------------------
# v1.0 Task 2 follow-up (spec §4.1): grids for the seven Task 5 functions.
# Argument order is run_special.rs's. These grids use the v1.0 `inputs.grid`
# key (the runner accepts both `grid` and `points`). A `value: null` leaf
# marks a point the reference calls non-finite; the runner serialises a
# failed root solve or an out-of-domain argument as {"undefined": code},
# which pairs with a null leaf. After PR #70 (exact R 4.6.2 nmath parity)
# every representable cell is in the grid — including p in {0, 1} and the
# domain-error shapes, where R's Inf/NaN leaf pairs with the engine's Err.
# The only points left out are ones a JSON grid or the runner cannot
# name: non-finite inputs (df = ±Inf, x = ±Inf, lambda = +Inf, NaN
# arguments); the scalar-CDF cells (t_cdf, f_sf at df <= 0), poisson_tail
# at lambda < 0 and binomial_quantile's out-of-domain cells (p or prob
# outside [0, 1], n < 0) — R's NaN leaf is null but the engine's NaN or
# clamped result serialises as a number or plain null, never
# {"undefined"}; and poisson_tail at k < 0, where run_special's `as u64`
# cast maps -1 to 0 and would mis-pair with R's ppois(-1, .) = 0. All are
# listed in docs/decisions.md ("v1.0 Task 2 follow-up — special.* grids").
# Every
# reference point in crates/epiora-core/tests/special.rs for these seven
# functions is covered: the expand.grid axes carry the reference-table
# values, and the seeded qbinom tables are regenerated by the same
# commands quoted in the test's doc comments.

# t_cdf: pt(t, df) — typical t, deep tails (|t| to 1e10), the large-df
# audit rows (df 1e8 to 1e300, the qnorm-valued t's) and the
# beta_l131_log_scale_redo cell (t = -40, df = 6000). df = +Inf (R:
# pnorm) and df <= 0 (R: NaN vs engine NaN) are unrepresentable —
# see docs/decisions.md.
tcdf_pts <- expand.grid(
  t = c(-1e10, -1e6, -1e4, -100, -40, -10, -5, -3.0902323061678136,
        -2, -1.6448536269514722, -1, -0.6744897501960817, 0, 0.5,
        0.6744897501960817, 1, 1.6448536269514722, 1.96, 2,
        3.0902323061678136, 5, 10, 100, 1e4, 1e6, 1e10),
  df = c(0.1, 0.5, 1, 2, 4, 30, 60, 120, 6000, 1e5, 1e6, 1e8, 1e10,
         1e12, 1e15, 1e20, 1e300))
special_fixture("t_cdf",
  lapply(seq_len(nrow(tcdf_pts)),
         function(i) list(tcdf_pts$t[i], tcdf_pts$df[i])),
  lapply(seq_len(nrow(tcdf_pts)),
         function(i) leaf(pt(tcdf_pts$t[i], tcdf_pts$df[i]))),
  key = "grid")

# t_quantile: qt(p, df) — p 0 and 1 are null leaves (R's -Inf/+Inf pair
# with the engine's Err), p outside [0, 1] and df <= 0 likewise (R's
# NaN; the engine's Result maps them to Err). p down to 1e-300 (R is
# finite or -Inf for df < 1; both now match), df to 1e300.
tq_pts <- expand.grid(
  p = c(-0.5, 0, 1e-300, 1e-100, 1e-16, 1e-12, 1e-8, 1e-6, 1e-4, 0.001,
        0.01, 0.025, 0.1, 0.25, 0.5, 0.75, 0.9, 0.975, 0.99, 0.999,
        0.9999, 1 - 1e-6, 1 - 1e-8, 1 - 1e-12, 1 - 1e-16, 1, 1.5),
  df = c(-1, 0, 0.1, 0.5, 1, 2, 4, 30, 60, 120, 1e5, 1e6, 1e8, 1e10,
         1e12, 1e15, 1e20, 1e300))
special_fixture("t_quantile",
  lapply(seq_len(nrow(tq_pts)),
         function(i) list(tq_pts$p[i], tq_pts$df[i])),
  lapply(seq_len(nrow(tq_pts)),
         function(i) leaf(qt(tq_pts$p[i], tq_pts$df[i]))),
  key = "grid")

# f_sf: pf(x, df1, df2, lower.tail = FALSE) — x 0 to 1e300 (sf down to
# underflow), df 0.5 to 1e10. The second block is the review's
# large-df-pair audit (tests/special.rs F_SF_LARGE_DF_REF); the
# explicit rows are the beta_tiny_ab_ratio_block and
# beta_l131_log_scale_redo f_sf cells. Infinite-df cells (R's pchisq
# paths) and x = ±Inf are unrepresentable — see docs/decisions.md.
f_pts <- expand.grid(
  x = c(-1, 0, 0.1, 0.5, 1, 3, 10, 1e4, 1e10, 1e100, 1e300),
  df1 = c(0.5, 1, 4, 30, 1e5, 1e6, 1e10),
  df2 = c(0.5, 1, 2, 10, 60, 1e5, 2e5, 1e10))
f_pairs <- rbind(c(1e8, 1e8), c(1e10, 1e12), c(1e12, 1e10), c(1e15, 1e8),
                 c(1e20, 1e20), c(1e300, 1e300), c(1e8, 1e300),
                 c(1e300, 1e8))
f_large <- expand.grid(x = c(0.01, 0.1, 0.5, 0.9, 0.99, 0.9999, 1,
                             1.0001, 1.01, 1.1, 2, 10, 100),
                       pair = seq_len(nrow(f_pairs)))
f_pts <- rbind(f_pts,
  data.frame(x = f_large$x, df1 = f_pairs[f_large$pair, 1],
             df2 = f_pairs[f_large$pair, 2]),
  data.frame(x = c(1, 0.25, 2, 1600, 1600, 1.5e308, 1.5e308, 1.5e308,
                   1.7e308, 1.7e308, 1e308, 1e308),
             df1 = c(4e-19, 4e-19, 2e-19, 1, 1, 3e-308, 5e-308, 2e-308,
                     2e-310, 2e-310, 4e-310, 4e-310),
             df2 = c(2e-19, 2e-19, 4e-19, 6400, 6600, 20, 20, 20,
                     1.8, 20, 1.8, 20)))
special_fixture("f_sf",
  lapply(seq_len(nrow(f_pts)),
         function(i) list(f_pts$x[i], f_pts$df1[i], f_pts$df2[i])),
  lapply(seq_len(nrow(f_pts)), function(i) {
    leaf(pf(f_pts$x[i], f_pts$df1[i], f_pts$df2[i], lower.tail = FALSE))
  }),
  key = "grid")

# gamma_quantile: qgamma(p, shape, scale = scale) — p 0 (R's 0, the
# engine's Ok(0) boundary) and 1 (R's +Inf leaf pairs with Err), shapes
# 0 (point mass) and -1 (NaN domain), deep-tail p (1e-300, 1e-100,
# 1e-15..1e-16), small shapes to 1e-300 and large to 1e6, scale 0
# (NaN domain). NaN arguments are unrepresentable — see
# docs/decisions.md.
gq_pts <- expand.grid(
  p = c(-0.5, 0, 1e-300, 1e-100, 1e-16, 1e-15, 1e-14, 1e-10, 1e-6,
        0.01, 0.1, 0.5, 0.9, 0.99, 1 - 1e-6, 1 - 1e-8, 1 - 1e-12,
        1 - 1e-15, 1 - 1e-16, 1, 1.5),
  shape = c(-1, 0, 1e-300, 0.01, 0.1, 0.5, 1, 2.5, 5, 50, 100, 1e3,
            3e3, 1e4, 1e6),
  scale = c(0, 0.5, 1, 2.5))
special_fixture("gamma_quantile",
  lapply(seq_len(nrow(gq_pts)),
         function(i) list(gq_pts$p[i], gq_pts$shape[i], gq_pts$scale[i])),
  lapply(seq_len(nrow(gq_pts)), function(i) {
    leaf(qgamma(gq_pts$p[i], gq_pts$shape[i], scale = gq_pts$scale[i]))
  }),
  key = "grid")

# chisq_quantile: qchisq(p, df) — p 0 and 1 like qgamma (0 vs +Inf),
# df 0 (point mass at 0) and -1 (NaN domain), deep-tail p, df 1e-300
# to 1e10.
cq_pts <- expand.grid(
  p = c(-0.5, 0, 1e-300, 1e-100, 1e-16, 1e-15, 1e-14, 1e-10, 1e-6,
        0.025, 0.1, 0.5, 0.9, 0.975, 1 - 1e-6, 1 - 1e-8, 1 - 1e-12,
        1 - 1e-15, 1 - 1e-16, 1, 1.5),
  df = c(-1, 0, 1e-300, 0.01, 0.5, 1, 4, 10, 30, 490, 1e3, 3e3, 1e4,
         1e5, 1e6, 1e10))
special_fixture("chisq_quantile",
  lapply(seq_len(nrow(cq_pts)),
         function(i) list(cq_pts$p[i], cq_pts$df[i])),
  lapply(seq_len(nrow(cq_pts)),
         function(i) leaf(qchisq(cq_pts$p[i], cq_pts$df[i]))),
  key = "grid")

# poisson_tail: [k, lambda, upper] — lower is ppois(k, lambda); upper is
# P(X >= k) = ppois(k - 1, lambda, lower.tail = FALSE). k 0 to
# u64::MAX (1.8446744073709552e19, the poisson_tail_u64_max_no_panic
# point) plus 2.5 (R floors the ppois argument; run_special's `as u64`
# truncates the same way), lambda 0 to 1e9.
pois_pts <- expand.grid(
  k = c(0, 1, 2, 2.5, 5, 50, 1000, 1e5, 1.8446744073709552e19),
  lambda = c(0, 0.5, 3, 20, 100, 1e4, 1e6, 1e9),
  upper = c(0, 1))
special_fixture("poisson_tail",
  lapply(seq_len(nrow(pois_pts)),
         function(i) list(pois_pts$k[i], pois_pts$lambda[i],
                          pois_pts$upper[i])),
  lapply(seq_len(nrow(pois_pts)), function(i) {
    if (pois_pts$upper[i] == 1) {
      leaf(ppois(pois_pts$k[i] - 1, pois_pts$lambda[i],
                 lower.tail = FALSE))
    } else {
      leaf(ppois(pois_pts$k[i], pois_pts$lambda[i]))
    }
  }),
  key = "grid")

# binomial_quantile: qbinom(p, n, prob) — p 0 to 1 including 1e-16 and
# 1 - 1e-16, n 0 to 1e9, prob 0/1 degenerate cases. The two appended
# blocks regenerate tests/special.rs's seeded tables verbatim:
# QBINOM_R_REF (seed 42) and QBINOM_PARITY_REF (seed 20261114) — the
# same commands in the same order, so the same (p, n, prob) triples.
bq_pts <- expand.grid(
  p = c(0, 1e-16, 1e-6, 0.001, 0.025, 0.05, 0.25, 0.5, 0.9, 0.975,
        0.99, 1 - 1e-6, 1 - 1e-16, 1),
  n = c(0, 1, 10, 20, 30, 50, 54, 100, 1000, 1e5, 1e9),
  prob = c(0, 0.01, 0.25, 0.3, 0.4, 0.5, 0.9, 1))
set.seed(42)
qr_n <- c(sample(1:1000, 60.0, TRUE), sample(1:1e6, 60.0, TRUE),
          sample(1:1e9, 60.0, TRUE), 1, 2, 10, 1000, 1e6, 1e9)
qr_prob <- c(runif(186.0, 0.0, 1.0), 0, 1, 1e-12, 1 - 1e-12, 0.5, 1e-9,
             1 - 1e-9, 0.3, 0.7, 0.99)[1:186]
qr_p <- c(runif(186.0, 0.0, 1.0), 0, 1, 1e-12, 1 - 1e-12, 0.5, 0.001,
          0.999, 1e-9, 1 - 1e-9)[1:186]
qr_n <- c(qr_n,
          c(1e9, 1e9, 100, 50, 1e6, 1e9, 2, 3, 5, 1e4, 1e9, 1e9, 1e6,
            1e9, 10, 1e9, 1e9, 1e5, 1e7, 1e8, 1e9, 1), 1e9, 1000000)
qr_prob <- c(qr_prob,
             c(0.5, 1e-9, 0.5, 0.9, 1e-6, 0.7, 0.4, 0.6, 0.2, 0.8, 1e-12,
               1 - 1e-15, 1 - 1e-12, 1e-15, 0.5, 0.999999999, 1e-10,
               1 - 1e-10, 0.123456789, 0.0001, 1 - 1e-8, 0.5),
             1 - 1e-12, 1e-12)
qr_p <- c(qr_p,
          c(1e-12, 1 - 1e-12, 0.9999999, 1e-9, 1 - 1e-15, 0.5, 0.1, 0.9,
            0.99, 1e-12, 0.999999, 1 - 1e-12, 0.75, 1e-10, 0.5,
            1 - 1e-9, 0.5, 0.99999999, 0.4321, 1e-10, 1 - 1e-14, 1),
          0.9999, 1e-8)
set.seed(20261114)
qp_n <- c(rep(c(1e5, 1e9), each = 6),
          10, 10, 10, 10, 0, 0, 1e9, 1e9, 10)
qp_prob <- c(rep(c(0.01, 0.25, 0.3, 0.4, 0.5, 0.9), 2),
             0.5, 0.5, 0, 1, 0.5, 0.5, 0.5, 1e-12, 1e-16)
qp_p <- c(rep(1 - 1e-16, 12),
          0, 1, 0.5, 0.5, 0.5, 1 - 1e-16, 1e-16, 0.5, 1e-16)
qp_ps <- c(1e-16, 1e-6, 0.001, 0.025, 0.05, 0.25, 0.5, 0.9, 0.975,
           0.99, 1 - 1e-6, 1 - 1e-16)
qp_ns <- c(1, 10.0, 20.0, 30.0, 50.0, 54.0, 100.0, 1000.0, 1e5, 1e9)
qp_prs <- c(0.01, 0.25, 0.3, 0.4, 0.5, 0.9)
qp_grid <- expand.grid(p = qp_ps, n = qp_ns, pr = qp_prs)
qp_idx <- sample(nrow(qp_grid), 108.0)
qp_n <- c(qp_n, qp_grid$n[qp_idx])
qp_prob <- c(qp_prob, qp_grid$pr[qp_idx])
qp_p <- c(qp_p, qp_grid$p[qp_idx])
bq_pts <- unique(rbind(
  bq_pts,
  data.frame(p = qr_p, n = qr_n, prob = qr_prob),
  data.frame(p = qp_p, n = qp_n, prob = qp_prob)))
special_fixture("binomial_quantile",
  lapply(seq_len(nrow(bq_pts)),
         function(i) list(bq_pts$p[i], bq_pts$n[i], bq_pts$prob[i])),
  lapply(seq_len(nrow(bq_pts)),
         function(i) leaf(qbinom(bq_pts$p[i], bq_pts$n[i],
                                 bq_pts$prob[i]))),
  key = "grid")
