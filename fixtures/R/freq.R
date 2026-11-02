# fixtures/freq/*.json — freq fixtures (plan Step 10).
# inputs.{dataset, variable, include_missing, labels};
# expected.{total, missing, categories[]}. Categories follow the
# spec §3.3 order and carry no label: they match inputs.labels
# positionally. "(missing)" is appended last and only counted when
# include_missing is true. Percentages and CIs are on the 0-100 scale.

# One category: count, percent, cumulative percent, Clopper-Pearson
# and Wilson 95% CIs (binom.test / prop.test, x100).
freq_category <- function(count, denom, cum) {
  cp <- binom.test(count, denom)$conf.int * 100
  wi <- suppressWarnings(
    prop.test(count, denom, correct = FALSE)$conf.int * 100)
  list(
    count = leaf(count),
    percent = leaf(count / denom * 100),
    cumulative_percent = leaf(cum / denom * 100),
    ci_95_clopper_pearson = list(
      lower = leaf(cp[1]), upper = leaf(cp[2])),
    ci_95_wilson = list(
      lower = leaf(wi[1]), upper = leaf(wi[2])))
}

# Full expected object for one variable. `labels` are the categories
# in spec §3.3 order (already sorted by the caller); "(missing)" is
# appended when include_missing is true and there are missing values.
freq_expected <- function(x, labels, include_missing) {
  n_total <- length(x)
  n_missing <- sum(is.na(x))
  denom <- if (include_missing) n_total else n_total - n_missing
  counts <- vapply(labels, function(lv) sum(x == lv, na.rm = TRUE),
                   numeric(1), USE.NAMES = FALSE)
  if (include_missing && n_missing > 0) {
    labels <- c(labels, "(missing)")
    counts <- c(counts, n_missing)
  }
  cum <- 0
  categories <- lapply(counts, function(cnt) {
    cum <<- cum + cnt
    freq_category(cnt, denom, cum)
  })
  list(
    total = leaf(n_total),
    missing = leaf(n_missing),
    categories = categories)
}

# Write one freq fixture for a column of a DATASETS entry.
freq_write <- function(id, dataset, variable, include_missing) {
  col <- get(dataset, envir = DATASETS)[[variable]]
  stop_if(is.null(col), sprintf("%s: no column %s in dataset %s",
                                id, variable, dataset))
  x <- col$values
  labels <- if (col$type == "boolean") {
    # spec §3.3: Boolean -> Yes, No
    c(TRUE, FALSE)[c(any(x, na.rm = TRUE), any(!x, na.rm = TRUE))]
  } else {
    sort(unique(x[!is.na(x)]), method = "radix")
  }
  has_missing_cat <- include_missing && sum(is.na(x)) > 0
  # inputs.labels mirrors categories[] positionally, so "(missing)" is
  # listed when the fixture carries that category.
  inputs <- list(dataset = dataset, variable = variable,
                 include_missing = include_missing,
                 labels = if (has_missing_cat) c(labels, "(missing)") else labels)
  write_fixture("freq", id, "freq", inputs,
                freq_expected(x, labels, include_missing),
                packages = c("stats"))
}

freq_write("oswego_sex", "oswego", "SEX", FALSE)
freq_write("oswego_age", "oswego", "AGE", FALSE)
freq_write("missing_group", "missing", "GROUP", FALSE)
freq_write("missing_group_incl_missing", "missing", "GROUP", TRUE)
