# Shared helpers for the corpus generator (v0.2 Task 2, spec §6.2).
# Sourced by fixtures/generate.R; expects FIXTURES_DIR (the fixtures/
# directory) to be set before use.

# One expected leaf: {value, class, r_default?}. A non-finite `v` means
# the reference says the statistic is undefined; it serialises as JSON
# null. `r_default` is recorded only on root-found leaves and is dropped
# when it is itself non-finite.
leaf <- function(v, class = c("closed_form", "iterative"), r_default = NULL) {
  class <- match.arg(class)
  out <- list(
    value = if (length(v) == 0L || !is.finite(v)) NA else unname(as.numeric(v)),
    class = class
  )
  if (!is.null(r_default) && length(r_default) > 0L && is.finite(r_default)) {
    out$r_default <- unname(as.numeric(r_default))
  }
  out
}

# E5 (spec §6.2): a tight root-found value must agree with R's
# default-tolerance result; otherwise the reimplementation is wrong and
# generation stops. Both sides non-finite counts as agreement only when
# they are the same kind (Inf with the same sign, or both NaN).
check_tight <- function(tight, r_default, what) {
  if (!is.finite(tight) || !is.finite(r_default)) {
    # Non-finite sides agree when they are the same kind: NA/NaN both
    # mean "reference says undefined", Inf only with matching signs.
    kind <- function(v) {
      if (is.na(v)) return("undefined")
      if (v == Inf) return("inf")
      if (v == -Inf) return("-inf")
      "finite"
    }
    stop_if(kind(tight) != kind(r_default), sprintf(
      "tight root check failed for %s: tight=%s r_default=%s",
      what, format(tight, digits = 17), format(r_default, digits = 17)))
    return(invisible(TRUE))
  }
  stop_if(
    abs(tight - r_default) > 1e-2 * abs(r_default),
    sprintf("tight root check failed for %s: tight=%s r_default=%s",
            what, format(tight, digits = 17), format(r_default, digits = 17))
  )
  invisible(TRUE)
}

# Stop with `msg` when `cond` is TRUE.
stop_if <- function(cond, msg) {
  if (isTRUE(cond)) stop(msg, call. = FALSE)
  invisible(!cond)
}

# --- mantelhaen.test port ------------------------------------------------
#
# Verbatim port of the non-exact 2x2xK branch of R 4.5.2's
# stats::mantelhaen.test (src/library/stats/R/mantelhaen.test.R), with
# two deliberate changes:
#   - the input guards are dropped so a single stratum (K = 1) works:
#     stock mantelhaen.test requires every dim(x) >= 2, so the plan's
#     "mantelhaen.test on the single-stratum array" cannot run
#     literally; this port is the same computation with the dimension
#     guard removed, which is exactly what the plan needs for the 2x2
#     fixtures and the one_stratum stratified fixture;
#   - only the inputs this corpus uses are kept (alternative =
#     "two.sided", conf.level = 0.95).
# Wherever stock mantelhaen.test can run (K >= 2 strata), generation
# checks the port reproduces it bit for bit (check_mh_port below).
mh_2x2xk <- function(x, correct = FALSE, conf.level = 0.95) {
  s.x <- apply(x, c(1L, 3L), sum)
  s.y <- apply(x, c(2L, 3L), sum)
  n <- as.double(apply(x, 3L, sum))
  DELTA <- sum(x[1, 1, ] - s.x[1, ] * s.y[1, ]/n)
  YATES <- if (correct && (abs(DELTA) >= 0.5)) 0.5 else 0
  STATISTIC <- ((abs(DELTA) - YATES)^2/sum(apply(rbind(s.x, s.y), 2L,
                                                prod)/(n^2 * (n - 1))))
  PVAL <- pchisq(STATISTIC, 1, lower.tail = FALSE)
  s.diag <- sum(x[1L, 1L, ] * x[2L, 2L, ]/n)
  s.offd <- sum(x[1L, 2L, ] * x[2L, 1L, ]/n)
  ESTIMATE <- s.diag/s.offd
  sd <- sqrt(sum((x[1L, 1L, ] + x[2L, 2L, ]) * x[1L, 1L, ] *
                   x[2L, 2L, ]/n^2)/(2 * s.diag^2) +
             sum(((x[1L, 1L, ] + x[2L, 2L, ]) * x[1L, 2L, ] * x[2L, 1L, ] +
                  (x[1L, 2L, ] + x[2L, 1L, ]) * x[1L, 1L, ] *
                  x[2L, 2L, ])/n^2)/(2 * s.diag * s.offd) +
             sum((x[1L, 2L, ] + x[2L, 1L, ]) * x[1L, 2L, ] *
                   x[2L, 1L, ]/n^2)/(2 * s.offd^2))
  CINT <- ESTIMATE * exp(c(1, -1) * qnorm((1 - conf.level)/2) * sd)
  list(statistic = STATISTIC, p.value = PVAL,
       estimate = ESTIMATE, conf.int = CINT)
}

# Generation-time check: on a case where stock mantelhaen.test runs
# (>= 2 strata, each n >= 2), the port must reproduce it bit for bit.
check_mh_port <- function(x, what) {
  for (corr in c(FALSE, TRUE)) {
    stock <- mantelhaen.test(x, correct = corr)
    port <- mh_2x2xk(x, correct = corr)
    stop_if(
      !isTRUE(all.equal(
        c(unname(stock$statistic), stock$p.value,
          unname(stock$estimate), stock$conf.int),
        c(unname(port$statistic), port$p.value,
          unname(port$estimate), port$conf.int),
        tolerance = 0)),
      sprintf("mantelhaen.test port mismatch for %s (correct=%s)",
              what, corr))
  }
  invisible(TRUE)
}

# Installed versions of `pkgs` as a named list (for source.packages).
pkg_versions <- function(pkgs) {
  v <- vapply(pkgs, function(p) as.character(utils::packageVersion(p)),
              character(1))
  as.list(v)
}

# Write one fixture in format v2: fills in tolerance_classes and source.
# `packages` is a character vector of package names whose functions
# produced the expected values.
write_fixture <- function(dir, id, procedure, inputs, expected, packages) {
  fixture <- list(
    id = id,
    procedure = procedure,
    inputs = inputs,
    expected = expected,
    tolerance_classes = list(closed_form = 1e-9, iterative = 1e-6),
    source = list(
      r_version = R.version.string,
      packages = pkg_versions(packages)
    )
  )
  out_dir <- file.path(FIXTURES_DIR, dir)
  dir.create(out_dir, showWarnings = FALSE, recursive = TRUE)
  out_file <- file.path(out_dir, paste0(id, ".json"))
  # digits = I(17): 17 significant digits, so every double round-trips
  # exactly and the 1e-9 tolerance stays meaningful.
  writeLines(
    jsonlite::toJSON(fixture, auto_unbox = TRUE, pretty = TRUE,
                     digits = I(17), na = "null"),
    out_file
  )
  message("wrote ", out_file)
  invisible(out_file)
}
