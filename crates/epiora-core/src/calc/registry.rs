//! The calculator registry: all 24 `docs/v1.0-calculators.md` ids
//! in inventory order. The counts definitions live with their
//! implementations in `counts/defs.rs`/`defs_rest.rs`; the Task 8/9
//! families carry their metadata in the sibling `registry_*` files.

use super::{
    counts::defs, counts::defs_rest, registry_continuous, registry_power, registry_rate,
    registry_sample_size, registry_sample_size_rest,
};
use crate::calc::CalculatorDef;

pub(super) static DEFS: [CalculatorDef; 24] = [
    defs::PROPORTION,
    defs::TWO_BY_TWO,
    defs::TWO_BY_N,
    defs::R_BY_C,
    defs_rest::DOSE_RESPONSE,
    defs_rest::MATCHED_CASE_CONTROL,
    defs_rest::SCREENING,
    defs_rest::SMR,
    defs_rest::BINOMIAL,
    defs_rest::POISSON,
    registry_rate::RATE_ONE,
    registry_rate::RATE_COMPARE,
    registry_continuous::MEAN_CI,
    registry_continuous::MEDIAN_CI,
    registry_continuous::T_TEST_SUMMARY,
    registry_continuous::ANOVA_SUMMARY,
    registry_sample_size::SS_PROPORTION,
    registry_sample_size::SS_UNMATCHED_CC,
    registry_sample_size_rest::SS_COHORT,
    registry_sample_size_rest::SS_MEAN_DIFFERENCE,
    registry_power::POWER_UNMATCHED_CC,
    registry_power::POWER_COHORT,
    registry_power::POWER_MEAN_DIFFERENCE,
    registry_power::RANDOM_NUMBERS,
];
