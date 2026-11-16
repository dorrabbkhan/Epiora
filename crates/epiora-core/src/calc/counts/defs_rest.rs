//! Registry data for counts calculators `dose_response` through
//! `poisson` (continued from `defs.rs`): declared inputs and
//! emitted statistics, in `docs/v1.0-calculators.md` terms.

use crate::calc::{
    idf, od, CalculatorDef, CellKind, DefaultValue, Family, InputDef, InputKind, OutputDef,
};
use crate::validation::ToleranceClass::{ClosedForm as CF, Iterative as IT};
const DOSE_RESPONSE_INPUTS: &[InputDef] = &[
    idf(
        "table",
        InputKind::Table {
            cols: 3,
            min_rows: 2,
            max_rows: 10,
            cell: CellKind::Real,
            strata: Some(9),
        },
        None,
        None,
    ),
    idf(
        "strata",
        InputKind::Integer { min: 1, max: 9 },
        Some(DefaultValue::Number(1.0)),
        None,
    ),
];

const DOSE_RESPONSE_OUTPUTS: &[OutputDef] = &[
    od("calc.dose_response.crude.odds_ratio[]", CF, "`epiR::epi.2by2`"),
    od("calc.dose_response.mh_or[]", CF, "`stats::mantelhaen.test`"),
    od("calc.dose_response.strata[].odds_ratio[]", CF, "`epiR::epi.2by2`"),
    od("calc.dose_response.xmh.cc.p", CF, "`fixtures/R/methods/dose_response.R::xmh_trend` — Rosner, Fundamentals of Biostatistics 5th ed., p.606"),
    od("calc.dose_response.xmh.cc.value", CF, "`fixtures/R/methods/dose_response.R::xmh_trend` — Rosner, Fundamentals of Biostatistics 5th ed., p.606"),
    od("calc.dose_response.xmh.uncorrected.p", CF, "`fixtures/R/methods/dose_response.R::xmh_trend` — Schlesselman 1982 p.203"),
    od("calc.dose_response.xmh.uncorrected.value", CF, "`fixtures/R/methods/dose_response.R::xmh_trend` — Schlesselman 1982 p.203"),
];
pub(crate) static DOSE_RESPONSE: CalculatorDef = CalculatorDef {
    id: "dose_response",
    family: Family::Counts,
    inputs: DOSE_RESPONSE_INPUTS,
    groups: &[],
    outputs: DOSE_RESPONSE_OUTPUTS,
};

const MATCHED_CASE_CONTROL_INPUTS: &[InputDef] = &[
    idf(
        "controls_per_case",
        InputKind::Integer { min: 1, max: 4 },
        Some(DefaultValue::Number(1.0)),
        None,
    ),
    idf(
        "matched_sets",
        InputKind::Table {
            cols: 0,
            min_rows: 2,
            max_rows: 2,
            cell: CellKind::Count,
            strata: None,
        },
        None,
        None,
    ),
    idf(
        "conf_level",
        InputKind::ConfLevel,
        Some(DefaultValue::Number(95.0)),
        None,
    ),
];

const MATCHED_CASE_CONTROL_OUTPUTS: &[OutputDef] = &[
    od("calc.matched_case_control.cmle_or.fisher.lower", IT, "`exact2x2::exact2x2(paired=TRUE, tsmethod=\"central\")`"),
    od("calc.matched_case_control.cmle_or.fisher.upper", IT, "`exact2x2::exact2x2(paired=TRUE, tsmethod=\"central\")`"),
    od("calc.matched_case_control.cmle_or.midp.est", IT, "`exact2x2::exact2x2(paired=TRUE, midp=TRUE)`"),
    od("calc.matched_case_control.cmle_or.midp.lower", IT, "`exact2x2::exact2x2(paired=TRUE, midp=TRUE)`"),
    od("calc.matched_case_control.cmle_or.midp.upper", IT, "`exact2x2::exact2x2(paired=TRUE, midp=TRUE)`"),
    od("calc.matched_case_control.discordant_pairs", CF, "`fixtures/R/methods/matched.R::discordant_pairs` — discordant pair count b+c as in OpenEpi `MatchCC.js` / EI7 `pmccTable.cs`"),
    od("calc.matched_case_control.exact.fisher_p_1t", IT, "`stats::binom.test` on discordants"),
    od("calc.matched_case_control.exact.fisher_p_2t", IT, "`stats::binom.test` on discordants"),
    od("calc.matched_case_control.exact.midp_p", IT, "`exact2x2::exact2x2(paired=TRUE, midp=TRUE)`"),
    od("calc.matched_case_control.matched_n.cmle_or.est", IT, "`survival::clogit`"),
    od("calc.matched_case_control.matched_n.cmle_or.lower", IT, "`survival::clogit`"),
    od("calc.matched_case_control.matched_n.cmle_or.upper", IT, "`survival::clogit`"),
    od("calc.matched_case_control.matched_n.mh_chisq.p", CF, "`stats::mantelhaen.test(correct=FALSE)`"),
    od("calc.matched_case_control.matched_n.mh_chisq.value", CF, "`stats::mantelhaen.test(correct=FALSE)`"),
    od("calc.matched_case_control.matched_n.mh_chisq_cc.p", CF, "`stats::mantelhaen.test(correct=TRUE)`"),
    od("calc.matched_case_control.matched_n.mh_chisq_cc.value", CF, "`stats::mantelhaen.test(correct=TRUE)`"),
    od("calc.matched_case_control.matched_n.mh_or.est", CF, "`stats::mantelhaen.test`"),
    od("calc.matched_case_control.matched_n.mh_or.lower", CF, "`stats::mantelhaen.test`"),
    od("calc.matched_case_control.matched_n.mh_or.upper", CF, "`stats::mantelhaen.test`"),
    od("calc.matched_case_control.mcnemar.p", CF, "`stats::mcnemar.test(correct=FALSE)`"),
    od("calc.matched_case_control.mcnemar.value", CF, "`stats::mcnemar.test(correct=FALSE)`"),
    od("calc.matched_case_control.mcnemar_cc.p", CF, "`stats::mcnemar.test(correct=TRUE)`"),
    od("calc.matched_case_control.mcnemar_cc.value", CF, "`stats::mcnemar.test(correct=TRUE)`"),
    od("calc.matched_case_control.or.est", CF, "`fixtures/R/methods/matched.R::matched_or_taylor` — b/c with log CI √(1/b+1/c)"),
    od("calc.matched_case_control.or.lower", CF, "`fixtures/R/methods/matched.R::matched_or_taylor` — b/c with log CI √(1/b+1/c)"),
    od("calc.matched_case_control.or.upper", CF, "`fixtures/R/methods/matched.R::matched_or_taylor` — b/c with log CI √(1/b+1/c)"),
];
pub(crate) static MATCHED_CASE_CONTROL: CalculatorDef = CalculatorDef {
    id: "matched_case_control",
    family: Family::Counts,
    inputs: MATCHED_CASE_CONTROL_INPUTS,
    groups: &[],
    outputs: MATCHED_CASE_CONTROL_OUTPUTS,
};

const SCREENING_INPUTS: &[InputDef] = &[
    idf(
        "table",
        InputKind::Table {
            cols: 2,
            min_rows: 2,
            max_rows: 10,
            cell: CellKind::Count,
            strata: None,
        },
        None,
        None,
    ),
    idf(
        "conf_level",
        InputKind::ConfLevel,
        Some(DefaultValue::Number(95.0)),
        None,
    ),
];

const SCREENING_OUTPUTS: &[OutputDef] = &[
    od("calc.screening.accuracy.est", CF, "`PropCIs::scoreci`"),
    od("calc.screening.accuracy.lower", CF, "`PropCIs::scoreci`"),
    od("calc.screening.accuracy.upper", CF, "`PropCIs::scoreci`"),
    od(
        "calc.screening.bias_index",
        CF,
        "`fixtures/R/methods/screening.R::bias_index` — OpenEpi `DiagnosticTest.js`",
    ),
    od(
        "calc.screening.diagnostic_odds.est",
        CF,
        "fn) with √(1/tp+1/fp+1/fn+1/tn) log CI",
    ),
    od(
        "calc.screening.diagnostic_odds.lower",
        CF,
        "fn) with √(1/tp+1/fp+1/fn+1/tn) log CI",
    ),
    od(
        "calc.screening.diagnostic_odds.upper",
        CF,
        "fn) with √(1/tp+1/fp+1/fn+1/tn) log CI",
    ),
    od(
        "calc.screening.entropy.negative",
        CF,
        "`fixtures/R/methods/screening.R::entropy_reduction` — OpenEpi `DiagnosticTest.js` formula",
    ),
    od(
        "calc.screening.entropy.positive",
        CF,
        "`fixtures/R/methods/screening.R::entropy_reduction` — OpenEpi `DiagnosticTest.js` formula",
    ),
    od("calc.screening.kappa.est", CF, "`epiR::epi.kappa`"),
    od("calc.screening.kappa.lower", CF, "`epiR::epi.kappa`"),
    od("calc.screening.kappa.upper", CF, "`epiR::epi.kappa`"),
    od(
        "calc.screening.level_lr[].est",
        CF,
        "`fixtures/R/methods/screening.R::level_lr` — log-method LR CI as in `DiagnosticTest.js`",
    ),
    od(
        "calc.screening.level_lr[].lower",
        CF,
        "`fixtures/R/methods/screening.R::level_lr` — log-method LR CI as in `DiagnosticTest.js`",
    ),
    od(
        "calc.screening.level_lr[].upper",
        CF,
        "`fixtures/R/methods/screening.R::level_lr` — log-method LR CI as in `DiagnosticTest.js`",
    ),
    od("calc.screening.lr_neg.est", CF, "fixtures/R/calc.R"),
    od("calc.screening.lr_neg.lower", CF, "fixtures/R/calc.R"),
    od("calc.screening.lr_neg.upper", CF, "fixtures/R/calc.R"),
    od("calc.screening.lr_pos.est", CF, "fixtures/R/calc.R"),
    od("calc.screening.lr_pos.lower", CF, "fixtures/R/calc.R"),
    od("calc.screening.lr_pos.upper", CF, "fixtures/R/calc.R"),
    od("calc.screening.npv.est", CF, "`PropCIs::scoreci`"),
    od("calc.screening.npv.lower", CF, "`PropCIs::scoreci`"),
    od("calc.screening.npv.upper", CF, "`PropCIs::scoreci`"),
    od("calc.screening.ppv.est", CF, "`PropCIs::scoreci`"),
    od("calc.screening.ppv.lower", CF, "`PropCIs::scoreci`"),
    od("calc.screening.ppv.upper", CF, "`PropCIs::scoreci`"),
    od("calc.screening.sensitivity.est", CF, "`PropCIs::scoreci`"),
    od("calc.screening.sensitivity.lower", CF, "`PropCIs::scoreci`"),
    od("calc.screening.sensitivity.upper", CF, "`PropCIs::scoreci`"),
    od("calc.screening.specificity.est", CF, "`PropCIs::scoreci`"),
    od("calc.screening.specificity.lower", CF, "`PropCIs::scoreci`"),
    od("calc.screening.specificity.upper", CF, "`PropCIs::scoreci`"),
];
pub(crate) static SCREENING: CalculatorDef = CalculatorDef {
    id: "screening",
    family: Family::Counts,
    inputs: SCREENING_INPUTS,
    groups: &[],
    outputs: SCREENING_OUTPUTS,
};

const SMR_INPUTS: &[InputDef] = &[
    idf("observed", InputKind::Count, None, None),
    idf(
        "expected",
        InputKind::Real {
            min: f64::MIN_POSITIVE,
            max: f64::MAX,
        },
        None,
        None,
    ),
    idf(
        "conf_level",
        InputKind::Percent,
        Some(DefaultValue::Number(95.0)),
        None,
    ),
];

const SMR_OUTPUTS: &[OutputDef] = &[
    od("calc.smr.ci.byar.lower", CF, "`epitools::pois.byar` scaled"),
    od("calc.smr.ci.byar.upper", CF, "`epitools::pois.byar` scaled"),
    od(
        "calc.smr.ci.fisher.lower",
        IT,
        "`exactci::poisson.exact(tsmethod=\"central\")`",
    ),
    od(
        "calc.smr.ci.fisher.upper",
        IT,
        "`exactci::poisson.exact(tsmethod=\"central\")`",
    ),
    od(
        "calc.smr.ci.midp.lower",
        IT,
        "`exactci::poisson.exact(midp=TRUE)`",
    ),
    od(
        "calc.smr.ci.midp.upper",
        IT,
        "`exactci::poisson.exact(midp=TRUE)`",
    ),
    od(
        "calc.smr.ci.rothman_greenland.lower",
        CF,
        "`fixtures/R/methods/poisson.R::rg_ci` — Rothman & Greenland, Modern Epidemiology 2e",
    ),
    od(
        "calc.smr.ci.rothman_greenland.upper",
        CF,
        "`fixtures/R/methods/poisson.R::rg_ci` — Rothman & Greenland, Modern Epidemiology 2e",
    ),
    od(
        "calc.smr.ci.ury_wiggins.lower",
        CF,
        "`fixtures/R/methods/poisson.R::ury_wiggins` — Am J Epidemiol 1985;122:197-8",
    ),
    od(
        "calc.smr.ci.ury_wiggins.upper",
        CF,
        "`fixtures/R/methods/poisson.R::ury_wiggins` — Am J Epidemiol 1985;122:197-8",
    ),
    od(
        "calc.smr.ci.vandenbroucke.lower",
        CF,
        "`fixtures/R/methods/poisson.R::vandenbroucke` — Am J Epidemiol 1982;115:303-4",
    ),
    od(
        "calc.smr.ci.vandenbroucke.upper",
        CF,
        "`fixtures/R/methods/poisson.R::vandenbroucke` — Am J Epidemiol 1982;115:303-4",
    ),
    od("calc.smr.estimate", CF, "`epiR::epi.smr` (`est`)"),
    od(
        "calc.smr.test.byar_chisq",
        CF,
        "`fixtures/R/methods/poisson.R::byar_stat` — Rothman & Boice 1979",
    ),
    od(
        "calc.smr.test.byar_p",
        CF,
        "`fixtures/R/methods/poisson.R::byar_stat` — Rothman & Boice 1979",
    ),
    od("calc.smr.test.chisq_p", CF, "`stats::pchisq`"),
    od("calc.smr.test.chisq_value", CF, "`stats::pchisq`"),
    od(
        "calc.smr.test.fisher_p",
        IT,
        "`exactci::poisson.exact(tsmethod=\"central\")`",
    ),
    od(
        "calc.smr.test.midp_p",
        IT,
        "`exactci::poisson.exact(midp=TRUE)`",
    ),
];
pub(crate) static SMR: CalculatorDef = CalculatorDef {
    id: "smr",
    family: Family::Counts,
    inputs: SMR_INPUTS,
    groups: &[],
    outputs: SMR_OUTPUTS,
};

const BINOMIAL_INPUTS: &[InputDef] = &[
    idf("numerator", InputKind::Count, None, None),
    idf("denominator", InputKind::Count, None, None),
    idf("expected_percent", InputKind::Percent, None, None),
];

const BINOMIAL_OUTPUTS: &[OutputDef] = &[
    od("calc.binomial.ci_95_count.lower", CF, "`stats::qbeta`"),
    od("calc.binomial.ci_95_count.upper", CF, "`stats::qbeta`"),
    od("calc.binomial.p_2t", CF, "`stats::pbinom`"),
    od("calc.binomial.tail.eq", CF, "`stats::dbinom`"),
    od("calc.binomial.tail.ge", CF, "`stats::pbinom`"),
    od("calc.binomial.tail.gt", CF, "`stats::pbinom`"),
    od("calc.binomial.tail.le", CF, "`stats::pbinom`"),
    od("calc.binomial.tail.lt", CF, "`stats::pbinom`"),
];
pub(crate) static BINOMIAL: CalculatorDef = CalculatorDef {
    id: "binomial",
    family: Family::Counts,
    inputs: BINOMIAL_INPUTS,
    groups: &[],
    outputs: BINOMIAL_OUTPUTS,
};

const POISSON_INPUTS: &[InputDef] = &[
    idf("observed", InputKind::Count, None, None),
    idf(
        "expected",
        InputKind::Real {
            min: f64::MIN_POSITIVE,
            max: f64::MAX,
        },
        None,
        None,
    ),
];

const POISSON_OUTPUTS: &[OutputDef] = &[
    od("calc.poisson.tail.eq", CF, "`stats::dpois`"),
    od("calc.poisson.tail.ge", CF, "`stats::ppois`"),
    od("calc.poisson.tail.gt", CF, "`stats::ppois`"),
    od("calc.poisson.tail.le", CF, "`stats::ppois`"),
    od("calc.poisson.tail.lt", CF, "`stats::ppois`"),
];
pub(crate) static POISSON: CalculatorDef = CalculatorDef {
    id: "poisson",
    family: Family::Counts,
    inputs: POISSON_INPUTS,
    groups: &[],
    outputs: POISSON_OUTPUTS,
};
