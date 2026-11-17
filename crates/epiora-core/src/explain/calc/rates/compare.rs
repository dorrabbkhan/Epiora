//! `rate_compare` — crude/strata/adjusted routing.
use super::adjusted::adjusted_leaf;
use super::leaf_pt::pt_block_leaf;
use super::*;
use crate::calc::counts::shared::num;
use crate::calc::person_time::rate_compare as rc;

pub(crate) fn rate_compare(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let strata = rc::strata(inputs);
    let pu = num(inputs, "per_unit");
    let conf = conf(inputs);

    match segs {
        [f, rest @ ..] if f.key == "adjusted" => {
            let rows: Vec<[f64; 4]> = strata.iter().map(|t| [t.a, t.b, t.n1, t.n0]).collect();
            adjusted_leaf(&rows, conf, statistic, rest, est)
        }
        [f, rest @ ..] => {
            let t = if f.key == "crude" {
                strata.iter().fold(
                    crate::calc::person_time::martin_pt::PtStratum {
                        a: 0.0,
                        b: 0.0,
                        n1: 0.0,
                        n0: 0.0,
                    },
                    |m, t| crate::calc::person_time::martin_pt::PtStratum {
                        a: m.a + t.a,
                        b: m.b + t.b,
                        n1: m.n1 + t.n1,
                        n0: m.n0 + t.n0,
                    },
                )
            } else if f.key == "strata" {
                let i = f.index.ok_or_else(u)?;
                let src = strata.get(i).ok_or_else(u)?;
                crate::calc::person_time::martin_pt::PtStratum {
                    a: src.a,
                    b: src.b,
                    n1: src.n1,
                    n0: src.n0,
                }
            } else {
                return Err(u());
            };
            pt_block_leaf(&t, pu, conf, statistic, rest, est)
        }
        _ => Err(u()),
    }
}
