//! Native `criterion` bench of the spec §5.3 workload: `Dataset::new`
//! (the native half of `datasetFromColumns`) plus stratified `tables`
//! on 1,000,000 rows. Reported only — the gate is the Chromium run in
//! `crates/epiora-wasm/bench/`.
//!
//! The generator is the same SplitMix32, seed (20261031) and draw
//! order as `crates/epiora-wasm/bench/gen.mjs`: every EXPOSED row,
//! then ILL, then REGION; one u32 per cell. Boolean columns mark
//! `u % 100 == 0` (~1%) missing and take bit 8 as the value; REGION
//! codes are `u % 3` into {"north", "south", "east"} with no missing.
//!
//!   cargo bench -p epiora-core --bench tables_1m

use criterion::{criterion_group, criterion_main, Criterion};
use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_core::procedures::{tables, RunContext, TablesOptions, TablesResult, VarSpec};
use std::time::Duration;

const SEED: u32 = 20261031;
const ROWS: usize = 1_000_000;
const REGION_LEVELS: [&str; 3] = ["north", "south", "east"];

struct SplitMix32 {
    state: u32,
}

impl SplitMix32 {
    fn new(seed: u32) -> Self {
        SplitMix32 { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x9e37_79b9);
        let mut z = self.state;
        z = (z ^ (z >> 16)).wrapping_mul(0x21f0_aaad);
        z = (z ^ (z >> 15)).wrapping_mul(0x735a_2d97);
        z ^ (z >> 15)
    }
}

fn boolean_column(name: &str, n: usize, rng: &mut SplitMix32) -> Column {
    let mut values = Vec::with_capacity(n);
    for _ in 0..n {
        let u = rng.next_u32();
        values.push(if u.is_multiple_of(100) {
            None
        } else {
            Some((u >> 8) & 1 == 1)
        });
    }
    Column {
        name: name.to_string(),
        data: ColumnData::Boolean(values),
    }
}

fn region_column(n: usize, rng: &mut SplitMix32) -> Column {
    let mut values = Vec::with_capacity(n);
    for _ in 0..n {
        values.push(Some(
            REGION_LEVELS[(rng.next_u32() % 3) as usize].to_string(),
        ));
    }
    Column {
        name: "REGION".to_string(),
        data: ColumnData::Text(values),
    }
}

fn generate(n: usize, seed: u32) -> Vec<Column> {
    let mut rng = SplitMix32::new(seed);
    vec![
        boolean_column("EXPOSED", n, &mut rng),
        boolean_column("ILL", n, &mut rng),
        region_column(n, &mut rng),
    ]
}

fn options() -> TablesOptions {
    TablesOptions {
        exposure: VarSpec {
            name: "EXPOSED".to_string(),
            levels: None,
        },
        outcome: VarSpec {
            name: "ILL".to_string(),
            levels: None,
        },
        stratify_by: vec!["REGION".to_string()],
        include_missing: false,
    }
}

fn tables_1m(c: &mut Criterion) {
    // Generated once, outside the timer, matching the page-side row
    // generation in bench.spec.mjs.
    let columns = generate(ROWS, SEED);
    let opts = options();
    let ctx = RunContext {
        timestamp: "2026-10-31T00:00:00Z".to_string(),
    };

    // Same pin as bench.spec.mjs: the first 3 EXPOSED values of
    // generate(…, 20261031) are 1, 1, 1 (verified against gen.mjs).
    if let ColumnData::Boolean(values) = &columns[0].data {
        assert_eq!(
            values[..3],
            [Some(true), Some(true), Some(true)],
            "SplitMix32 drifted from gen.mjs"
        );
    }
    let ds = Dataset::new("bench-1m", 1, columns.clone());
    let ds = match ds {
        Ok(ds) => ds,
        Err(e) => panic!("Dataset::new failed: {e}"),
    };
    match tables(&ds, &opts, &ctx) {
        Ok(out) => match out.result {
            TablesResult::TwoByTwo {
                stratified: Some(ref s),
                ..
            } => assert_eq!(s.strata_labels.len(), 3, "expected three strata"),
            _ => panic!("expected a stratified TwoByTwo result"),
        },
        Err(e) => panic!("tables failed: {e}"),
    }

    c.bench_function("dataset_new_plus_stratified_tables_1m", |b| {
        b.iter(|| {
            // Timed: dataset construction (the datasetFromColumns half)
            // plus tables — the same two calls the Chromium gate times.
            let ds = match Dataset::new("bench-1m", 1, columns.clone()) {
                Ok(ds) => ds,
                Err(e) => panic!("Dataset::new failed: {e}"),
            };
            std::hint::black_box(tables(&ds, &opts, &ctx))
        })
    });
}

criterion_group! {
    name = benches;
    // 1M-row iterations are slow; a short measurement keeps the run
    // bounded (report only, no gate).
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(5))
        .sample_size(10);
    targets = tables_1m
}
criterion_main!(benches);
