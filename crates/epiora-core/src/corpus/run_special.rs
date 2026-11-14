//! `run_special` — runs one `special.*` corpus fixture through the
//! matching `special` function and returns `{"values": [...]}` for
//! `compare` (v0.2 spec §6.3). The v0.2 grids carry `inputs.points`;
//! the v1.0 spec §4.1 grids carry `inputs.grid` (plan Task 2) — both
//! are accepted. Root-found results serialize as `{"undefined": code}`
//! on `Err`, matching a `{"value": null}` expected leaf.

use super::super::special::{
    beta_quantile, binomial_quantile, binomial_tail, chisq_quantile, chisq_sf, f_sf,
    gamma_quantile, hypergeom_pmf, ln_gamma, normal_cdf, normal_quantile, normal_sf, poisson_tail,
    t_cdf, t_quantile, t_sf, NcHypergeom, RootError,
};
use crate::EngineError;
use serde_json::{json, Value};

fn corpus(reason: impl Into<String>) -> EngineError {
    EngineError::CorpusFormat {
        reason: reason.into(),
    }
}

/// A failed root solve as a corpus leaf: `{"undefined": "not_bracketed"
/// | "no_convergence"}` so a fixture can mark the point undefined
/// (spec §6.2); a numeric expected then fails as a mismatch, not a
/// corpus error.
fn undefined(e: RootError) -> Value {
    json!({
        "undefined": match e {
            RootError::NotBracketed => "not_bracketed",
            RootError::NoConvergence => "no_convergence",
        }
    })
}

/// `special.<fn>`: the grid is an array of argument arrays;
/// `actual` is `{"values": [...]}` matching `expected.values`.
pub fn run_special(fixture: &Value) -> Result<Value, EngineError> {
    let id = fixture["id"].as_str().unwrap_or("special");
    let name = &fixture["procedure"].as_str().unwrap_or("")[8..];
    let inputs = &fixture["inputs"];
    let points = inputs["grid"]
        .as_array()
        .or_else(|| inputs["points"].as_array())
        .ok_or_else(|| corpus(format!("{id}: inputs.grid/points is not an array")))?;
    let args: Vec<Vec<f64>> = points
        .iter()
        .map(|p| {
            p.as_array()
                .map(|a| a.iter().filter_map(Value::as_f64).collect())
                .ok_or_else(|| corpus(format!("{id}: point is not an array")))
        })
        .collect::<Result<_, _>>()?;
    let arity = |a: &Vec<f64>, n: usize| -> Result<(), EngineError> {
        if a.len() == n {
            Ok(())
        } else {
            Err(corpus(format!(
                "{id}: special.{name} wants {n} args, got {}",
                a.len()
            )))
        }
    };
    let mut values = Vec::with_capacity(args.len());
    for a in &args {
        values.push(match name {
            "ln_gamma" => {
                arity(a, 1)?;
                json!(ln_gamma(a[0]))
            }
            "normal_cdf" => {
                arity(a, 1)?;
                json!(normal_cdf(a[0]))
            }
            "normal_sf" => {
                arity(a, 1)?;
                json!(normal_sf(a[0]))
            }
            "normal_quantile" => {
                arity(a, 1)?;
                match normal_quantile(a[0]) {
                    Some(v) => json!(v),
                    None => json!({ "undefined": "out_of_range" }),
                }
            }
            "chisq_sf" => {
                arity(a, 2)?;
                json!(chisq_sf(a[0], a[1]))
            }
            "hypergeom_pmf" => {
                arity(a, 4)?;
                json!(hypergeom_pmf(
                    a[0] as u64,
                    a[1] as u64,
                    a[2] as u64,
                    a[3] as u64
                ))
            }
            "binomial_tail" => {
                arity(a, 4)?;
                json!(binomial_tail(a[0] as u64, a[1] as u64, a[2], a[3] == 1.0))
            }
            "beta_quantile" => {
                arity(a, 3)?;
                match beta_quantile(a[0], a[1], a[2]) {
                    Some(v) => json!(v),
                    None => json!({ "undefined": "out_of_range" }),
                }
            }
            "nc_hypergeom" => {
                arity(a, 5)?;
                let h = NcHypergeom::new(a[0] as u64, a[1] as u64, a[2] as u64);
                let (x, psi) = (a[3] as u64, a[4]);
                json!({
                    "pmf": h.pmf(x, psi),
                    "mean": h.mean(psi),
                    "lower_tail": h.lower_tail(x, psi),
                    "upper_tail": h.upper_tail(x, psi),
                })
            }
            "t_cdf" => {
                arity(a, 2)?;
                json!(t_cdf(a[0], a[1]))
            }
            "t_sf" => {
                arity(a, 2)?;
                json!(t_sf(a[0], a[1]))
            }
            "t_quantile" => {
                arity(a, 2)?;
                match t_quantile(a[0], a[1]) {
                    Ok(v) => json!(v),
                    Err(e) => undefined(e),
                }
            }
            "f_sf" => {
                arity(a, 3)?;
                json!(f_sf(a[0], a[1], a[2]))
            }
            "gamma_quantile" => {
                arity(a, 3)?;
                match gamma_quantile(a[0], a[1], a[2]) {
                    Ok(v) => json!(v),
                    Err(e) => undefined(e),
                }
            }
            "chisq_quantile" => {
                arity(a, 2)?;
                match chisq_quantile(a[0], a[1]) {
                    Ok(v) => json!(v),
                    Err(e) => undefined(e),
                }
            }
            "poisson_tail" => {
                arity(a, 3)?;
                json!(poisson_tail(a[0] as u64, a[1], a[2] == 1.0))
            }
            "binomial_quantile" => {
                arity(a, 3)?;
                json!(binomial_quantile(a[0], a[1] as u64, a[2]))
            }
            other => return Err(corpus(format!("{id}: unknown special fn {other:?}"))),
        });
    }
    Ok(json!({ "values": values }))
}
