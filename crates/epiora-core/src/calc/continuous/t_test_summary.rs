//! `t_test_summary` — Student and Welch t-tests plus the
//! variance-ratio F test from two groups' `n`/`mean`/`sd`. The t-test
//! pair shares `stats::means_summary::t_tests_from_summary_at` with
//! MEANS; the F test is `stats::var.test`'s two-sided p
//! (`2·min(F_cdf, 1 − F_cdf)`), per `fixtures/R/calc.R`.

use super::super::counts::shared::{cnt, conf, leaf, num, undefined};
use super::est_leaf;
use crate::special::f_sf;
use crate::stats::{t_tests_from_summary_at, GroupSummary, TTest};
use crate::EngineError;
use serde_json::{json, Value};

fn t_block(t: Option<&TTest>) -> Value {
    match t {
        Some(t) => json!({
            "t": est_leaf(&t.t),
            "df": est_leaf(&t.df),
            "p_2t": est_leaf(&t.p_value),
            "diff": est_leaf(&t.diff),
            "lower": est_leaf(&t.diff_ci_95.lower),
            "upper": est_leaf(&t.diff_ci_95.upper),
        }),
        // `t_tests_from_summary` returns None when a group is below
        // two observations or the standard error is "essentially
        // constant" — R's t.test errors; the calculator reports
        // `zero_variance` (the corpus has no such case).
        None => {
            let u = undefined("zero_variance");
            json!({
                "t": u, "df": u, "p_2t": u, "diff": u, "lower": u, "upper": u,
            })
        }
    }
}

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let c = conf(inputs);
    let a = GroupSummary {
        n: cnt(inputs, "n_1"),
        mean: num(inputs, "mean_1"),
        variance: Some({
            let s = num(inputs, "sd_1");
            s * s
        }),
    };
    let b = GroupSummary {
        n: cnt(inputs, "n_2"),
        mean: num(inputs, "mean_2"),
        variance: Some({
            let s = num(inputs, "sd_2");
            s * s
        }),
    };
    let tests = t_tests_from_summary_at(&a, &b, c);
    let v1 = a.variance.unwrap_or(f64::NAN);
    let v2 = b.variance.unwrap_or(f64::NAN);
    let f = v1 / v2;
    let (df_n, df_d) = ((a.n - 1) as f64, (b.n - 1) as f64);
    let pf_f = 1.0 - f_sf(f, df_n, df_d);
    let p_f = 2.0 * pf_f.min(1.0 - pf_f);
    Ok(json!({
        "pooled": t_block(tests.as_ref().map(|t| &t.pooled)),
        "welch": t_block(tests.as_ref().map(|t| &t.welch)),
        "f_variance": {
            "f": leaf(f),
            "df_num": leaf(df_n),
            "df_den": leaf(df_d),
            "p": leaf(p_f),
        },
    }))
}
