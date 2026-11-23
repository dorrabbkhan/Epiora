//! Value-label sets (v1.1 spec §4.3): an Epi Info code table or
//! Legal-Values/Comment-Legal/Option field maps stored values to the
//! text a result shows beside them. Entries keep the source order —
//! Epi Info lists them that way — and the first entry wins on a
//! duplicated value. Label sets are part of the dataset hash (the
//! `LBLS` section): a different label for the same value changes what
//! FREQ and TABLES display.

use super::Value;
use serde::{Deserialize, Serialize};

/// A named, ordered list of value → label pairs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct LabelSet {
    /// The set's id — the name `ColumnMeta::label_set` references.
    pub id: String,
    /// `(value, label)` pairs in source order.
    pub entries: Vec<(Value, String)>,
}

impl LabelSet {
    /// The label entry for `v`; the first matching entry wins on a
    /// duplicated value.
    pub fn label_for(&self, v: &Value) -> Option<&str> {
        self.entries.iter().find_map(|(value, label)| {
            if value_eq(value, v) {
                Some(label.as_str())
            } else {
                None
            }
        })
    }
}

/// Whether two values are the same for label lookup: `-0.0` equals
/// `0.0`, matching the canonical-bytes normalisation everywhere else.
fn value_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Real(x), Value::Real(y)) => {
            let norm = |v: &f64| if *v == 0.0 { 0.0 } else { *v };
            norm(x) == norm(y)
        }
        _ => a == b,
    }
}
