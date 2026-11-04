//! The validation registry (spec §4.3), split one file per statistic
//! family so every file stays under the ~400-line limit. `ALL`
//! flattens the per-family slices into one ordered array.

mod freq;
mod rxc;
mod stratified_two_by_two;
mod two_by_two;

use super::{Coverage, ToleranceClass};

const LEN: usize = freq::ENTRIES.len()
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
    freq::ENTRIES,
    rxc::ENTRIES,
    stratified_two_by_two::ENTRIES,
    two_by_two::ENTRIES,
]);
