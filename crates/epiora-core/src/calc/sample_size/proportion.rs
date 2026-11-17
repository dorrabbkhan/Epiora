//! `ss_proportion` — sample size for one proportion at OpenEpi's
//! seven displayed confidence levels. `n_openepi` is the OpenEpi
//! population-survey formula (Lwanga & Lemeshow as `SSPropor.js`
//! writes it: `DEFF·N·p(1−p) / ((d²/z²)·(N−1) + p(1−p))`, folding FPC
//! and DEFF into one expression); `n_ei7`/`cluster_size`/`total` are
//! the Epi Info 7 chain — `n0 = z²·freq·(100−freq)/margin²` (the
//! percent units cancel), FPC `n = round(n0/(1+n0/N))`, then
//! `cluster_real = DEFF·n/clusters`, whose ceiling is both `n_ei7`
//! and `cluster_size`, and `total = clusters × cluster_size`
//! (`fixtures/R/methods/sample_size.R`, inventory `## ss_proportion`).

use crate::calc::counts::shared::{cnt, leaf, num, z};
use crate::EngineError;
use serde_json::Value;

/// `(leaf key, confidence level)` — OpenEpi's seven displayed levels.
const LEVELS: [(&str, f64); 7] = [
    ("80", 80.0),
    ("90", 90.0),
    ("95", 95.0),
    ("97", 97.0),
    ("99", 99.0),
    ("999", 99.9),
    ("9999", 99.99),
];

/// One level's chain — `oe_real` (the OpenEpi formula), `n0`, the
/// FPC-rounded `n_raw`, and the post-DEFF `cs_real` whose ceiling is
/// `cs` — shared by the run and the explain trace (v1.0 Task 10,
/// spec §6).
pub(crate) struct SsPropParts {
    /// `DEFF·N·p(1−p) / ((d²/z²)·(N−1) + p(1−p))`.
    pub oe_real: f64,
    /// `rint(n0/(1 + n0/N))` — the FPC round.
    pub n_raw: f64,
    /// `DEFF·n_raw/clusters`.
    pub cs_real: f64,
    /// `cs_real.ceil()` — the reported cluster size.
    pub cs: f64,
}

/// The `(oe_real, n_raw, cs_real)` chain of `run` at level `za`
/// — shared by the result and the explain trace.
pub(crate) fn level_parts(
    deff: f64,
    n_pop: f64,
    p: f64,
    d: f64,
    factor: f64,
    clusters: f64,
    za: f64,
) -> SsPropParts {
    let za2 = za * za;
    let oe_real = deff * n_pop * p * (1.0 - p) / ((d * d / za2) * (n_pop - 1.0) + p * (1.0 - p));
    let n0 = za2 * factor;
    // R's `round` — nearest, ties to even — is libm::rint.
    let n_raw = libm::rint(n0 / (1.0 + n0 / n_pop));
    let cs_real = deff * n_raw / clusters;
    SsPropParts {
        oe_real,
        n_raw,
        cs_real,
        cs: cs_real.ceil(),
    }
}

/// `freq·(100−freq)/margin²` — the percent-unit numerator of `n0`.
/// Shared by `run` and the explain trace (v1.0 Task 10, spec §6).
pub(crate) fn margin_factor(freq: f64, margin_pct: f64) -> f64 {
    freq * (100.0 - freq) / (margin_pct * margin_pct)
}

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let n_pop = cnt(inputs, "population_size") as f64;
    let freq = num(inputs, "expected_freq");
    // Two scales: the OpenEpi formula's `d` is a fraction; the EI7
    // factor uses the percent directly (freq·(100−freq)/margin²
    // already carries the units through).
    let margin_pct = num(inputs, "margin_error");
    let d = margin_pct / 100.0;
    let deff = num(inputs, "design_effect");
    let clusters = cnt(inputs, "clusters") as f64;
    let p = freq / 100.0;
    let factor = margin_factor(freq, margin_pct);

    let mut n_openepi = serde_json::Map::new();
    let mut n_openepi_real = serde_json::Map::new();
    let mut n_ei7 = serde_json::Map::new();
    let mut n_ei7_real = serde_json::Map::new();
    let mut cluster_size = serde_json::Map::new();
    let mut cluster_size_real = serde_json::Map::new();
    let mut total = serde_json::Map::new();

    for (key, lv) in LEVELS {
        let za = z(lv);
        let p_ = level_parts(deff, n_pop, p, d, factor, clusters, za);
        let (oe_real, cs_real, cs) = (p_.oe_real, p_.cs_real, p_.cs);

        n_openepi.insert(key.into(), leaf(oe_real.ceil()));
        n_openepi_real.insert(key.into(), leaf(oe_real));
        // n_ei7 is the cluster size: EI7 reports the post-DEFF,
        // post-clustering number (ss_prop_ei7 returns it twice).
        n_ei7.insert(key.into(), leaf(cs));
        n_ei7_real.insert(key.into(), leaf(cs_real));
        cluster_size.insert(key.into(), leaf(cs));
        cluster_size_real.insert(key.into(), leaf(cs_real));
        total.insert(key.into(), leaf(cs * clusters));
    }

    Ok(serde_json::json!({
        "n_openepi": n_openepi,
        "n_openepi_real": n_openepi_real,
        "n_ei7": n_ei7,
        "n_ei7_real": n_ei7_real,
        "cluster_size": cluster_size,
        "cluster_size_real": cluster_size_real,
        "total": total,
    }))
}
