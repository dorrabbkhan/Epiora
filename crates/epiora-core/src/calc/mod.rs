//! `calc` — the v1.0 calculator surface (spec §5.3). `REGISTRY` lists
//! every calculator the engine knows, in `docs/v1.0-calculators.md`
//! inventory order; `run` validates the inputs against the definition,
//! dispatches the calculator and seals the result in a v3 receipt.
//! Only the counts family is implemented in Task 7; the other families
//! carry their inventory metadata and are added by Tasks 8 and 9.

pub mod continuous;
pub mod counts;
pub mod input;
pub mod person_time;
mod registry;
mod registry_continuous;
mod registry_power;
mod registry_rate;
mod registry_sample_size;
mod registry_sample_size_rest;

use crate::error::EngineError;
use crate::procedures::RunContext;
use crate::receipt::{CalculatorStamp, EngineStamp, Receipt, ValidationEntry};
use crate::validation::{statistic_ids, ToleranceClass};
use serde::Serialize;
use std::collections::BTreeMap;

/// Calculator family (spec §5.3, inventory families).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// Counts on 2×2/R×C/matched tables.
    Counts,
    /// Person-time rates.
    PersonTime,
    /// Continuous summary data.
    Continuous,
    /// Sample-size calculators.
    SampleSize,
    /// Power calculators.
    Power,
    /// Other (random numbers).
    Other,
}

/// The kind of a table cell in a `table` input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum CellKind {
    /// Non-negative integer `< 2^53`.
    Count,
    /// Finite real.
    Real,
}

/// The declared shape of one calculator input (spec §5.3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    /// Non-negative integer `< 2^53`.
    Count,
    /// Integer in `[min, max]`.
    Integer { min: i64, max: i64 },
    /// Finite real in `[min, max]`.
    Real { min: f64, max: f64 },
    /// Real in the open interval `(0, 1)`.
    Proportion,
    /// Real in `[0, 100]`.
    Percent,
    /// Exactly 90, 95 or 99.
    ConfLevel,
    /// String in `options`.
    Choice { options: &'static [&'static str] },
    /// A table: `min_rows..=max_rows` rows; each row `cols` cells of
    /// `cell` kind — or `cols == 0` for a calculator-defined column
    /// count (uniform across rows, 2..=10 wide). `strata` is the
    /// inventory's strata bound: `Some(bound)` means the input is a
    /// strata list — the flat single-stratum form or a list of
    /// `1..=bound` tables of this spec; `None` rejects a nested
    /// list-of-tables as `bad_table_shape`.
    Table {
        cols: u8,
        min_rows: u8,
        max_rows: u8,
        cell: CellKind,
        strata: Option<u8>,
    },
}

/// What a missing input is filled with.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "snake_case")]
pub enum DefaultValue {
    /// A numeric default.
    Number(f64),
    /// A string default (choices).
    Choice(&'static str),
    /// Optional input: omitted when absent, never filled.
    Omit,
}

/// One declared input of a calculator.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct InputDef {
    /// Input name as it appears in the JSON inputs object.
    pub name: &'static str,
    /// Its declared kind.
    pub kind: InputKind,
    /// Missing-input behaviour: `None` = required,
    /// `Some(Number|Choice)` = filled, `Some(Omit)` = optional.
    pub default: Option<DefaultValue>,
    /// Optional unit label for display.
    pub unit: Option<&'static str>,
}

/// One emitted statistic of a calculator.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct OutputDef {
    /// `calc.<id>.<path>` registry id, `[]` for array positions.
    pub statistic: &'static str,
    /// Its tolerance class.
    pub class: ToleranceClass,
    /// Method label (the inventory's method/source name).
    pub method: &'static str,
}

/// The definition of one calculator: id, family, declared inputs and
/// emitted statistics — the inventory data of
/// `docs/v1.0-calculators.md`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct CalculatorDef {
    /// Registry id.
    pub id: &'static str,
    /// Calculator family.
    pub family: Family,
    /// Declared inputs.
    pub inputs: &'static [InputDef],
    /// Emitted statistics.
    pub outputs: &'static [OutputDef],
}

/// `od` — one registry output row (registry data tables).
pub(super) const fn od(
    statistic: &'static str,
    class: ToleranceClass,
    method: &'static str,
) -> OutputDef {
    OutputDef {
        statistic,
        class,
        method,
    }
}

/// `idf` — one declared input (registry data tables).
pub(super) const fn idf(
    name: &'static str,
    kind: InputKind,
    default: Option<DefaultValue>,
    unit: Option<&'static str>,
) -> InputDef {
    InputDef {
        name,
        kind,
        default,
        unit,
    }
}

/// The calculator registry: all 24 inventory ids in inventory order.
pub static REGISTRY: &[CalculatorDef] = &registry::DEFS;

/// The registry definition of `id`, or `CalcUnknown`.
pub fn definition(id: &str) -> Result<&'static CalculatorDef, EngineError> {
    registry::DEFS
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| EngineError::CalcUnknown { id: id.into() })
}

/// The output of one calculator run.
#[derive(Debug, Clone)]
pub struct CalcOutput {
    /// The result tree, keyed by the output paths the definition
    /// declares; leaves are numbers or `{"undefined": reason}`.
    pub result: serde_json::Value,
    /// The v3 receipt (`calculator` stamp, no dataset).
    pub receipt: Receipt,
}

/// Runs calculator `id` on `inputs`: validates them against the
/// definition, computes, and seals a v3 receipt. Non-counts families
/// are registered but not implemented in Task 7 — they fail closed
/// with `invalid_options`, a defined error rather than a guess.
pub fn run(
    id: &str,
    inputs: &serde_json::Value,
    ctx: &RunContext,
) -> Result<CalcOutput, EngineError> {
    if ctx.timestamp.is_empty() {
        return Err(EngineError::InvalidOptions {
            reason: "RunContext.timestamp is empty".into(),
        });
    }
    let def = definition(id)?;
    if !matches!(
        def.family,
        Family::Counts | Family::PersonTime | Family::Continuous
    ) {
        return Err(EngineError::InvalidOptions {
            reason: format!("calculator {id} is registered but its family is not implemented yet"),
        });
    }
    let canonical = input::validate(def, inputs)?;
    let result = match def.family {
        Family::Counts => counts::run(def.id, &canonical)?,
        Family::PersonTime => person_time::run(def.id, &canonical)?,
        Family::Continuous => continuous::run(def.id, &canonical)?,
        _ => unreachable!("family gate above"),
    };

    let mut entries = BTreeMap::new();
    for path in statistic_ids(&result) {
        let stat = format!("calc.{id}.{path}");
        if let Some(c) = crate::validation::REGISTRY
            .iter()
            .find(|c| c.statistic == stat)
        {
            entries.insert(
                stat.clone(),
                ValidationEntry {
                    statistic: stat,
                    fixtures: c.fixtures.iter().map(|f| (*f).to_string()).collect(),
                    tolerance_class: c.class,
                    tolerance: c.class.tolerance(),
                },
            );
        }
    }
    let stamp = CalculatorStamp {
        id: id.to_string(),
        inputs: canonical.clone(),
    };
    let receipt = Receipt::for_calculator(
        stamp,
        format!("calc.{id}"),
        canonical,
        entries.into_values().collect(),
        ctx.timestamp.clone(),
        EngineStamp::current(),
    );
    Ok(CalcOutput { result, receipt })
}
