//! The validation registry (spec §4.3), split one file per statistic
//! family so every file stays under the ~400-line limit. `ALL`
//! flattens the per-family slices into one ordered array.

mod calc_binomial;
mod calc_dose_response;
mod calc_matched_case_control;
mod calc_poisson;
mod calc_proportion;
mod calc_r_by_c;
mod calc_screening;
mod calc_smr;
mod calc_two_by_n;
mod calc_two_by_two;
mod calc_two_by_two_adjusted;
mod calc_two_by_two_strata;
mod freq;
mod means;
mod rxc;
mod stratified_two_by_two;
mod two_by_two;

use super::{Coverage, ToleranceClass};

const LEN: usize = calc_binomial::ENTRIES.len()
    + calc_dose_response::ENTRIES.len()
    + calc_matched_case_control::ENTRIES.len()
    + calc_poisson::ENTRIES.len()
    + calc_proportion::ENTRIES.len()
    + calc_r_by_c::ENTRIES.len()
    + calc_screening::ENTRIES.len()
    + calc_smr::ENTRIES.len()
    + calc_two_by_n::ENTRIES.len()
    + calc_two_by_two::ENTRIES.len()
    + calc_two_by_two_adjusted::ENTRIES.len()
    + calc_two_by_two_strata::ENTRIES.len()
    + freq::ENTRIES.len()
    + means::ENTRIES.len()
    + rxc::ENTRIES.len()
    + stratified_two_by_two::ENTRIES.len()
    + two_by_two::ENTRIES.len();

const EMPTY: Coverage = Coverage {
    statistic: "",
    fixtures: &[],
    class: ToleranceClass::ClosedForm,
};

/// Compile-time flatten of the family slices into one ordered array.
const fn flatten<const N: usize>(families: &[&'static [Coverage]]) -> [Coverage; N] {
    let mut out = [EMPTY; N];
    let (mut i, mut f) = (0, 0);
    while f < families.len() {
        let mut j = 0;
        while j < families[f].len() {
            out[i] = families[f][j];
            i += 1;
            j += 1;
        }
        f += 1;
    }
    out
}

pub(super) static ALL: [Coverage; LEN] = flatten(&[
    calc_binomial::ENTRIES,
    calc_dose_response::ENTRIES,
    calc_matched_case_control::ENTRIES,
    calc_poisson::ENTRIES,
    calc_proportion::ENTRIES,
    calc_r_by_c::ENTRIES,
    calc_screening::ENTRIES,
    calc_smr::ENTRIES,
    calc_two_by_n::ENTRIES,
    calc_two_by_two::ENTRIES,
    calc_two_by_two_adjusted::ENTRIES,
    calc_two_by_two_strata::ENTRIES,
    freq::ENTRIES,
    means::ENTRIES,
    rxc::ENTRIES,
    stratified_two_by_two::ENTRIES,
    two_by_two::ENTRIES,
]);
