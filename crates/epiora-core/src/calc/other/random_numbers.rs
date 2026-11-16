//! `random_numbers` — `count` uniform integers in `[lower, upper]`
//! from a seeded stream (spec §5.2; the algorithm is fixed forever by
//! the plan's Interfaces):
//!
//! `ChaCha20Rng::seed_from_u64(seed)`; each draw is Lemire's
//! nearly-divisionless bounded integer over `span = upper − lower +
//! 1` — multiply `next_u32()` by the span, keep the high 32 bits,
//! reject and redraw while the low 32 bits are below
//! `(2^32 − span) mod span`. `unique = yes` redraws duplicates;
//! `sorted = yes` sorts ascending after drawing. i64 output; no
//! floats anywhere in the generation path.
//!
//! Bounds (owner decision C2): `lower ≤ upper`,
//! `upper − lower + 1 ≤ 2^32`, `unique = yes` requires
//! `count ≤ span` — else the `calc_input_out_of_range` typed input
//! error. `columns` is display-only and never touches the values.

use super::super::counts::shared::cnt;
use crate::EngineError;
use rand_chacha::ChaCha20Rng;
use rand_core::{Rng, SeedableRng};
use serde_json::{json, Value};

fn out_of_range(input: &str, reason: &str) -> EngineError {
    EngineError::CalcInputOutOfRange {
        input: input.into(),
        reason: reason.into(),
    }
}

/// Lemire's rejection threshold `(2^32 − span) mod span` on the low
/// product — 0 at `span = 2^32` (the whole u32 range: no rejection).
fn threshold(span: u64) -> u64 {
    ((1u64 << 32) - span) % span
}

/// One bounded draw: `high32(x·span)` with the low product rejected
/// below the threshold — Lemire's nearly-divisionless method.
fn draw_bounded(rng: &mut ChaCha20Rng, span: u64) -> u64 {
    let t = threshold(span);
    loop {
        let m = u64::from(rng.next_u32()) * span;
        if (m & 0xffff_ffff) >= t {
            return m >> 32;
        }
    }
}

/// `n` sequential draws over `[lower, lower + span)` — the stream the
/// golden and χ²-uniformity tests measure.
pub fn draw_values(seed: u64, lower: i64, span: u64, n: usize) -> Vec<i64> {
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    (0..n)
        .map(|_| lower + draw_bounded(&mut rng, span) as i64)
        .collect()
}

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let lower = inputs["lower"].as_i64().unwrap_or(0);
    let upper = inputs["upper"].as_i64().unwrap_or(0);
    if lower > upper {
        return Err(out_of_range("upper", "below_lower"));
    }
    let span = i128::from(upper) - i128::from(lower) + 1;
    if span > (1i128 << 32) {
        return Err(out_of_range("upper", "span_too_wide"));
    }
    let span = span as u64;
    let count = cnt(inputs, "count") as usize;
    let unique = inputs["unique"].as_str() == Some("yes");
    if unique && count as u64 > span {
        return Err(out_of_range("count", "unique_impossible"));
    }
    let seed = cnt(inputs, "seed");

    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let mut values = Vec::with_capacity(count);
    if unique {
        let mut seen = std::collections::HashSet::with_capacity(count);
        while values.len() < count {
            let v = lower + draw_bounded(&mut rng, span) as i64;
            if seen.insert(v) {
                values.push(v);
            }
        }
    } else {
        values.extend((0..count).map(|_| lower + draw_bounded(&mut rng, span) as i64));
    }
    if inputs["sorted"].as_str() == Some("yes") {
        values.sort_unstable();
    }

    Ok(json!({ "values": values }))
}
