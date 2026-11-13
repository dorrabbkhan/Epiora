# fixtures/data/means_*.dataset.json and fixtures/means/*.json —
# MEANS datasets and fixtures (v1.0 Task 2, spec §4.2).
# inputs.{dataset, variable, group_by?, include_missing};
# expected.{total, missing, groups[], t_test?, anova?, bartlett?,
# kruskal_wallis?}. Groups are ordered by sorted group label;
# "(missing)" rows form the last group when include_missing is true.
# Percentiles are Epi Info 7's Tukey hinges — R's fivenum() (no single
# quantile(type) reproduces hinges for every n; type 2 differs for
# n %% 4 == 3). Mode is the smallest value attaining the maximum count.
# A group with one observation has undefined variance/std_dev (null
# leaves, spec §4.2 "Undefined { too_few }").

# --- datasets ---------------------------------------------------------

write_dataset("means_basic", list(
  value = list(type = "real",
    values = c(2.5, 4.0, 3.5, 8.0, 7.5, 9.0, 15.0, 14.5, 16.5, NA)),
  grp = list(type = "text",
    values = c("A", "A", "A", "B", "B", "B", "C", "C", "C", NA))))

write_dataset("means_ties", list(
  value = list(type = "integer",
    values = c(3L, 3L, 4L, 4L, 5L, 7L, 7L, 7L, 8L, 8L, 8L, 9L, NA)),
  grp = list(type = "text",
    values = c("X", "X", "X", "X", "X", "X", "Y", "Y", "Y", "Y", "Y",
               "Y", "Y"))))

write_dataset("means_one_obs_group", list(
  value = list(type = "real",
    values = c(1.5, 2.5, 3.5, 4.5, 99.5, 5.5, 6.5)),
  grp = list(type = "text",
    values = c("A", "A", "A", "A", "B", "C", "C"))))

write_dataset("means_constant", list(
  value = list(type = "real", values = rep(7.25, 8)),
  grp = list(type = "text",
    values = c("A", "A", "A", "A", "B", "B", "B", "B"))))

write_dataset("means_all_missing", list(
  value = list(type = "real", values = c(NA, NA, NA, NA)),
  grp = list(type = "text", values = c("A", "A", "B", "B"))))

# Dates include days before the 1970-01-01 epoch.
write_dataset("means_dates", list(
  onset = list(type = "date",
    values = c("1969-12-30", "1970-01-02", "1970-01-05", "1970-01-10",
               "1960-06-15", "1960-06-20", "1960-06-30", NA)),
  grp = list(type = "text",
    values = c("E", "E", "E", "E", "L", "L", "L", "L"))))

# --- per-group statistics ---------------------------------------------

# One group's descriptives as leaves. `x` is the non-missing values in
# data order; dates are passed as numeric days since 1970-01-01.
means_group <- function(x) {
  n <- length(x)
  fv <- fivenum(x)
  tab <- table(x)
  mode_val <- min(as.numeric(names(tab)[tab == max(tab)]))
  list(
    observations = leaf(n),
    total = leaf(sum(x)),
    mean = leaf(mean(x)),
    variance = leaf(if (n >= 2) var(x) else NA),
    std_dev = leaf(if (n >= 2) sd(x) else NA),
    minimum = leaf(fv[1]),
    p25 = leaf(fv[2]),
    median = leaf(fv[3]),
    p75 = leaf(fv[4]),
    maximum = leaf(fv[5]),
    mode = leaf(mode_val))
}

# 95% t-test leaves for exactly two groups (spec §4.2 fixed 95%).
means_ttest <- function(x1, x2) {
  pooled <- t.test(x1, x2, var.equal = TRUE, conf.level = 0.95)
  welch <- t.test(x1, x2, var.equal = FALSE, conf.level = 0.95)
  diff <- mean(x1) - mean(x2)
  tset <- function(tt) list(
    t = leaf(unname(tt$statistic)),
    df = leaf(unname(tt$parameter)),
    p = leaf(tt$p.value),
    diff = leaf(diff),
    lower = leaf(tt$conf.int[1]),
    upper = leaf(tt$conf.int[2]))
  list(pooled = tset(pooled), welch = tset(welch))
}

# Full expected object: `x` numeric values and `grp` (NULL or a vector
# parallel to x) in data order, including NAs.
means_expected <- function(x, grp, include_missing) {
  na_var <- is.na(x)
  na_grp <- if (is.null(grp)) rep(FALSE, length(x)) else is.na(grp)
  dropped <- na_var | (na_grp & !include_missing)
  total <- length(x)
  missing <- sum(dropped)
  keep <- !dropped
  xs <- x[keep]
  gs <- if (is.null(grp)) NULL else grp[keep]

  group_values <- if (is.null(gs)) {
    list(xs)
  } else {
    labs <- sort(unique(gs[!is.na(gs)]))
    if (include_missing && any(is.na(gs))) labs <- c(labs, "(missing)")
    lapply(labs, function(lb) {
      if (lb == "(missing)") xs[is.na(gs)] else xs[!is.na(gs) & gs == lb]
    })
  }
  expected <- list(
    total = leaf(total),
    missing = leaf(missing),
    groups = lapply(group_values, means_group))

  nonempty <- group_values[lengths(group_values) > 0]
  if (!is.null(gs) && length(nonempty) >= 2) {
    if (length(nonempty) == 2) {
      tt <- tryCatch(means_ttest(nonempty[[1]], nonempty[[2]]),
                     error = function(e) NULL)
      if (!is.null(tt)) expected$t_test <- tt
    }
    all_x <- unlist(nonempty)
    all_g <- factor(rep(seq_along(nonempty), lengths(nonempty)))
    n_tot <- length(all_x); k <- length(nonempty)
    ns <- lengths(nonempty); ms <- vapply(nonempty, mean, numeric(1))
    ss <- vapply(nonempty, function(v) {
      if (length(v) >= 2) var(v) else NA_real_ }, numeric(1))
    grand <- sum(ns * ms) / n_tot
    ss_b <- sum(ns * (ms - grand)^2)
    ss_w <- sum((ns - 1) * ss)
    df_b <- k - 1; df_w <- n_tot - k
    f <- (ss_b / df_b) / (ss_w / df_w)
    expected$anova <- list(
      ss_between = leaf(ss_b), df_between = leaf(df_b),
      ms_between = leaf(ss_b / df_b),
      ss_within = leaf(ss_w), df_within = leaf(df_w),
      ms_within = leaf(ss_w / df_w),
      f = leaf(f), p = leaf(pf(f, df_b, df_w, lower.tail = FALSE)),
      ss_total = leaf(ss_b + ss_w), df_total = leaf(n_tot - 1))
    bt <- tryCatch(bartlett.test(all_x, all_g), error = function(e) NULL)
    if (!is.null(bt)) {
      expected$bartlett <- list(
        chisq = leaf(unname(bt$statistic)),
        df = leaf(unname(bt$parameter)),
        p = leaf(bt$p.value))
    }
    kw <- tryCatch(kruskal.test(all_x, all_g), error = function(e) NULL)
    if (!is.null(kw)) {
      expected$kruskal_wallis <- list(
        h = leaf(unname(kw$statistic)),
        df = leaf(unname(kw$parameter)),
        p = leaf(kw$p.value))
    }
  }
  expected
}

means_write <- function(id, dataset, variable, group_by = NULL,
                        include_missing = FALSE) {
  cols <- get(dataset, envir = DATASETS)
  col <- cols[[variable]]
  stop_if(is.null(col), sprintf("%s: no column %s in dataset %s",
                                id, variable, dataset))
  x <- col$values
  if (col$type == "date") x <- as.numeric(as.Date(x))
  gcol <- if (is.null(group_by)) NULL else cols[[group_by]]$values
  inputs <- list(dataset = dataset, variable = variable,
                 include_missing = include_missing)
  if (!is.null(group_by)) inputs$group_by <- group_by
  write_fixture("means", id, "means", inputs,
                means_expected(x, gcol, include_missing), c("stats"))
}

# --- fixtures -----------------------------------------------------------

means_write("means_ungrouped", "means_basic", "value")
means_write("means_grouped", "means_basic", "value", group_by = "grp")
means_write("means_include_missing", "means_basic", "value",
            group_by = "grp", include_missing = TRUE)
means_write("means_ties", "means_ties", "value", group_by = "grp")
means_write("means_one_obs_group", "means_one_obs_group", "value",
            group_by = "grp")
means_write("means_constant", "means_constant", "value", group_by = "grp")
means_write("means_all_missing", "means_all_missing", "value",
            group_by = "grp")
means_write("means_dates", "means_dates", "onset", group_by = "grp")
means_write("means_oswego_age", "oswego", "AGE")
means_write("means_oswego_age_sex", "oswego", "AGE", group_by = "SEX")
