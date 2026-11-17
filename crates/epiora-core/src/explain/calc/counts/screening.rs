//! `screening` — the diagnostic-measure blocks.
use super::*;
use crate::calc::counts::screening::level_lr_parts;
use crate::calc::counts::shared::{prop, simel_parts, wald_parts};
use crate::calc::counts::shared_ci::kappa_parts;

pub(crate) fn screening(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let (tp, fp, fn_, tn) = screening_cells(inputs);
    let conf = conf(inputs);
    let (d, nd) = (tp + fn_, fp + tn);
    let (tpos, tneg, n) = (tp + fp, fn_ + tn, d + nd);
    let (df, dnf) = (d as f64, nd as f64);

    let block = |x: u64, nn: u64, side: &str, est: crate::stats::Estimate| {
        wilson_block(x, nn, conf, statistic, side, est)
    };

    match segs {
        [f, l] if f.key == "level_lr" => {
            let i = f.index.ok_or_else(u)?;
            let t = count_table(inputs, "table");
            let r = t.get(i).ok_or_else(u)?;
            let (ld, ln) = (r[0] as f64, r[1] as f64);
            let (p1, p2, lr, half) = level_lr_parts(ld, ln, df, dnf, conf);
            match l.key.as_str() {
                "est" => Ok(closed(
                    statistic,
                    "lr_ratio",
                    finite(&[("p1", p1), ("p2", p2)]),
                    est,
                )),
                "lower" => Ok(closed(
                    statistic,
                    "level_lr_lower",
                    finite(&[("est", lr), ("half", half)]),
                    est,
                )),
                "upper" => Ok(closed(
                    statistic,
                    "level_lr_upper",
                    finite(&[("est", lr), ("half", half)]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        [f] if f.key == "bias_index" => Ok(closed(
            statistic,
            "bias_index",
            finite(&[("fp", fp as f64), ("fn_", fn_ as f64), ("n", n as f64)]),
            est,
        )),
        [f, l] => {
            match f.key.as_str() {
                "sensitivity" => block(tp, d, &l.key, est),
                "specificity" => block(tn, nd, &l.key, est),
                "ppv" => block(tp, tpos, &l.key, est),
                "npv" => block(tn, tneg, &l.key, est),
                "accuracy" => block(tp + tn, n, &l.key, est),
                "lr_pos" | "lr_neg" => {
                    // `simel_lr(a, c, b, d)`: the positive cell's row
                    // counts in each column.
                    let (a, c, b, dd) = if f.key == "lr_pos" {
                        (tp, fn_, fp, tn)
                    } else {
                        (fn_, tp, tn, fp)
                    };
                    let (p1, p2, lr, se) = simel_parts(a, c, b, dd);
                    if l.key == "est" {
                        Ok(closed(
                            statistic,
                            "lr_ratio",
                            finite(&[("p1", p1), ("p2", p2)]),
                            est,
                        ))
                    } else {
                        wald_exp_block(lr, se, conf, statistic, &l.key, est)
                    }
                }
                "diagnostic_odds" => {
                    let [_, _, _, _, _, _, dod, se] = wald_parts(&crate::stats::Table2x2 {
                        a: tp,
                        b: fp,
                        c: fn_,
                        d: tn,
                    });
                    if l.key == "est" {
                        Ok(closed(
                            statistic,
                            "diagnostic_odds",
                            finite(&[
                                ("tp", tp as f64),
                                ("tn", tn as f64),
                                ("fp", fp as f64),
                                ("fn_", fn_ as f64),
                            ]),
                            est,
                        ))
                    } else {
                        wald_exp_block(dod, se, conf, statistic, &l.key, est)
                    }
                }
                "kappa" => {
                    // The `kappa` Fleiss intermediates.
                    let p = kappa_parts(tp, fp, fn_, tn);
                    let (po, pe, k, se) = (p.po, p.pe, p.k, p.se);
                    match l.key.as_str() {
                        "est" => Ok(closed(
                            statistic,
                            "kappa",
                            finite(&[("po", po), ("pe", pe)]),
                            est,
                        )),
                        "lower" => Ok(closed(
                            statistic,
                            "wald_lower",
                            finite(&[("est", k), ("se", se), ("z", z(conf))]),
                            est,
                        )),
                        "upper" => Ok(closed(
                            statistic,
                            "wald_upper",
                            finite(&[("est", k), ("se", se), ("z", z(conf))]),
                            est,
                        )),
                        _ => Err(u()),
                    }
                }
                "entropy" => {
                    let prev = prop(d, n);
                    let post = match l.key.as_str() {
                        "positive" => prop(tp, tpos),
                        "negative" => prop(tn, tneg),
                        _ => return Err(u()),
                    };
                    Ok(closed(
                        statistic,
                        "entropy_reduction",
                        finite(&[("prev", prev), ("post", post)]),
                        est,
                    ))
                }
                _ => Err(u()),
            }
        }
        _ => Err(u()),
    }
}
