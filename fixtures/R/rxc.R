# fixtures/rxc/*.json — rxc fixtures (plan Step 10): chisq.test on a
# 3x2 and a 4x3 table.

rxc_write <- function(id, tab) {
  chi <- suppressWarnings(chisq.test(tab))
  write_fixture("rxc", id, "rxc",
                inputs = list(table = tab),
                expected = list(
                  chi_square = list(
                    statistic = leaf(unname(chi$statistic)),
                    df = leaf(unname(chi$parameter)),
                    p_value = leaf(chi$p.value))),
                packages = c("stats"))
}

rxc_write("table_3x2",
          matrix(c(12, 8,
                   5, 20,
                   9, 3), nrow = 3, byrow = TRUE))
rxc_write("table_4x3",
          matrix(c(10, 5, 2,
                   8, 12, 4,
                   3, 7, 15,
                   6, 6, 6), nrow = 4, byrow = TRUE))
