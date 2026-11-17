//! `random_numbers` — the seeded draw, explained as a method.
use super::*;

pub(crate) fn random_numbers(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    match segs {
        [f] if f.key == "values" => {
            let i = f.index.ok_or_else(u)?;
            let mut v = vals(&[
                ("seed", cnt(inputs, "seed") as f64),
                ("lower", inputs["lower"].as_f64().unwrap_or(0.0)),
                ("upper", inputs["upper"].as_f64().unwrap_or(0.0)),
                ("count", cnt(inputs, "count") as f64),
                ("index", i as f64),
            ]);
            v.insert(
                "unique".into(),
                if inputs["unique"].as_str() == Some("yes") {
                    1.0
                } else {
                    0.0
                },
            );
            v.insert(
                "sorted".into(),
                if inputs["sorted"].as_str() == Some("yes") {
                    1.0
                } else {
                    0.0
                },
            );
            Ok(method(statistic, "method.random_numbers", v, est))
        }
        _ => Err(u()),
    }
}
