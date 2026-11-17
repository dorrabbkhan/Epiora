//! `attach` — one adapter per run surface: the spec §6.1 rules
//! applied to the values the procedure or calculator itself
//! computed, then sorted by code and by the (already sorted)
//! statistics list — the plan's deterministic order. Triggers come
//! from the shared intermediates the run used (`two_by_two_parts`,
//! `rxc_parts`, `oe_cells`, `strata2x2`, `groups_of`,
//! `bartlett_from_summary`, `woolf`), never a second implementation.

use super::{rules, Note};
use crate::calc::continuous::anova_summary;
use crate::calc::counts::{adjusted, shared};
use crate::procedures::MeansResult;
use crate::stats::{
    bartlett_from_summary, rxc_parts, two_by_two_parts, Estimate, StratifiedStats, Table2x2,
};
use serde_json::Value;

fn sorted(mut notes: Vec<Note>) -> Vec<Note> {
    notes.sort_by(|a, b| {
        a.code
            .cmp(&b.code)
            .then_with(|| a.statistics.cmp(&b.statistics))
    });
    notes
}

fn estimate(e: &Estimate) -> Option<f64> {
    match e {
        Estimate::Value(v) => Some(*v),
        Estimate::Undefined { .. } => None,
    }
}

/// `tables` 2×2 notes (spec §6.1): the pooled table's three
/// chi-square p-values and its Wald OR/RR intervals — the only
/// chi-square and Wald statistics a 2×2 procedure emits — plus
/// `heterogeneous_strata` when a stratified run produced a
/// Breslow–Day–Tarone p.
pub(crate) fn tables_two_by_two(
    pooled: &Table2x2,
    stratified: Option<&StratifiedStats>,
) -> Vec<Note> {
    let p = two_by_two_parts(pooled);
    let mut notes = Vec::new();
    notes.extend(rules::expected_below_5(
        &p.e,
        vec![
            "two_by_two.chi_square.mantel_haenszel.p_value".to_string(),
            "two_by_two.chi_square.uncorrected.p_value".to_string(),
            "two_by_two.chi_square.yates.p_value".to_string(),
        ],
        "two_by_two.fisher_exact.p_two_sided",
        &[],
    ));
    notes.extend(rules::zero_cell(
        &[pooled.a, pooled.b, pooled.c, pooled.d],
        vec![
            "two_by_two.odds_ratio.ci_95_wald.lower".to_string(),
            "two_by_two.odds_ratio.ci_95_wald.upper".to_string(),
            "two_by_two.risk_ratio.ci_95_wald.lower".to_string(),
            "two_by_two.risk_ratio.ci_95_wald.upper".to_string(),
        ],
        "two_by_two.odds_ratio.cmle",
        &[],
    ));
    if let Some(p) = stratified.and_then(|s| estimate(&s.breslow_day_tarone.p_value)) {
        notes.extend(rules::heterogeneous_strata(
            ("breslow_day_tarone_p", p),
            vec![
                "stratified_two_by_two.breslow_day_tarone.p_value".to_string(),
                "stratified_two_by_two.mantel_haenszel.odds_ratio.estimate".to_string(),
                "stratified_two_by_two.mantel_haenszel.risk_ratio.estimate".to_string(),
            ],
            "",
        ));
    }
    sorted(notes)
}

/// `tables` R×C notes: `expected_count_below_5` on the Pearson
/// table's own expected cells (`rxc_parts` is the helper `rxc`
/// computed them with).
pub(crate) fn tables_rxc(counts: &[Vec<u64>]) -> Vec<Note> {
    let p = rxc_parts(counts);
    sorted(
        rules::expected_below_5(
            &p.expected,
            vec![
                "rxc.chi_square.p_value".to_string(),
                "rxc.chi_square.statistic".to_string(),
            ],
            "",
            &[],
        )
        .into_iter()
        .collect(),
    )
}

/// `means` notes: `unequal_variances` on Bartlett's p. Welch's t is
/// the suggestion for two groups, Kruskal–Wallis for more (spec
/// §6.1); only statistics the run actually emitted are listed.
pub(crate) fn means(result: &MeansResult) -> Vec<Note> {
    let Some(p) = result.bartlett.as_ref().and_then(|b| estimate(&b.p_value)) else {
        return Vec::new();
    };
    let mut statistics = vec![
        "means.bartlett.p_value".to_string(),
        "means.bartlett.statistic".to_string(),
    ];
    if result.t_test.is_some() {
        statistics.push("means.t_test.pooled.p_value".to_string());
        statistics.push("means.t_test.pooled.t".to_string());
    }
    if result.anova.is_some() {
        statistics.push("means.anova.f".to_string());
        statistics.push("means.anova.p_value".to_string());
    }
    let suggests = if result.t_test.is_some() {
        "means.t_test.welch.t"
    } else if result.kruskal_wallis.is_some() {
        "means.kruskal_wallis.h"
    } else {
        ""
    };
    sorted(
        rules::unequal_variances(p, statistics, suggests)
            .into_iter()
            .collect(),
    )
}

/// One calculator run's notes — the §6.1 rules over the same
/// canonical `inputs` the run computed on, through the same shared
/// helpers. Families with no matching rule emit none.
pub(crate) fn calc(id: &str, inputs: &Value) -> Vec<Note> {
    sorted(match id {
        "proportion" => proportion(inputs),
        "two_by_two" | "two_by_n" => two_by_two_calc(id, inputs),
        "r_by_c" => r_by_c(inputs),
        "matched_case_control" => matched(inputs),
        "anova_summary" => anova(inputs),
        _ => Vec::new(),
    })
}

/// `proportion`: `normal_approx_small` on the Wald interval's own
/// `n·p`/`n·(1−p)` — the Wilson interval is the suggested
/// alternative (spec §6.1).
fn proportion(inputs: &Value) -> Vec<Note> {
    rules::normal_approx_small(
        shared::cnt(inputs, "numerator"),
        shared::cnt(inputs, "denominator"),
        vec![
            "calc.proportion.ci.wald.lower".to_string(),
            "calc.proportion.ci.wald.upper".to_string(),
        ],
        "calc.proportion.ci.wilson.lower",
    )
    .into_iter()
    .collect()
}

/// `two_by_two`/`two_by_n`: `expected_count_below_5` and `zero_cell`
/// per emitted table block — every `strata[]` block and the `crude`
/// block carries chi-squares and Wald intervals, so each gets its
/// own note (`stratum` marks the index; the pooled `crude` has
/// none). With ≥ 2 strata the OR homogeneity test (`adjusted.
/// breslow_day.or_p` — the run's Woolf chi-square through
/// `adjusted::woolf`/`woolf_chisq`) adds `heterogeneous_strata`.
fn two_by_two_calc(id: &str, inputs: &Value) -> Vec<Note> {
    let strata = shared::strata2x2(inputs);
    let crude = strata.iter().fold(
        Table2x2 {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
        },
        |m, t| Table2x2 {
            a: m.a + t.a,
            b: m.b + t.b,
            c: m.c + t.c,
            d: m.d + t.d,
        },
    );
    // Per-id block spellings: (chi-square p paths, the exact p to
    // suggest, the exact/mid-p odds-ratio estimate to suggest).
    let (chi_ps, fisher, exact_or): ([&str; 3], &str, &str) = if id == "two_by_two" {
        (
            [
                "chisq.mantel_haenszel.p_2t",
                "chisq.uncorrected.p_2t",
                "chisq.yates.p_2t",
            ],
            "fisher_exact.p_2t",
            "cmle_or.midp.est",
        )
    } else {
        (
            [
                "chisq.mantel.p_2t",
                "chisq.uncorrected.p_2t",
                "chisq.yates.p_2t",
            ],
            "exact.fisher_2t",
            "odds_ratio_mle.midp.est",
        )
    };
    let mut notes = Vec::new();
    for (idx, t) in strata
        .iter()
        .enumerate()
        .map(|(i, t)| (Some(i), *t))
        .chain(std::iter::once((None, crude)))
    {
        let prefix = format!("calc.{id}.{}", idx.map_or("crude", |_| "strata[]"));
        let extra: Vec<(&str, f64)> = idx.map_or_else(Vec::new, |i| vec![("stratum", i as f64)]);
        notes.extend(rules::expected_below_5(
            &shared::oe_cells(&t).map(|c| c[1]),
            chi_ps.iter().map(|s| format!("{prefix}.{s}")).collect(),
            &format!("{prefix}.{fisher}"),
            &extra,
        ));
        notes.extend(rules::zero_cell(
            &[t.a, t.b, t.c, t.d],
            vec![
                format!("{prefix}.odds_ratio.lower"),
                format!("{prefix}.odds_ratio.upper"),
                format!("{prefix}.risk_ratio.lower"),
                format!("{prefix}.risk_ratio.upper"),
            ],
            &format!("{prefix}.{exact_or}"),
            &extra,
        ));
    }
    if id == "two_by_two" && strata.len() > 1 {
        // `adjusted.breslow_day` is the run's own Woolf homogeneity
        // chi-square for the OR (k − 1 df) — the same helpers
        // `adjusted()` computed it with.
        let p = adjusted::woolf(&strata, "or");
        let chi = adjusted::woolf_chisq(p.adj, &p.ests, &p.ws);
        notes.extend(rules::heterogeneous_strata(
            (
                "breslow_day_or_p",
                shared::chi_p(chi, strata.len() as f64 - 1.0),
            ),
            vec![
                "calc.two_by_two.adjusted.breslow_day.or_p".to_string(),
                "calc.two_by_two.adjusted.mh_or.est".to_string(),
            ],
            "calc.two_by_two.strata[].odds_ratio.est",
        ));
    }
    notes
}

/// `r_by_c`: `expected_count_below_5` on `rxc_parts`' expected
/// cells — the calculator's own diagnostics (spec §6.1).
fn r_by_c(inputs: &Value) -> Vec<Note> {
    let p = rxc_parts(&shared::count_table(inputs, "table"));
    rules::expected_below_5(
        &p.expected,
        vec![
            "calc.r_by_c.chisq.p".to_string(),
            "calc.r_by_c.chisq.value".to_string(),
        ],
        "",
        &[],
    )
    .into_iter()
    .collect()
}

/// `matched_case_control` (1:1): `zero_cell` on the discordant cells
/// — `b` or `c` at zero makes the Taylor log OR interval undefined
/// the same way a 2×2's Wald interval is. The 1:n block is
/// Mantel–Haenszel, so the rule does not apply there.
fn matched(inputs: &Value) -> Vec<Note> {
    if shared::cnt(inputs, "controls_per_case") != 1 {
        return Vec::new();
    }
    let t = shared::count_table(inputs, "matched_sets");
    let discordants = [
        t.first().and_then(|r| r.get(1)).copied().unwrap_or(0),
        t.get(1).and_then(|r| r.first()).copied().unwrap_or(0),
    ];
    rules::zero_cell(
        &discordants,
        vec![
            "calc.matched_case_control.or.lower".to_string(),
            "calc.matched_case_control.or.upper".to_string(),
        ],
        "calc.matched_case_control.cmle_or.midp.est",
        &[],
    )
    .into_iter()
    .collect()
}

/// `anova_summary`: `unequal_variances` on the Bartlett block's p
/// (`bartlett_from_summary` — the same helper the run used). For two
/// groups the suggestion is the `t_test_summary` calculator's Welch
/// path; more groups have no calculator alternative.
fn anova(inputs: &Value) -> Vec<Note> {
    let groups = anova_summary::groups_of(&shared::table(inputs, "table"));
    let Some(b) = bartlett_from_summary(&groups) else {
        return Vec::new();
    };
    let Some(p) = estimate(&b.p_value) else {
        return Vec::new();
    };
    rules::unequal_variances(
        p,
        vec![
            "calc.anova_summary.anova.f".to_string(),
            "calc.anova_summary.anova.p".to_string(),
            "calc.anova_summary.bartlett.chisq".to_string(),
            "calc.anova_summary.bartlett.p".to_string(),
            "calc.anova_summary.group_ci_pooled[].lower".to_string(),
            "calc.anova_summary.group_ci_pooled[].upper".to_string(),
        ],
        if groups.len() == 2 {
            "t_test_summary"
        } else {
            ""
        },
    )
    .into_iter()
    .collect()
}
