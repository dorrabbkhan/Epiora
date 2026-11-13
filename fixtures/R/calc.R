# fixtures/calc/<id>/*.json — calculator reference fixtures (v1.0
# Task 2). Each fixture's procedure is `calc.<id>`; inputs are exactly
# the calculator's named inputs from docs/v1.0-calculators.md;
# expected is keyed by that calculator's output paths (minus the
# `calc.<id>.` prefix). `random_numbers` gets no R fixture (spec §5.2:
# seeded ChaCha, no R reference).
#
# Table layout for two_by_two / two_by_n strata: [[a,b],[c,d]] rows
# exposed/unexposed, columns ill/well. Person-time strata:
# [[cases_exp, cases_unexp],[pt_exp, pt_unexp]].

calc_write <- function(dir, id, inputs, expected, packages) {
  write_fixture(file.path("calc", dir), id, paste0("calc.", dir),
                inputs, expected, packages)
}

stat_ci <- function(x, class = "closed_form") {
  list(est = leaf(x[1]), lower = leaf(x[2], class),
       upper = leaf(x[3], class))
}

# Generation-time conf-level guard (fix round 4). A package call that
# drops conf.level silently uses its 0.95 default (the
# riskratio/oddsratio leak fixed this round), so for a case with
# conf_level != 95 its limits come out identical to the 95%
# computation. The guard rebuilds the case's expected at 95 and
# requires every interval leaf — a leaf under a key containing
# "lower" or "upper" (lower, upper, lower_rank, midp_lower,
# fisher_lower, ...) — to differ; an unchanged leaf stops generation.
# identical() flags any bitwise-same value: a real limit at a
# different level always differs, and an undefined leaf (NA at both
# levels) is indistinguishable from a leak, so it flags too.
# Estimate leaves are conf-invariant and unchecked. `rebuild`
# recomputes the case's expected at a level given in percent (the
# same units as each writer's `conf`); the guard is a no-op at 95.
conf_interval_walk <- function(at_conf, at_95, path, bad) {
  if (is.list(at_conf) &&
      identical(sort(names(at_conf)[names(at_conf) != "r_default"]),
                c("class", "value"))) {
    if (grepl("lower|upper", path[length(path)]) &&
        identical(at_conf$value, at_95$value)) {
      bad[[length(bad) + 1]] <- paste(path, collapse = ".")
    }
    return(bad)
  }
  if (is.list(at_conf)) {
    nms <- names(at_conf)
    for (i in seq_along(at_conf)) {
      nm <- if (!is.null(nms) && nms[i] != "") {
        nms[i]
      } else {
        paste0("[", i, "]")
      }
      bad <- conf_interval_walk(at_conf[[i]], at_95[[i]],
                                c(path, nm), bad)
    }
  }
  bad
}

check_conf_leak <- function(id, conf, expected, rebuild) {
  if (conf == 95) return(invisible(TRUE))
  bad <- conf_interval_walk(expected, rebuild(95), id, list())
  stop_if(length(bad) > 0, sprintf(paste0(
    "conf-level leak check failed for %s (conf=%s): interval leaves ",
    "identical at 95%%: %s"),
    id, conf, paste(unlist(bad), collapse = ", ")))
  invisible(TRUE)
}

# --- proportion ---------------------------------------------------------

proportion_expected <- function(x, n, conf, pop, compare_with) {
  z <- qnorm((1 + conf / 100) / 2)
  p <- x / n
  midp <- tryCatch(
    exactci::binom.exact(x, n, conf.level = conf / 100,
                         midp = TRUE)$conf.int,
    error = function(e) c(NA, NA))
  cp <- tryCatch(binom.test(x, n, conf.level = conf / 100)$conf.int,
                 error = function(e) c(NA, NA))
  ac <- PropCIs::addz2ci(x, n, conf.level = conf / 100)$conf.int
  wi <- PropCIs::scoreci(x, n, conf.level = conf / 100)$conf.int
  wald_l <- max(0, p - z * sqrt(p * (1 - p) / n))
  wald_u <- min(1, p + z * sqrt(p * (1 - p) / n))
  expected <- list(
    estimate = leaf(p),
    ci = list(
      midp = list(lower = leaf(midp[1], "iterative"),
                  upper = leaf(midp[2], "iterative")),
      clopper_pearson = list(lower = leaf(cp[1], "iterative"),
                             upper = leaf(cp[2], "iterative")),
      wald = list(lower = leaf(wald_l), upper = leaf(wald_u)),
      agresti_coull = list(lower = leaf(ac[1]), upper = leaf(ac[2])),
      wilson = list(lower = leaf(wi[1]), upper = leaf(wi[2])),
      score_cc = { scc <- score_cc_ci(x, n, conf)
                   list(lower = leaf(scc[1]), upper = leaf(scc[2])) }))
  if (pop > 2 && pop < 1e6) {
    wf <- wilson_fpc_ci(x, n, conf, pop)
    expected$ci$wilson_fpc <- list(lower = leaf(wf[1]),
                                   upper = leaf(wf[2]))
  }
  if (!is.null(compare_with)) {
    p0 <- compare_with / 100
    zv <- (p - p0) / sqrt(p0 * (1 - p0) / n)
    expected$ztest <- list(
      z = leaf(zv), p = leaf(2 * pnorm(-abs(zv))))
  }
  expected
}

proportion_write <- function(id, x, n, conf = 95, pop = 1000000,
                             compare_with = NULL, multiplier = 100) {
  inputs <- list(numerator = x, denominator = n, conf_level = conf,
                 multiplier = multiplier, population_size = pop)
  if (!is.null(compare_with)) inputs$compare_with <- compare_with
  build <- function(c) proportion_expected(x, n, c, pop, compare_with)
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("proportion", id, inputs, expected,
             c("stats", "exactci", "PropCIs"))
}

proportion_write("textbook", 10, 45, compare_with = 50)
proportion_write("zero_numerator", 0, 40)
proportion_write("all_positive", 40, 40, compare_with = 50)
proportion_write("finite_population", 30, 100, pop = 500,
                 compare_with = 35)
proportion_write("large", 60000, 100000, pop = 0)

# --- 2x2 machinery -----------------------------------------------------

# Fisher/mid-p 1- and 2-tailed p for a single 2x2 (corpus layout).
# p_1t = the smaller directional tail (OpenEpi's <= rule); p_2t for
# mid-p = 2*min capped at 1 (exact2x2 central); Fisher p_2t =
# stats::fisher.test (probability ordering).
tab_pvals <- function(tab) {
  a <- tab[1, 1]; b <- tab[1, 2]; cc <- tab[2, 1]; d <- tab[2, 2]
  m1 <- a + cc; n1 <- a + b; n0 <- cc + d
  fis <- tryCatch(fisher.test(tab), error = function(e) NULL)
  lo_f <- phyper(a, n1, n0, m1)
  up_f <- phyper(a - 1, n1, n0, m1, lower.tail = FALSE)
  p1_f <- min(up_f, lo_f)
  pm <- dhyper(a, n1, n0, m1) / 2
  lo_m <- phyper(a - 1, n1, n0, m1) + pm
  up_m <- phyper(a, n1, n0, m1, lower.tail = FALSE) + pm
  p1_m <- min(up_m, lo_m)
  list(
    fisher = list(
      p_1t = leaf(p1_f, "iterative"),
      p_2t = leaf(if (is.null(fis)) NA else fis$p.value, "iterative")),
    midp = list(
      p_1t = leaf(p1_m, "iterative"),
      p_2t = leaf(min(1, 2 * p1_m), "iterative")))
}

chi_leaf <- function(tab) {
  cu <- suppressWarnings(chisq.test(tab, correct = FALSE))
  cy <- suppressWarnings(chisq.test(tab, correct = TRUE))
  mh <- mh_2x2xk(array(tab, dim = c(2, 2, 1)), correct = FALSE)
  list(
    uncorrected = list(
      value = leaf(unname(cu$statistic)),
      p_1t = leaf(cu$p.value / 2),
      p_2t = leaf(cu$p.value)),
    yates = list(
      value = leaf(unname(cy$statistic)),
      p_1t = leaf(cy$p.value / 2),
      p_2t = leaf(cy$p.value)),
    mantel_haenszel = list(
      value = leaf(unname(mh$statistic)),
      p_1t = leaf(mh$p.value / 2),
      p_2t = leaf(mh$p.value)))
}

# The two_by_two per-table expected block (risk + odds + exacts +
# etiologic). `id` labels check_tight failures.
two_by_two_table <- function(tab, conf, id) {
  a <- tab[1, 1]; b <- tab[1, 2]; cc <- tab[2, 1]; d <- tab[2, 2]
  n1 <- a + b; n0 <- cc + d; n <- n1 + n0
  pv <- tab_pvals(tab)
  rr <- tryCatch(riskratio(tab, rev = "both", method = "wald",
                           conf.level = conf / 100),
                 error = function(e) NULL)
  orw <- tryCatch(oddsratio(tab, rev = "both", method = "wald",
                            conf.level = conf / 100),
                  error = function(e) NULL)
  # epiR reports risks/RD in percent; scale accordingly.
  rre <- risk_ci(a, n1, conf) * 100
  rru <- risk_ci(cc, n0, conf) * 100
  rro <- risk_ci(a + cc, n, conf) * 100
  e22 <- tryCatch(
    epiR::epi.2by2(tab, outcome = "as.columns", conf.level = conf / 100),
    error = function(e) NULL)
  # epi.2by2 errors on a zero cell; DescTools' Wald BinomDiffCI is the
  # same ARisk.strata.wald computation (bit-identical where epi.2by2
  # runs, checked on the corpus tables) and stays defined there.
  rd_w <- if (!is.null(e22)) {
    unname(unlist(e22$massoc.detail$ARisk.strata.wald))
  } else {
    tryCatch(
      as.numeric(DescTools::BinomDiffCI(
        a, n1, cc, n0, conf.level = conf / 100,
        method = "wald")) * 100,
      error = function(e) rep(NA_real_, 3))
  }
  # The cmle_or.midp estimate is the conditional MLE:
  # stats::fisher.test's $estimate (bit-identical to exact2x2's;
  # 0 / Inf at the support boundaries, which leaf() records as
  # defined 0 / undefined null). or.midp's own estimate is the
  # median-unbiased estimator — wrong for this leaf — but its limits
  # are the mid-p exact limits and stay; where or.midp fails (zero
  # cells) exact2x2's mid-p CI supplies the same quantity.
  cmle_est <- tryCatch(
    unname(fisher.test(tab, conf.level = conf / 100)$estimate),
    error = function(e) NA_real_)
  midp <- tryCatch(or.midp(tab, conf.level = conf / 100),
                   error = function(e) NULL)
  if (is.null(midp)) {
    midp <- tryCatch(
      exact2x2::exact2x2(tab, midp = TRUE, conf.level = conf / 100),
      error = function(e) NULL)
  }
  fis <- tryCatch(exact2x2::exact2x2(tab, tsmethod = "central",
                                     conf.level = conf / 100),
                  error = function(e) NULL)
  efr <- ef_risk(a, b, cc, d, conf)
  efo <- ef_or(a, b, cc, d, conf)
  list(
    chisq = chi_leaf(tab),
    fisher_exact = pv$fisher,
    midp_exact = pv$midp,
    risk = list(
      exposed = lapply(rre, leaf),
      unexposed = lapply(rru, leaf),
      overall = lapply(rro, leaf)),
    risk_ratio = if (is.null(rr)) {
      stat_ci(c(NA, NA, NA))
    } else {
      stat_ci(unname(rr$measure[2, ]))
    },
    risk_difference = list(
      est = leaf(rd_w[1]), lower = leaf(rd_w[2]),
      upper = leaf(rd_w[3])),
    odds_ratio = if (is.null(orw)) {
      stat_ci(c(NA, NA, NA))
    } else {
      stat_ci(unname(orw$measure[2, ]))
    },
    cmle_or = list(
      midp = list(
        est = leaf(cmle_est, "iterative"),
        lower = leaf(if (is.null(midp)) NA else midp$conf.int[1],
                     "iterative"),
        upper = leaf(if (is.null(midp)) NA else midp$conf.int[2],
                     "iterative")),
      fisher = list(
        lower = leaf(if (is.null(fis)) NA else fis$conf.int[1],
                     "iterative"),
        upper = leaf(if (is.null(fis)) NA else fis$conf.int[2],
                     "iterative"))),
    etiologic = lapply(efr, function(v) stat_ci(v * 100)),
    etiologic_or = lapply(efo, function(v) stat_ci(v * 100)))
}

crude_table <- function(strata) {
  m <- Reduce(`+`, strata)
  matrix(m, 2, 2)
}

# Kind of a limit value for boundary agreement in the generation-time
# checks: "undefined" (NA/NaN), "inf", "-inf" or "finite". Non-finite
# port/stock pairs agree only when they are the same kind (the support
# boundary returns 0 / Inf on both).
val_kind <- function(v) {
  if (is.na(v)) "undefined"
  else if (v == Inf) "inf"
  else if (v == -Inf) "-inf"
  else "finite"
}

# Generation-time sanity check (fix round 2): for a stratified
# two_by_two fixture, the recorded `stratified_cmle` estimate and
# Fisher limits must agree with stats::mantelhaen.test(exact = TRUE)
# ($estimate, $conf.int) within 1e-3 relative. Stock finds its
# exact-CI roots at uniroot's default tolerance
# (.Machine$double.eps^0.25, ~1.2e-4), so 1e-3 is a sanity bound at
# stock's own convergence — a conductor-chosen check, not a fixture
# tolerance; it never changes a fixture value. Stock's every-stratum
# n >= 2 guard is already enforced by the non-exact call above.
# Non-finite sides agree only when they are the same kind (the
# boundary returns 0 / Inf on both).
check_cmle_exact <- function(arr, conf, cmle, what) {
  # stats::mantelhaen.test's exact method silently overflows at large
  # counts — measured on this corpus: it still agrees with the port at
  # ~8e4 total counts, drifts past the 1e-3 sanity bound by ~1e5, and
  # returns a garbage estimate (~8e-5 for OR ~0.33 strata) by ~1.3e5.
  # No stock reference exists beyond that, so the check applies only
  # below 5e4 total counts — clear headroom under the measured break.
  if (sum(arr) > 5e4) return(invisible(TRUE))
  stock <- mantelhaen.test(arr, exact = TRUE, conf.level = conf)
  for (p in list(
    list(cmle$est, unname(stock$estimate), "est"),
    list(cmle$fisher_lower, unname(stock$conf.int[1]),
         "fisher.lower"),
    list(cmle$fisher_upper, unname(stock$conf.int[2]),
         "fisher.upper"))) {
    port <- p[[1]]; ref <- p[[2]]
    bad <- if (is.finite(port) && is.finite(ref)) {
      abs(port - ref) > 1e-3 * abs(ref)
    } else {
      val_kind(port) != val_kind(ref)
    }
    stop_if(bad, sprintf(paste0(
      "exact-CMLE sanity check failed for %s %s: port=%s ",
      "mantelhaen.test(exact = TRUE)=%s"), what, p[[3]],
      format(port, digits = 17), format(ref, digits = 17)))
  }
  invisible(TRUE)
}

# Generation-time sanity checks (fix round 3) for the MartinStats
# port's single-stratum paths — the shapes no multi-stratum fixture
# can give stock. A lone 2x2 must reproduce stats::fisher.test's
# conditional-MLE estimate and central limits; a lone person-time
# stratum must reproduce the conditional-binomial identity
# RR = (a/b)*(n0/n1) with stats::binom.test's Clopper-Pearson limits
# mapped p -> p/(1-p)*(n0/n1). Same 1e-3-relative sanity bound as
# check_cmle_exact (stock's own root tolerance; never a fixture
# tolerance) and the same non-finite kind agreement.
check_cmle_single <- function(port_est, port_lo, port_hi, ref_est,
                              ref_lo, ref_hi, what) {
  for (p in list(
    list(port_est, ref_est, "est"),
    list(port_lo, ref_lo, "fisher.lower"),
    list(port_hi, ref_hi, "fisher.upper"))) {
    port <- p[[1]]; ref <- p[[2]]
    bad <- if (is.finite(port) && is.finite(ref)) {
      abs(port - ref) > 1e-3 * abs(ref)
    } else {
      val_kind(port) != val_kind(ref)
    }
    stop_if(bad, sprintf(paste0(
      "single-stratum CMLE sanity check failed for %s %s: port=%s ",
      "stock=%s"), what, p[[3]],
      format(port, digits = 17), format(ref, digits = 17)))
  }
  invisible(TRUE)
}

check_cmle_single_cc <- function(tab, conf, what) {
  if (sum(tab) == 0) return(invisible(TRUE)) # no stock reference
  port <- stratified_cmle_one(list(tab), conf, 1e-14)
  stock <- fisher.test(tab, conf.level = conf)
  check_cmle_single(port$est, port$fisher_lower, port$fisher_upper,
                    unname(stock$estimate), stock$conf.int[1],
                    stock$conf.int[2], what)
}

check_cmle_single_pt <- function(tab, conf, what) {
  a <- tab[1, 1]; b <- tab[1, 2]; n1 <- tab[2, 1]; n0 <- tab[2, 2]
  if (a + b == 0) return(invisible(TRUE)) # no stock reference
  port <- stratified_cmle_pt_one(list(tab), conf, 1e-14)
  ci <- binom.test(a, a + b, conf.level = conf)$conf.int
  map <- function(p) p / (1 - p) * (n0 / n1)
  check_cmle_single(port$est, port$fisher_lower, port$fisher_upper,
                    (a / b) * (n0 / n1), map(ci[1]), map(ci[2]), what)
}

two_by_two_expected <- function(strata, conf, id) {
  # Single-stratum port-vs-fisher.test sanity check on every table
  # that feeds this fixture (fix round 3).
  for (i in seq_along(strata)) {
    check_cmle_single_cc(strata[[i]], conf / 100,
                         paste0(id, ".", i, ".cmle_or"))
  }
  check_cmle_single_cc(crude_table(strata), conf / 100,
                       paste0(id, ".crude.cmle_or"))
  out <- list(
    strata = lapply(seq_along(strata), function(i)
      two_by_two_table(strata[[i]], conf, paste0(id, ".", i))),
    crude = two_by_two_table(crude_table(strata), conf,
                             paste0(id, ".crude")))
  if (length(strata) > 1) {
    arr <- array(unlist(strata), dim = c(2, 2, length(strata)))
    mh <- mantelhaen.test(arr, correct = FALSE, conf.level = conf / 100)
    check_mh_port(arr, paste0(id, ".adjusted"))
    da_or <- direct_adjust(strata, conf, "or")
    da_rr <- direct_adjust(strata, conf, "rr")
    da_rd <- direct_adjust(strata, conf, "rd")
    bd <- breslow_day(strata, conf)
    cmle <- stratified_cmle(strata, conf / 100)
    check_tight(cmle$tight$est, cmle$r_default$est,
                paste0(id, ".adjusted.cmle_or.est"))
    check_tight(cmle$tight$midp_lower, cmle$r_default$midp_lower,
                paste0(id, ".adjusted.cmle_or.midp.lower"))
    check_tight(cmle$tight$midp_upper, cmle$r_default$midp_upper,
                paste0(id, ".adjusted.cmle_or.midp.upper"))
    check_tight(cmle$tight$fisher_lower, cmle$r_default$fisher_lower,
                paste0(id, ".adjusted.cmle_or.fisher.lower"))
    check_tight(cmle$tight$fisher_upper, cmle$r_default$fisher_upper,
                paste0(id, ".adjusted.cmle_or.fisher.upper"))
    # Sanity check against stock's exact CMLE at stock's own
    # convergence (1e-3 relative; check only — fixture keeps the
    # tight values).
    check_cmle_exact(arr, conf / 100, cmle$tight,
                     paste0(id, ".adjusted.cmle_or"))
    # Uncorrected Mantel-Haenszel OR: stats::mantelhaen.test's
    # estimate and Robins-Breslow-Greenland CI (the mh handle above).
    # epiR::epi.mh adds 0.5 to every cell of a zero-cell stratum,
    # which changes the estimator — the MH estimator itself is
    # uncorrected (stock estimate 0.2420575 on one_inside_boundary,
    # OpenEpi's displayed 0.2421). The uncorrected MH RR port with
    # the Greenland-Robins CI is checked bit for bit against epi.mh
    # wherever no zero cell exists (check_mh_rr_port).
    rr_mh <- mh_rr_2x2xk(strata, conf)
    check_mh_rr_port(strata, conf, rr_mh,
                     paste0(id, ".adjusted.mh_rr"))
    out$adjusted <- list(
      mh_chisq = list(
        value = leaf(unname(mh$statistic)),
        p = leaf(mh$p.value)),
      mh_or = list(
        est = leaf(unname(mh$estimate)),
        lower = leaf(mh$conf.int[1]),
        upper = leaf(mh$conf.int[2])),
      direct_or = lapply(da_or, leaf),
      direct_rr = lapply(da_rr, leaf),
      direct_rd = lapply(da_rd, leaf),
      mh_rr = list(
        est = leaf(rr_mh["est"]),
        lower = leaf(rr_mh["lower"]),
        upper = leaf(rr_mh["upper"])),
      breslow_day = lapply(bd, leaf),
      cmle_or = list(
        est = leaf(cmle$tight$est, "iterative",
                   r_default = cmle$r_default$est),
        midp = list(
          lower = leaf(cmle$tight$midp_lower, "iterative",
                       r_default = cmle$r_default$midp_lower),
          upper = leaf(cmle$tight$midp_upper, "iterative",
                       r_default = cmle$r_default$midp_upper)),
        fisher = list(
          lower = leaf(cmle$tight$fisher_lower, "iterative",
                       r_default = cmle$r_default$fisher_lower),
          upper = leaf(cmle$tight$fisher_upper, "iterative",
                       r_default = cmle$r_default$fisher_upper))))
  }
  out
}

two_by_two_write <- function(id, strata, conf = 95,
                             study_type = "cohort") {
  inputs <- list(strata = lapply(strata, function(t) t),
                 conf_level = conf, study_type = study_type)
  build <- function(c) two_by_two_expected(strata, c, id)
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("two_by_two", id, inputs, expected,
             c("stats", "epitools", "exact2x2", "epiR", "DescTools"))
}

# --- two_by_n (EI7 StatCalc SingleTableResults set) --------------------

two_by_n_table <- function(tab, conf, id) {
  a <- tab[1, 1]; b <- tab[1, 2]; cc <- tab[2, 1]; d <- tab[2, 2]
  n1 <- a + b; n0 <- cc + d
  pv <- tab_pvals(tab)
  rr <- tryCatch(riskratio(tab, rev = "both", method = "wald",
                           conf.level = conf / 100),
                 error = function(e) NULL)
  orw <- tryCatch(oddsratio(tab, rev = "both", method = "wald",
                            conf.level = conf / 100),
                  error = function(e) NULL)
  mle <- tryCatch(
    exact2x2::exact2x2(tab, midp = TRUE, conf.level = conf / 100),
    error = function(e) NULL)
  fis <- tryCatch(exact2x2::exact2x2(tab, tsmethod = "central",
                                     conf.level = conf / 100),
                  error = function(e) NULL)
  cu <- suppressWarnings(chisq.test(tab, correct = FALSE))
  cy <- suppressWarnings(chisq.test(tab, correct = TRUE))
  mh <- mh_2x2xk(array(tab, dim = c(2, 2, 1)), correct = FALSE)
  check_mh_single_table(tab, unname(mh$statistic), paste0(id, " chisq.mantel"))
  e22 <- tryCatch(
    epiR::epi.2by2(tab, outcome = "as.columns", conf.level = conf / 100),
    error = function(e) NULL)
  # Same zero-cell fallback as two_by_two_table: DescTools' Wald
  # BinomDiffCI is bit-identical to epi.2by2's ARisk.strata.wald and
  # stays defined where epi.2by2 errors.
  rd_w <- if (!is.null(e22)) {
    unname(unlist(e22$massoc.detail$ARisk.strata.wald))
  } else {
    tryCatch(
      as.numeric(DescTools::BinomDiffCI(
        a, n1, cc, n0, conf.level = conf / 100,
        method = "wald")) * 100,
      error = function(e) rep(NA_real_, 3))
  }
  list(
    odds_ratio = if (is.null(orw)) stat_ci(c(NA, NA, NA)) else
      stat_ci(unname(orw$measure[2, ])),
    odds_ratio_mle = list(
      midp = list(
        est = leaf(if (is.null(mle)) NA else mle$estimate, "iterative"),
        lower = leaf(if (is.null(mle)) NA else mle$conf.int[1],
                     "iterative"),
        upper = leaf(if (is.null(mle)) NA else mle$conf.int[2],
                     "iterative")),
      fisher = list(
        lower = leaf(if (is.null(fis)) NA else fis$conf.int[1],
                     "iterative"),
        upper = leaf(if (is.null(fis)) NA else fis$conf.int[2],
                     "iterative"))),
    risk_ratio = if (is.null(rr)) stat_ci(c(NA, NA, NA)) else
      stat_ci(unname(rr$measure[2, ])),
    risk_difference = list(
      est = leaf(rd_w[1]), lower = leaf(rd_w[2]),
      upper = leaf(rd_w[3])),
    chisq = list(
      uncorrected = list(value = leaf(unname(cu$statistic)),
                         p_2t = leaf(cu$p.value)),
      mantel = list(value = leaf(unname(mh$statistic)),
                    p_2t = leaf(mh$p.value)),
      yates = list(value = leaf(unname(cy$statistic)),
                   p_2t = leaf(cy$p.value))),
    exact = list(
      midp_2t = leaf(pv$midp$p_2t$value, "iterative"),
      fisher_1t = leaf(pv$fisher$p_1t$value, "iterative"),
      fisher_2t = leaf(pv$fisher$p_2t$value, "iterative")))
}

two_by_n_write <- function(id, strata, conf = 95) {
  inputs <- list(strata = strata, conf_level = conf)
  build <- function(c) list(
    strata = lapply(seq_along(strata), function(i)
      two_by_n_table(strata[[i]], c, paste0(id, ".", i))),
    crude = two_by_n_table(crude_table(strata), c,
                           paste0(id, ".crude")))
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("two_by_n", id, inputs, expected,
             c("stats", "epitools", "exact2x2", "epiR", "DescTools"))
}

# --- r_by_c ---------------------------------------------------------------

r_by_c_write <- function(id, tab) {
  ct <- suppressWarnings(chisq.test(tab, correct = FALSE))
  ex <- ct$expected
  expected <- list(
    chisq = list(
      value = leaf(unname(ct$statistic)),
      df = leaf(unname(ct$parameter)),
      p = leaf(ct$p.value)),
    expected_lt5 = list(
      count = leaf(sum(ex < 5)),
      percent = leaf(sum(ex < 5) / length(ex) * 100)),
    expected_lt1 = list(
      count = leaf(sum(ex < 1))))
  calc_write("r_by_c", id, list(table = tab), expected, c("stats"))
}

# --- dose_response --------------------------------------------------------

# inputs$strata: array of level-tables, each [[score, cases, controls]]
# rows = exposure levels (baseline first). 3 columns per row.
dr_strata_df <- function(strata) {
  lapply(strata, function(s) {
    data.frame(score = s[, 1], cases = s[, 2], controls = s[, 3])
  })
}

# Wald OR of level i vs the baseline row via epiR::epi.2by2 (the
# inventory's named function); the baseline row returns NA (1 vs
# itself is a fixed 1 — leaf emits it as a defined 1 for row 1).
dr_or_levels <- function(df, conf) {
  abase <- df$cases[1]; bbase <- df$controls[1]
  vapply(seq_len(nrow(df)), function(i) {
    tab <- matrix(c(df$cases[i], df$controls[i], abase, bbase),
                  2, 2, byrow = TRUE)
    e22 <- tryCatch(
      epiR::epi.2by2(tab, outcome = "as.columns",
                     conf.level = conf / 100),
      error = function(e) NULL)
    if (is.null(e22)) {
      return((df$cases[i] * bbase) / (df$controls[i] * abase))
    }
    unname(e22$massoc.detail$OR.strata.wald$est)
  }, numeric(1))
}

# MH summary OR of level i vs baseline pooled over strata: the
# per-level 2x2xk array through stats::mantelhaen.test.
dr_mh_or_est <- function(dfs) {
  nlev <- nrow(dfs[[1]])
  out <- numeric(nlev - 1)
  for (i in 2:nlev) {
    arrs <- lapply(dfs, function(d) {
      matrix(c(d$cases[i], d$controls[i], d$cases[1], d$controls[1]),
             2, 2, byrow = TRUE)
    })
    arr <- array(unlist(arrs), dim = c(2, 2, length(arrs)))
    # mh_2x2xk: the mantelhaen.test port (stock errors on K = 1).
    out[i - 1] <- unname(mh_2x2xk(arr)$estimate)
  }
  out
}

dose_response_write <- function(id, strata, conf = 95) {
  dfs <- dr_strata_df(strata)
  crude_df <- data.frame(
    score = dfs[[1]]$score,
    cases = Reduce(`+`, lapply(dfs, function(d) d$cases)),
    controls = Reduce(`+`, lapply(dfs, function(d) d$controls)))
  xmh_cc <- xmh_trend(dfs, correct = TRUE)
  xmh_u <- xmh_trend(dfs, correct = FALSE)
  build <- function(c) list(
    strata = lapply(seq_along(dfs), function(i) list(
      odds_ratio = lapply(dr_or_levels(dfs[[i]], c), leaf))),
    crude = list(
      odds_ratio = lapply(dr_or_levels(crude_df, c), leaf)),
    mh_or = lapply(dr_mh_or_est(dfs), leaf),
    xmh = list(
      cc = list(value = leaf(xmh_cc$value), p = leaf(xmh_cc$p)),
      uncorrected = list(value = leaf(xmh_u$value),
                         p = leaf(xmh_u$p))))
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  inputs <- list(table = strata, strata = length(strata))
  calc_write("dose_response", id, inputs, expected,
             c("stats", "epiR"))
}

# --- matched_case_control -------------------------------------------------

matched_expected <- function(matched_sets, controls_per_case, conf, id) {
  expected <- list()
  if (controls_per_case == 1) {
    a <- matched_sets[1, 1]; b <- matched_sets[1, 2]
    c <- matched_sets[2, 1]; d <- matched_sets[2, 2]
    # rows = case exposed/unexposed; cols = exposed controls count
    # (0..1): [[concordant exposed, discordant b],[discordant c,
    # concordant unexposed]]
    disc_b <- matched_sets[1, 2]
    disc_c <- matched_sets[2, 1]
    mu <- suppressWarnings(
      mcnemar.test(matrix(c(a, disc_c, disc_b, d), 2, 2),
                   correct = FALSE))
    mc <- suppressWarnings(
      mcnemar.test(matrix(c(a, disc_c, disc_b, d), 2, 2),
                   correct = TRUE))
    or_t <- matched_or_taylor(disc_b, disc_c, conf)
    e2m <- tryCatch(exact2x2::exact2x2(
      matrix(c(a, disc_c, disc_b, d), 2, 2), paired = TRUE,
      midp = TRUE, conf.level = conf / 100), error = function(e) NULL)
    e2f <- tryCatch(exact2x2::exact2x2(
      matrix(c(a, disc_c, disc_b, d), 2, 2), paired = TRUE,
      tsmethod = "central", conf.level = conf / 100),
      error = function(e) NULL)
    bt <- binom.test(disc_b, disc_b + disc_c,
                     conf.level = conf / 100)
    b1 <- min(pbinom(disc_b, disc_b + disc_c, 0.5),
              pbinom(disc_b - 1, disc_b + disc_c, 0.5,
                     lower.tail = FALSE))
    expected <- list(
      mcnemar = list(value = leaf(unname(mu$statistic)),
                     p = leaf(mu$p.value)),
      mcnemar_cc = list(value = leaf(unname(mc$statistic)),
                        p = leaf(mc$p.value)),
      or = lapply(or_t, leaf),
      cmle_or = list(
        midp = list(
          est = leaf(if (is.null(e2m)) NA else e2m$estimate,
                     "iterative"),
          lower = leaf(if (is.null(e2m)) NA else e2m$conf.int[1],
                       "iterative"),
          upper = leaf(if (is.null(e2m)) NA else e2m$conf.int[2],
                       "iterative")),
        fisher = list(
          lower = leaf(if (is.null(e2f)) NA else e2f$conf.int[1],
                       "iterative"),
          upper = leaf(if (is.null(e2f)) NA else e2f$conf.int[2],
                       "iterative"))),
      exact = list(
        midp_p = leaf(if (is.null(e2m)) NA else e2m$p.value,
                      "iterative"),
        fisher_p_1t = leaf(b1, "iterative"),
        fisher_p_2t = leaf(bt$p.value, "iterative")),
      discordant_pairs = leaf(discordant_pairs(disc_b, disc_c)))
  }
  if (controls_per_case >= 2) {
    # 1:n matched sets: each set becomes one 2x2 stratum
    # (case vs pooled controls x exposed/unexposed).
    nsets <- 0
    arrs <- list()
    for (case_exp in 1:2) {
      for (k in 0:controls_per_case) {
        cnt <- matched_sets[case_exp, k + 1]
        if (cnt > 0) {
          for (rep_i in seq_len(cnt)) {
            # case row: exposed iff case_exp == 1 (ill side = exposed
            # member); controls: k exposed of M.
            if (case_exp == 1) {
              tab <- matrix(c(1, 0, k, controls_per_case - k), 2, 2)
            } else {
              tab <- matrix(c(0, 1, k, controls_per_case - k), 2, 2)
            }
            arrs[[length(arrs) + 1]] <- tab
          }
        }
      }
    }
    arr <- array(unlist(arrs), dim = c(2, 2, length(arrs)))
    mh <- mantelhaen.test(arr, correct = FALSE, conf.level = conf / 100)
    mhc <- mantelhaen.test(arr, correct = TRUE, conf.level = conf / 100)
    # clogit: one stratum per set; case first with its exposure flag,
    # then M controls, the first k exposed (order irrelevant).
    rows <- list()
    for (case_exp in 1:2) {
      for (k in 0:controls_per_case) {
        cnt <- matched_sets[case_exp, k + 1]
        if (cnt > 0) {
          for (rep_i in seq_len(cnt)) {
            sid <- length(rows) + 1
            rows[[sid]] <- data.frame(
              stratum = sid,
              case = c(1, rep(0, controls_per_case)),
              exposed = c(case_exp - 1,
                          c(rep(1, k),
                            rep(0, controls_per_case - k))))
          }
        }
      }
    }
    sets_df <- do.call(rbind, rows)
    cl <- tryCatch(
      survival::clogit(case ~ exposed + strata(stratum),
                       data = sets_df),
      error = function(e) NULL)
    z <- qnorm((1 + conf / 100) / 2)
    expected$matched_n <- list(
      mh_or = list(
        est = leaf(unname(mh$estimate)),
        lower = leaf(mh$conf.int[1]),
        upper = leaf(mh$conf.int[2])),
      mh_chisq = list(
        value = leaf(unname(mh$statistic)),
        p = leaf(mh$p.value)),
      mh_chisq_cc = list(
        value = leaf(unname(mhc$statistic)),
        p = leaf(mhc$p.value)),
      cmle_or = if (is.null(cl)) {
        list(est = leaf(NA, "iterative"),
             lower = leaf(NA, "iterative"),
             upper = leaf(NA, "iterative"))
      } else {
        co <- summary(cl)$coefficients
        list(
          est = leaf(exp(co[1, "coef"]), "iterative"),
          lower = leaf(exp(co[1, "coef"] - z * co[1, "se(coef)"]),
                       "iterative"),
          upper = leaf(exp(co[1, "coef"] + z * co[1, "se(coef)"]),
                       "iterative"))
      })
  }
  expected
}

matched_write <- function(id, matched_sets, controls_per_case = 1,
                          conf = 95) {
  inputs <- list(controls_per_case = controls_per_case,
                 matched_sets = matched_sets, conf_level = conf)
  build <- function(c) {
    matched_expected(matched_sets, controls_per_case, c, id)
  }
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("matched_case_control", id, inputs, expected,
             c("stats", "exact2x2", "survival"))
}

# --- screening ------------------------------------------------------------

wilson_stat <- function(x, n, conf) {
  ci <- PropCIs::scoreci(x, n, conf.level = conf / 100)$conf.int
  list(est = leaf(x / n), lower = leaf(ci[1]), upper = leaf(ci[2]))
}

screening_expected <- function(tab, conf) {
  # rows = test levels; col1 = diseased+, col2 = diseased-.
  # 2x2 cut: row1+? For k=2 rows are +/-. For k>2 the outputs below
  # use the full table's column totals.
  tp <- tab[1, 1]; fp <- tab[1, 2]
  fn <- sum(tab[-1, 1]); tn <- sum(tab[-1, 2])
  big_d <- tp + fn; big_nd <- fp + tn
  tpos <- tp + fp; tneg <- fn + tn; n <- big_d + big_nd
  z <- qnorm((1 + conf / 100) / 2)

  et <- epiR::epi.tests(matrix(c(tp, fp, fn, tn), 2, 2, byrow = TRUE),
                        conf.level = conf / 100)
  lrpos <- et$detail[et$detail$statistic == "lr.pos", ]
  lrneg <- et$detail[et$detail$statistic == "lr.neg", ]
  kap <- epiR::epi.kappa(matrix(c(tp, fp, fn, tn), 2, 2, byrow = TRUE),
                         conf.level = conf / 100)
  dodds <- diagnostic_odds(tp, fp, fn, tn, conf)
  sn <- tp / big_d; sp <- tn / big_nd
  ppv <- tp / tpos; npv <- tn / tneg
  ent <- entropy_reduction(big_d / n, ppv, npv)

  expected <- list(
    sensitivity = wilson_stat(tp, big_d, conf),
    specificity = wilson_stat(tn, big_nd, conf),
    ppv = wilson_stat(tp, tpos, conf),
    npv = wilson_stat(tn, tneg, conf),
    accuracy = wilson_stat(tp + tn, n, conf),
    lr_pos = lapply(lrpos[, c("est", "lower", "upper")], leaf),
    lr_neg = lapply(lrneg[, c("est", "lower", "upper")], leaf),
    diagnostic_odds = lapply(dodds, leaf),
    kappa = list(
      est = leaf(kap$kappa$est),
      lower = leaf(kap$kappa$lower),
      upper = leaf(kap$kappa$upper)),
    entropy = list(
      positive = leaf(ent$positive),
      negative = leaf(ent$negative)),
    bias_index = leaf(bias_index(fp, fn, n)))
  k <- nrow(tab)
  if (k > 2) {
    expected$level_lr <- lapply(seq_len(k), function(i) {
      lapply(level_lr(tab[i, 1], tab[i, 2], big_d, big_nd, conf),
             leaf)
    })
  }
  expected
}

screening_write <- function(id, tab, conf = 95) {
  build <- function(c) screening_expected(tab, c)
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("screening", id, list(table = tab, conf_level = conf),
             expected, c("stats", "PropCIs", "epiR"))
}

# --- smr ------------------------------------------------------------------

smr_expected <- function(o, e, conf) {
  cl <- conf / 100
  midp <- exactci::poisson.exact(o, e, conf.level = cl, midp = TRUE)
  fis <- exactci::poisson.exact(o, e, conf.level = cl)
  by <- epitools::pois.byar(o, e, conf = cl)
  rg <- rg_ci(o, e, conf)
  uw <- ury_wiggins(o, e, conf)
  vb <- vandenbroucke(o, e, conf)
  bs <- byar_stat(o, e)
  cs <- (o - e)^2 / e
  smr <- epiR::epi.smr(o, e, conf.level = cl)
  list(
    estimate = leaf(smr$est),
    test = list(
      midp_p = leaf(midp$p.value, "iterative"),
      fisher_p = leaf(fis$p.value, "iterative"),
      byar_chisq = leaf(bs$chisq),
      byar_p = leaf(bs$p),
      chisq_value = leaf(cs),
      chisq_p = leaf(pchisq(cs, 1, lower.tail = FALSE))),
    ci = list(
      midp = list(lower = leaf(midp$conf.int[1], "iterative"),
                  upper = leaf(midp$conf.int[2], "iterative")),
      fisher = list(lower = leaf(fis$conf.int[1], "iterative"),
                    upper = leaf(fis$conf.int[2], "iterative")),
      byar = list(lower = leaf(by$lower), upper = leaf(by$upper)),
      rothman_greenland = list(lower = leaf(rg[1]),
                               upper = leaf(rg[2])),
      ury_wiggins = list(lower = leaf(uw[1]), upper = leaf(uw[2])),
      vandenbroucke = list(lower = leaf(vb[1]),
                           upper = leaf(vb[2]))))
}

smr_write <- function(id, o, e, conf = 95) {
  build <- function(c) smr_expected(o, e, c)
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("smr", id,
             list(observed = o, expected = e, conf_level = conf),
             expected,
             c("stats", "exactci", "epitools", "epiR"))
}

# --- binomial --------------------------------------------------------------

binomial_write <- function(id, x, n, expected_percent) {
  p <- expected_percent / 100
  cp_lo <- if (x == 0) 0 else qbeta(0.025, x, n - x + 1)
  cp_hi <- if (x == n) 1 else qbeta(0.975, x + 1, n - x)
  le <- pbinom(x, n, p)
  ge <- pbinom(x - 1, n, p, lower.tail = FALSE)
  expected <- list(
    tail = list(
      lt = leaf(pbinom(x - 1, n, p)),
      le = leaf(le),
      ge = leaf(ge),
      gt = leaf(pbinom(x, n, p, lower.tail = FALSE)),
      eq = leaf(dbinom(x, n, p))),
    p_2t = leaf(min(1, 2 * min(le, ge))),
    ci_95_count = list(
      lower = leaf(round(cp_lo * n)),
      upper = leaf(round(cp_hi * n))))
  calc_write("binomial", id,
             list(numerator = x, denominator = n,
                  expected_percent = expected_percent),
             expected, c("stats"))
}

# --- poisson ----------------------------------------------------------------

poisson_write <- function(id, o, e) {
  expected <- list(
    tail = list(
      lt = leaf(ppois(o - 1, e)),
      le = leaf(ppois(o, e)),
      ge = leaf(ppois(o - 1, e, lower.tail = FALSE)),
      gt = leaf(ppois(o, e, lower.tail = FALSE)),
      eq = leaf(dpois(o, e))))
  calc_write("poisson", id, list(observed = o, expected = e),
             expected, c("stats"))
}

# --- rate_one --------------------------------------------------------------

rate_one_write <- function(id, cases, pt, per_unit = 1000, conf = 95) {
  build <- function(c) {
    cl <- c / 100
    midp <- exactci::poisson.exact(cases, pt, conf.level = cl,
                                   midp = TRUE)
    fis <- exactci::poisson.exact(cases, pt, conf.level = cl)
    na_ <- epitools::pois.approx(cases, pt, conf = cl)
    by <- epitools::pois.byar(cases, pt, conf = cl)
    rg <- rg_ci(cases, pt, c)
    list(
      estimate = leaf(cases / pt * per_unit),
      ci = list(
        midp = list(
          lower = leaf(midp$conf.int[1] * per_unit, "iterative"),
          upper = leaf(midp$conf.int[2] * per_unit, "iterative")),
        fisher = list(
          lower = leaf(fis$conf.int[1] * per_unit, "iterative"),
          upper = leaf(fis$conf.int[2] * per_unit, "iterative")),
        normal = list(lower = leaf(na_$lower * per_unit),
                      upper = leaf(na_$upper * per_unit)),
        byar = list(lower = leaf(by$lower * per_unit),
                    upper = leaf(by$upper * per_unit)),
        rothman_greenland = list(lower = leaf(rg[1] * per_unit),
                                 upper = leaf(rg[2] * per_unit))))
  }
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("rate_one", id,
             list(cases = cases, person_time = pt, per_unit = per_unit,
                  conf_level = conf),
             expected, c("stats", "exactci", "epitools"))
}

# --- rate_compare ----------------------------------------------------------

pt_rate_stat <- function(cases, pt, per_unit, conf) {
  pa <- epitools::pois.approx(cases, pt, conf = conf / 100)
  list(est = leaf(cases / pt * per_unit),
       lower = leaf(pa$lower * per_unit),
       upper = leaf(pa$upper * per_unit))
}

pt_zscore <- function(a, b, n1, n0) {
  m1 <- a + b; n <- n1 + n0
  z <- (a - n1 * m1 / n) / sqrt(m1 * n1 * n0 / (n * n))
  p <- pnorm(-abs(z))
  list(value = leaf(z), p_1t = leaf(p), p_2t = leaf(2 * p))
}

rate_compare_table <- function(tab, per_unit, conf, id) {
  a <- tab[1, 1]; b <- tab[1, 2]; n1 <- tab[2, 1]; n0 <- tab[2, 2]
  # rateratio takes [[count, person_time], ...] rows (exposed first).
  rrw <- tryCatch(
    epitools::rateratio(matrix(c(a, n1, b, n0), 2, 2, byrow = TRUE),
                        method = "wald", rev = "both",
                        conf.level = conf / 100),
    error = function(e) NULL)
  rd <- rd_byar(a, b, n1, n0, conf)
  cmle <- cmle_rr(list(tab), conf / 100)
  check_tight(cmle$tight$est, cmle$r_default$est,
              paste0(id, " cmle_rr.est"))
  check_tight(cmle$tight$midp_lower, cmle$r_default$midp_lower,
              paste0(id, " cmle_rr.midp_lower"))
  check_tight(cmle$tight$midp_upper, cmle$r_default$midp_upper,
              paste0(id, " cmle_rr.midp_upper"))
  check_tight(cmle$tight$fisher_lower, cmle$r_default$fisher_lower,
              paste0(id, " cmle_rr.fisher_lower"))
  check_tight(cmle$tight$fisher_upper, cmle$r_default$fisher_upper,
              paste0(id, " cmle_rr.fisher_upper"))
  # Single-stratum port-vs-binom.test sanity check (fix round 3).
  check_cmle_single_pt(tab, conf / 100, paste0(id, " cmle_rr"))
  efp <- ef_person_time(a, b, n1, n0, conf)
  list(
    zscore = pt_zscore(a, b, n1, n0),
    rate = list(
      exposed = pt_rate_stat(a, n1, per_unit, conf),
      unexposed = pt_rate_stat(b, n0, per_unit, conf),
      overall = pt_rate_stat(a + b, n1 + n0, per_unit, conf)),
    rate_ratio = if (is.null(rrw)) stat_ci(c(NA, NA, NA)) else
      stat_ci(unname(rrw$measure[2, ])),
    rate_difference = lapply(rd * per_unit, leaf),
    cmle_rr = list(
      est = leaf(cmle$tight$est, "iterative",
                 r_default = cmle$r_default$est),
      midp_lower = leaf(cmle$tight$midp_lower, "iterative",
                        r_default = cmle$r_default$midp_lower),
      midp_upper = leaf(cmle$tight$midp_upper, "iterative",
                        r_default = cmle$r_default$midp_upper),
      fisher_lower = leaf(cmle$tight$fisher_lower, "iterative",
                          r_default = cmle$r_default$fisher_lower),
      fisher_upper = leaf(cmle$tight$fisher_upper, "iterative",
                          r_default = cmle$r_default$fisher_upper)),
    etiologic = lapply(efp, function(v) stat_ci(v * 100)))
}

rate_compare_write <- function(id, strata, per_unit = 1000, conf = 95) {
  build <- function(c) {
    out <- list(
      strata = lapply(seq_along(strata), function(i)
        rate_compare_table(strata[[i]], per_unit, c,
                           paste0(id, ".", i))),
      crude = rate_compare_table(crude_table(strata), per_unit, c,
                                 paste0(id, ".crude")))
    if (length(strata) > 1) {
      adj <- adjust(strata, c)
      bd <- breslow_day_rates(strata, c)
      out$adjusted <- list(
        zscore = list(value = leaf(adj$zscore[1]),
                      p = leaf(adj$zscore[2])),
        direct_rr = lapply(adj$direct_rr, leaf),
        mh_rr = lapply(adj$mh_rr, leaf),
        direct_rd = lapply(adj$direct_rd, leaf),
        breslow_day = lapply(bd, leaf))
    }
    out
  }
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("rate_compare", id,
             list(strata = strata, per_unit = per_unit,
                  conf_level = conf),
             expected, c("stats", "epitools"))
}

# --- continuous -------------------------------------------------------------

mean_ci_write <- function(id, mean, sd_or, sd_value, n,
                          population_size = 999999999, conf = 95) {
  sd <- switch(sd_or,
               sd = sd_value,
               se = sd_value * sqrt(n),
               variance = sqrt(sd_value))
  pop_finite <- population_size > 0 && population_size < 999999999
  build <- function(c) {
    zci <- mean_ci(mean, sd, n, c,
                   if (pop_finite) population_size else Inf)
    tc <- qt((1 + c / 100) / 2, df = n - 1)
    # The t-row applies the same finite-population correction the
    # z-row gets inside mean_ci (OpenEpi CIMean.js:161 applies it to
    # both).
    se_t <- sd / sqrt(n)
    if (pop_finite) {
      se_t <- se_t *
        sqrt((population_size - n) / (population_size - 1))
    }
    half <- tc * se_t
    list(
      z = list(lower = leaf(zci[1]), upper = leaf(zci[2])),
      t = list(lower = leaf(mean - half), upper = leaf(mean + half)))
  }
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("mean_ci", id,
             list(mean = mean, sd_or = sd_or, sd_value = sd_value,
                  n = n, population_size = population_size,
                  conf_level = conf),
             expected, c("stats"))
}

median_ci_write <- function(id, n, percentile, conf = 95) {
  build <- function(c) {
    pc <- percentile_ci(n, percentile, c)
    list(
      normal = list(lower_rank = leaf(pc[1]),
                    upper_rank = leaf(pc[2])))
  }
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("median_ci", id,
             list(n = n, percentile = percentile, conf_level = conf),
             expected, c("stats"))
}

t_test_summary_write <- function(id, n1, mean1, sd1, n2, mean2, sd2,
                                 conf = 95) {
  diff <- mean1 - mean2
  sp2 <- ((n1 - 1) * sd1^2 + (n2 - 1) * sd2^2) / (n1 + n2 - 2)
  se_p <- sqrt(sp2 * (1 / n1 + 1 / n2))
  t_p <- diff / se_p; df_p <- n1 + n2 - 2
  se_w <- sqrt(sd1^2 / n1 + sd2^2 / n2)
  t_w <- diff / se_w
  df_w <- (sd1^2 / n1 + sd2^2 / n2)^2 /
    ((sd1^2 / n1)^2 / (n1 - 1) + (sd2^2 / n2)^2 / (n2 - 1))
  f <- sd1^2 / sd2^2
  pf_f <- pf(f, n1 - 1, n2 - 1)
  build <- function(c) {
    cl <- c / 100
    tc_p <- qt(1 - (1 - cl) / 2, df_p)
    tc_w <- qt(1 - (1 - cl) / 2, df_w)
    tset <- function(t_, df_, tc_, se_) list(
      t = leaf(t_), df = leaf(df_),
      p_2t = leaf(2 * pt(-abs(t_), df_)),
      diff = leaf(diff),
      lower = leaf(diff - tc_ * se_),
      upper = leaf(diff + tc_ * se_))
    list(
      pooled = tset(t_p, df_p, tc_p, se_p),
      welch = tset(t_w, df_w, tc_w, se_w),
      f_variance = list(
        f = leaf(f), df_num = leaf(n1 - 1), df_den = leaf(n2 - 1),
        p = leaf(2 * min(pf_f, 1 - pf_f))))
  }
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("t_test_summary", id,
             list(conf_level = conf,
                  n_1 = n1, mean_1 = mean1, sd_1 = sd1,
                  n_2 = n2, mean_2 = mean2, sd_2 = sd2),
             expected, c("stats"))
}

anova_summary_write <- function(id, tab) {
  an <- anova_from_summary(tab)
  bt <- bartlett_from_summary(tab)
  n <- tab[, 1]; m <- tab[, 2]; s <- tab[, 3]
  ms_w <- an$ms_within
  tc_i <- qt(0.975, df = n - 1)
  tc_p <- qt(0.975, df = an$df_within)
  expected <- list(
    anova = lapply(an, leaf),
    bartlett = lapply(bt, leaf),
    group_ci = lapply(seq_len(nrow(tab)), function(i) list(
      lower = leaf(m[i] - tc_i[i] * s[i] / sqrt(n[i])),
      upper = leaf(m[i] + tc_i[i] * s[i] / sqrt(n[i])))),
    group_ci_pooled = lapply(seq_len(nrow(tab)), function(i) list(
      lower = leaf(m[i] - tc_p * sqrt(ms_w / n[i])),
      upper = leaf(m[i] + tc_p * sqrt(ms_w / n[i])))))
  calc_write("anova_summary", id, list(table = tab), expected,
             c("stats"))
}

# --- sample size & power ----------------------------------------------------

ss_proportion_write <- function(id, pop, freq, margin, deff,
                                clusters = 1) {
  on <- ss_prop_openepi(pop, freq, margin, deff)
  ei <- ss_prop_ei7(pop, freq, margin, deff, clusters)
  expected <- list(
    n_openepi = lapply(on, leaf),
    n_ei7 = lapply(ei$n_ei7, leaf),
    cluster_size = lapply(ei$cluster_size, leaf),
    total = lapply(ei$total, leaf))
  calc_write("ss_proportion", id,
             list(population_size = pop, expected_freq = freq,
                  margin_error = margin, design_effect = deff,
                  clusters = clusters),
             expected, c("stats"))
}

ss_cc_write <- function(id, conf, power, r, p2_pct, p1_pct = NULL,
                        odds_ratio = NULL) {
  p2 <- p2_pct / 100
  p1 <- if (!is.null(odds_ratio)) {
    p2 * odds_ratio / (1 + p2 * (odds_ratio - 1))
  } else {
    p1_pct / 100
  }
  build <- function(c) {
    ke <- kelsey(c, power, r, p2, p1)
    fl <- fleiss(c, power, r, p2, p1)
    fc <- fleiss_cc(c, power, r, p2, p1)
    list(
      kelsey = list(cases = leaf(ke[1]), controls = leaf(ke[2]),
                    total = leaf(ke[3])),
      fleiss = list(cases = leaf(fl[1]), controls = leaf(fl[2]),
                    total = leaf(fl[3])),
      fleiss_cc = list(cases = leaf(fc[1]), controls = leaf(fc[2]),
                       total = leaf(fc[3])))
  }
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  inputs <- list(conf_level = conf, power = power,
                 ratio_controls_per_case = r,
                 percent_controls_exposed = p2_pct)
  if (!is.null(odds_ratio)) inputs$odds_ratio <- odds_ratio
  else inputs$percent_cases_exposed <- p1_pct
  calc_write("ss_unmatched_cc", id, inputs, expected, c("stats"))
}

ss_cohort_write <- function(id, conf, power, r, p2_pct,
                            percent_exposed = NULL, odds_ratio = NULL,
                            risk_ratio = NULL, risk_difference = NULL) {
  p2 <- p2_pct / 100
  p1 <- if (!is.null(odds_ratio)) {
    p2 * odds_ratio / (1 + p2 * (odds_ratio - 1))
  } else if (!is.null(risk_ratio)) {
    p2 * risk_ratio
  } else if (!is.null(risk_difference)) {
    p2 + risk_difference / 100
  } else {
    percent_exposed / 100
  }
  build <- function(c) {
    ke <- kelsey(c, power, r, p2, p1)
    fl <- fleiss(c, power, r, p2, p1)
    fc <- fleiss_cc(c, power, r, p2, p1)
    list(
      kelsey = list(exposed = leaf(ke[1]), unexposed = leaf(ke[2]),
                    total = leaf(ke[3])),
      fleiss = list(exposed = leaf(fl[1]), unexposed = leaf(fl[2]),
                    total = leaf(fl[3])),
      fleiss_cc = list(exposed = leaf(fc[1]), unexposed = leaf(fc[2]),
                       total = leaf(fc[3])))
  }
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  inputs <- list(conf_level = conf, power = power,
                 ratio_unexposed_per_exposed = r,
                 percent_unexposed_outcome = p2_pct)
  if (!is.null(odds_ratio)) inputs$odds_ratio <- odds_ratio
  else if (!is.null(risk_ratio)) inputs$risk_ratio <- risk_ratio
  else if (!is.null(risk_difference))
    inputs$risk_difference <- risk_difference
  else inputs$percent_exposed_outcome <- percent_exposed
  calc_write("ss_cohort", id, inputs, expected, c("stats"))
}

ss_mean_difference_write <- function(id, conf, power, ratio, diff,
                                     s1, s2, mean_1 = NULL,
                                     mean_2 = NULL) {
  build <- function(c) {
    ss <- ss_means(c, power, ratio, diff, s1, s2)
    list(n1 = leaf(ss[1]), n2 = leaf(ss[2]),
         total = leaf(ss[3]))
  }
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  inputs <- list(conf_level = conf, power = power, ratio = ratio,
                 difference = diff, sd_1 = s1, sd_2 = s2)
  calc_write("ss_mean_difference", id, inputs, expected, c("stats"))
}

power_unmatched_cc_write <- function(id, conf, n_cases, n_controls,
                                     pct_cases, pct_controls) {
  build <- function(c) list(
    normal = leaf(power_cc(n_cases, n_controls, pct_cases,
                           pct_controls, c)),
    normal_cc = leaf(power_cc_cc(n_cases, n_controls, pct_cases,
                                 pct_controls, c)),
    odds_ratio = leaf(implied_or(pct_cases, pct_controls)))
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("power_unmatched_cc", id,
             list(conf_level = conf, n_cases = n_cases,
                  n_controls = n_controls,
                  percent_cases_exposed = pct_cases,
                  percent_controls_exposed = pct_controls),
             expected, c("stats"))
}

power_cohort_write <- function(id, conf, n_exp, n_unexp, pct_exp,
                               pct_unexp, design = "cohort") {
  rr_rd <- implied_rr_rd(pct_exp, pct_unexp)
  build <- function(c) list(
    normal = leaf(power_cohort(n_exp, n_unexp, pct_exp, pct_unexp,
                               c)),
    normal_cc = leaf(power_cohort_cc(n_exp, n_unexp, pct_exp,
                                     pct_unexp, c)),
    risk_ratio = leaf(rr_rd[1]),
    risk_difference = leaf(rr_rd[2]))
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("power_cohort", id,
             list(design = design, conf_level = conf,
                  n_exposed = n_exp, n_unexposed = n_unexp,
                  percent_exposed_outcome = pct_exp,
                  percent_unexposed_outcome = pct_unexp),
             expected, c("stats"))
}

power_mean_difference_write <- function(id, conf, n1, n2, diff, s1,
                                        s2) {
  build <- function(c) {
    list(power = leaf(power_means(n1, n2, diff, s1, s2, c)))
  }
  expected <- build(conf)
  check_conf_leak(id, conf, expected, build)
  calc_write("power_mean_difference", id,
             list(conf_level = conf, n_1 = n1, n_2 = n2,
                  difference = diff, sd_1 = s1, sd_2 = s2),
             expected, c("stats"))
}

# --- fixture cases ----------------------------------------------------------

two_by_two_write("textbook", list(matrix(c(10, 11, 4, 20), 2, 2,
                                         byrow = TRUE)))
two_by_two_write("zero_cell", list(matrix(c(0, 15, 8, 30), 2, 2,
                                          byrow = TRUE)))
two_by_two_write("stratified", list(
  matrix(c(30, 70, 10, 90), 2, 2, byrow = TRUE),
  matrix(c(5, 15, 20, 60), 2, 2, byrow = TRUE),
  matrix(c(8, 12, 3, 25), 2, 2, byrow = TRUE)))
two_by_two_write("large", list(matrix(c(4000, 6000, 2500, 7500), 2, 2,
                                      byrow = TRUE)), conf = 99)
two_by_two_write("case_control",
                 list(matrix(c(40, 20, 15, 45), 2, 2, byrow = TRUE)),
                 study_type = "case_control")
# Fix round 3 — CMLE support-boundary and large-count coverage (the
# MartinStats port's robust convolution and explicit boundary paths).
# zero_event: every stratum's a-cell sits at the support minimum, so
# sum_a = min_sum -> est 0, both lower limits 0, finite uppers.
two_by_two_write("zero_event", list(
  matrix(c(0, 10, 5, 20), 2, 2, byrow = TRUE),
  matrix(c(0, 15, 8, 30), 2, 2, byrow = TRUE)))
# support_max: every a-cell sits at the support maximum, so
# sum_a = max_sum -> est and both upper limits undefined (the
# corpus's null leaf), finite lowers.
two_by_two_write("support_max", list(
  matrix(c(10, 0, 5, 20), 2, 2, byrow = TRUE),
  matrix(c(15, 0, 8, 30), 2, 2, byrow = TRUE)))
# one_inside_boundary: sum_a - min_sum = 1 — the fisher numerator is
# degree 0 and used to error; now computes normally. Stock
# fisher.test's own root at this extreme tail is loose (~1.3e-3
# relative on small tables, verified against the port's 1e-14 root),
# so the margins are sized to keep the 1e-3 sanity check meaningful.
two_by_two_write("one_inside_boundary", list(
  matrix(c(1, 50, 10, 200), 2, 2, byrow = TRUE),
  matrix(c(0, 10, 5, 25), 2, 2, byrow = TRUE)))
# large_counts: cells in the millions — the unnormalised convolved
# coefficients overflow double range; the rescaled convolution does
# not. Two identical strata: the convolved CMLE then solves
# 2*E_r[K] = 2*a — the same equation as fisher.test's single-table
# CMLE — so the stratified estimate still has a stock check at counts
# where mantelhaen.test(exact) silently overflows (it is gated off
# inside check_cmle_exact above 5e4 total counts; measured there).
{
  lc <- matrix(c(1e6, 500, 2e6, 500), 2, 2, byrow = TRUE)
  two_by_two_write("large_counts", list(lc, lc))
  lc_ref <- unname(fisher.test(lc)$estimate)
  lc_port <- stratified_cmle_one(list(lc, lc), 0.95, 1e-14)$est
  stop_if(abs(lc_port - lc_ref) > 1e-3 * abs(lc_ref), sprintf(paste0(
    "large_counts stratified CMLE sanity check failed: port=%s ",
    "fisher.test=%s"),
    format(lc_port, digits = 17), format(lc_ref, digits = 17)))
}

two_by_n_write("textbook", list(matrix(c(10, 11, 4, 20), 2, 2,
                                       byrow = TRUE)))
two_by_n_write("zero_cell", list(matrix(c(0, 15, 8, 30), 2, 2,
                                        byrow = TRUE)))
two_by_n_write("stratified", list(
  matrix(c(30, 70, 10, 90), 2, 2, byrow = TRUE),
  matrix(c(5, 15, 20, 60), 2, 2, byrow = TRUE)))

r_by_c_write("textbook",
             matrix(c(10, 20, 15, 5, 30, 25), 2, 3, byrow = TRUE))
r_by_c_write("large",
             matrix(c(200, 150, 400, 300, 50, 350, 600, 100), 2, 4,
                    byrow = TRUE))
r_by_c_write("sparse",
             matrix(c(1, 0, 2, 3, 0, 1, 4, 2, 1, 0), 2, 5,
                    byrow = TRUE))

dose_response_write("textbook", list(
  matrix(c(0, 50, 100, 1, 40, 90, 2, 30, 80, 3, 20, 70), 4, 3,
         byrow = TRUE)))
dose_response_write("stratified", list(
  matrix(c(0, 20, 40, 1, 25, 35, 2, 30, 30), 3, 3, byrow = TRUE),
  matrix(c(0, 10, 50, 1, 15, 45, 2, 20, 40), 3, 3, byrow = TRUE)))
# Boundary + sign coverage: a decreasing trend (V < 0) with a zero
# case at the highest level — exercises the (|V| - 0.5) correction.
dose_response_write("decreasing", list(
  matrix(c(0, 40, 60, 1, 25, 65, 2, 12, 68, 3, 0, 70), 4, 3,
         byrow = TRUE)))
dose_response_write("large", list(
  matrix(c(0, 1000, 2000, 1, 1500, 1800, 2, 2000, 1500, 3, 2500,
           1200), 4, 3, byrow = TRUE)))

matched_write("textbook",
              matrix(c(30, 8, 3, 60), 2, 2, byrow = TRUE))
matched_write("few_discordant",
              matrix(c(15, 4, 2, 20), 2, 2, byrow = TRUE))
matched_write("matched_1to2",
              matrix(c(20, 10, 4, 10, 8, 12), 2, 3, byrow = TRUE),
              controls_per_case = 2)
matched_write("matched_1to4",
              matrix(c(12, 8, 5, 3, 2, 15, 10, 7, 4, 2), 2, 5,
                     byrow = TRUE),
              controls_per_case = 4)

screening_write("textbook",
                matrix(c(80, 40, 20, 360), 2, 2, byrow = TRUE))
screening_write("multi_level",
                matrix(c(60, 15, 15, 10, 10, 35, 15, 340), 4, 2,
                       byrow = TRUE))
screening_write("boundary",
                matrix(c(50, 0, 50, 200), 2, 2, byrow = TRUE))

smr_write("textbook", 50, 36.0)
smr_write("boundary_low", 2, 4.5, conf = 90)
smr_write("conf_97", 30, 20.0, conf = 97)
smr_write("large", 500, 400.0, conf = 99)

binomial_write("textbook", 10, 50, 30)
binomial_write("zero", 0, 50, 25)
binomial_write("large", 700, 1000, 60)

poisson_write("textbook", 20, 15.5)
poisson_write("zero", 0, 5.0)
poisson_write("large", 800, 500.0)

rate_one_write("textbook", 25, 5000)
rate_one_write("boundary", 2, 1200, per_unit = 10000)
rate_one_write("large", 4000, 2000000, per_unit = 100000, conf = 99)

rate_compare_write("textbook",
                   list(matrix(c(40, 20, 5000, 4000), 2, 2,
                               byrow = TRUE)))
rate_compare_write("stratified", list(
  matrix(c(30, 10, 2000, 1500), 2, 2, byrow = TRUE),
  matrix(c(50, 40, 3000, 5000), 2, 2, byrow = TRUE)),
  per_unit = 10000, conf = 90)
# Boundary: equal rates (RR = 1, null effect — z ~ 0, CIs straddle
# the null).
rate_compare_write("boundary",
                   list(matrix(c(10, 20, 1000, 2000), 2, 2,
                               byrow = TRUE)))
rate_compare_write("large",
                   list(matrix(c(400, 200, 80000, 90000), 2, 2,
                               byrow = TRUE)),
                   per_unit = 10000, conf = 99)
# Fix round 3 — CMLE support-boundary and large-count coverage.
# zero_event: a = 0, the support minimum -> est 0, both lower limits
# 0, finite uppers. support_max: b = 0, the support maximum -> est
# and both upper limits undefined (the corpus's null leaf), finite
# lowers. one_inside_boundary: a = 1 — the fisher numerator is
# degree 0 and used to error; now computes normally.
rate_compare_write("zero_event",
                   list(matrix(c(0, 20, 1000, 2000), 2, 2,
                               byrow = TRUE)))
rate_compare_write("support_max",
                   list(matrix(c(20, 0, 1000, 2000), 2, 2,
                               byrow = TRUE)))
rate_compare_write("one_inside_boundary",
                   list(matrix(c(1, 20, 1000, 2000), 2, 2,
                               byrow = TRUE)))
# Counts in the millions: the binomial coefficient polynomial
# overflows double range before rescaling.
rate_compare_write("large_counts",
                   list(matrix(c(1e6, 2e5, 4e6, 1e6), 2, 2,
                               byrow = TRUE)))

mean_ci_write("textbook", 10.5, "sd", 2.5, 25)
mean_ci_write("se_input", 10.5, "se", 0.5, 25)
mean_ci_write("variance_input", 10.5, "variance", 6.25, 25,
              population_size = 250, conf = 90)

median_ci_write("median", 25, 50)
median_ci_write("p25", 100, 25, conf = 90)
# Boundary: an extreme percentile — the normal-approximation ranks
# leave [1, n], which the fixture records as the reference output.
median_ci_write("boundary", 30, 5)
median_ci_write("large", 5000, 75, conf = 99)

t_test_summary_write("textbook", 25, 10.5, 2.5, 30, 8.5, 3.0)
t_test_summary_write("equal_n", 20, 100.0, 10.0, 20, 95.0, 12.0,
                     conf = 99)
# Boundary: minimum group size n1 = 2.
t_test_summary_write("boundary", 2, 10.5, 3.0, 30, 8.5, 4.0)
t_test_summary_write("large", 5000, 100.2, 15.0, 4500, 99.8, 16.0,
                     conf = 99)

anova_summary_write("textbook",
                    matrix(c(10, 5.0, 1.5, 12, 7.5, 2.0, 8, 6.0,
                             1.2), 3, 3, byrow = TRUE))
anova_summary_write("two_groups",
                    matrix(c(15, 50.0, 5.0, 15, 55.0, 6.0), 2, 3,
                           byrow = TRUE))
# Boundary: a group at the minimum n = 2.
anova_summary_write("small_group",
                    matrix(c(2, 5.0, 1.0, 25, 6.5, 1.8, 30, 7.0,
                             2.0), 3, 3, byrow = TRUE))
anova_summary_write("large",
                    matrix(c(2000, 50.1, 4.9, 2000, 51.2, 5.1, 2000,
                             49.8, 5.0, 2000, 50.5, 5.2, 2000, 50.0,
                             4.8, 2000, 50.9, 5.0), 6, 3,
                           byrow = TRUE))

ss_proportion_write("textbook", 1000000, 50, 5, 1)
ss_proportion_write("clustered", 5000, 30, 5, 1.8, clusters = 25)
# Boundary: finite population at the limit — the uncorrected n (~384
# at 95%) exceeds N = 350, so the FPC pulls it below the population.
ss_proportion_write("small_population", 350, 50, 5, 1)
ss_proportion_write("large", 10000000, 10, 1, 1)

ss_cc_write("odds_ratio_input", 95, 80, 1, 40, odds_ratio = 2.0)
ss_cc_write("percent_input", 95, 80, 2, 25, p1_pct = 40)
ss_cc_write("power_90", 90, 90, 1, 30, odds_ratio = 2.5)

ss_cohort_write("percent_input", 95, 80, 1, 5, percent_exposed = 10)
ss_cohort_write("odds_ratio_input", 95, 80, 1, 5, odds_ratio = 2.0)
ss_cohort_write("risk_ratio_input", 95, 80, 1, 5, risk_ratio = 2.0)
ss_cohort_write("risk_difference_input", 95, 80, 2, 20,
                risk_difference = 10)

ss_mean_difference_write("textbook", 95, 80, 1, 5, 10, 10)
# Boundary: a very small standardised effect.
ss_mean_difference_write("small_effect", 95, 80, 1, 0.5, 10, 12)
# Large: 99% power at 99% confidence with ratio != 1.
ss_mean_difference_write("high_power", 99, 99, 2, 5, 10, 12)

power_unmatched_cc_write("textbook", 95, 100, 150, 50, 30)
power_unmatched_cc_write("rare_exposure", 95, 200, 400, 30, 10)
# Boundary: a very small exposure difference (31% vs 30%).
power_unmatched_cc_write("small_effect", 95, 100, 150, 31, 30)
power_unmatched_cc_write("large", 99, 5000, 10000, 45, 30)

power_cohort_write("cohort", 95, 200, 400, 10, 5, "cohort")
power_cohort_write("rct", 95, 150, 150, 30, 20, "rct")
power_cohort_write("cross_sectional", 95, 300, 300, 40, 30,
                   "cross_sectional")

power_mean_difference_write("textbook", 95, 50, 50, 5, 10, 10)
# Boundary: a very small standardised effect.
power_mean_difference_write("small_effect", 95, 30, 30, 0.5, 10, 10)
power_mean_difference_write("large", 99, 5000, 6000, 2, 10, 12)
