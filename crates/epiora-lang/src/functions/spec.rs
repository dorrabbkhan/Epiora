//! The static function table: every `supported@1.1` function in
//! `register.json` with its arity, per-argument [`ArgKind`] and
//! result `ValueType` — the surface `type_check` checks against and
//! `call` dispatches on.

use epiora_core::dataset::ValueType;
use std::ops::RangeInclusive;

/// The static operand kind a function argument needs (the checker;
/// the eval coercion lives in each implementation).
#[derive(Clone, Copy, Debug)]
pub(crate) enum ArgKind {
    /// `ToString()` + `TryParse` — accepts `Integer`/`Real`/`Text`.
    Num,
    /// `is DateTime` — `Date` or `DateTime`.
    DateT,
    /// `is DateTime` or `is TimeSpan` — `Date`, `DateTime`, `Time`.
    DateTimeT,
    /// A bare variable (`LAGVALUE`/`GROUPROWINDEX` read the preceding
    /// row; anything else is an `InvalidCastException` in EI7).
    Ident,
    /// Any operand (`ToString()`-style consumers accept it).
    Any,
}

impl ArgKind {
    /// Human label for `expr_type` messages.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Num => "a number",
            Self::DateT => "a date",
            Self::DateTimeT => "a date or time",
            Self::Ident => "a variable",
            Self::Any => "any value",
        }
    }
}

/// A function's static surface: arity, per-argument kind, result type.
pub(crate) struct FnSpec {
    /// The upper-case name.
    pub name: &'static str,
    /// Accepted argument counts.
    pub arity: RangeInclusive<usize>,
    /// The kind of each position (as many as `arity.end()`).
    pub args: &'static [ArgKind],
    /// The `ValueType` `type_check` reports.
    pub result: ValueType,
}

impl FnSpec {
    /// Arity spelled for an error message.
    pub fn arity_label(&self) -> String {
        let (lo, hi) = (*self.arity.start(), *self.arity.end());
        if lo == hi {
            format!("{lo}")
        } else {
            format!("{lo}–{hi}")
        }
    }
}

use ArgKind::{Any, DateT, DateTimeT, Ident, Num};
use ValueType::{Boolean, Date, DateTime, Integer, Real, Text, Time};

const NUM: &[ArgKind] = &[Num];
const NUM2: &[ArgKind] = &[Num, Num];
const ANY: &[ArgKind] = &[Any];
const DT: &[ArgKind] = &[DateT];
const DT2: &[ArgKind] = &[DateT, DateT];

/// The recognized set — EI7's `EpiFunctions` dispatch plus
/// `LAGVALUE`/`GROUPROWINDEX`/`ZSCORE`/`PFROMZ`, every one `supported`
/// at `1.1` in `register.json`.
static FUNCTIONS: &[FnSpec] = &[
    FnSpec {
        name: "ABS",
        arity: 1..=1,
        args: NUM,
        result: Real,
    },
    FnSpec {
        name: "COS",
        arity: 1..=1,
        args: NUM,
        result: Real,
    },
    FnSpec {
        name: "DAY",
        arity: 1..=1,
        args: DT,
        result: Integer,
    },
    FnSpec {
        name: "DAYS",
        arity: 2..=2,
        args: DT2,
        result: Integer,
    },
    FnSpec {
        name: "ENVIRON",
        arity: 1..=1,
        args: ANY,
        result: Text,
    },
    FnSpec {
        name: "EPIWEEK",
        arity: 1..=2,
        args: &[DateT, Num],
        result: Integer,
    },
    FnSpec {
        name: "EXISTS",
        arity: 1..=1,
        args: ANY,
        result: Boolean,
    },
    FnSpec {
        name: "EXP",
        arity: 1..=1,
        args: NUM,
        result: Real,
    },
    FnSpec {
        name: "FILEDATE",
        arity: 1..=1,
        args: ANY,
        result: DateTime,
    },
    FnSpec {
        name: "FINDTEXT",
        arity: 2..=2,
        args: &[Any, Any],
        result: Integer,
    },
    FnSpec {
        name: "FORMAT",
        arity: 1..=2,
        args: &[Any, Any],
        result: Text,
    },
    FnSpec {
        name: "GROUPROWINDEX",
        arity: 1..=1,
        args: &[Ident],
        result: Integer,
    },
    FnSpec {
        name: "HOUR",
        arity: 1..=1,
        args: &[DateTimeT],
        result: Integer,
    },
    FnSpec {
        name: "HOURS",
        arity: 2..=2,
        args: DT2,
        result: Real,
    },
    FnSpec {
        name: "LAGVALUE",
        arity: 1..=1,
        args: &[Ident],
        result: Text,
    },
    FnSpec {
        name: "LN",
        arity: 1..=1,
        args: NUM,
        result: Real,
    },
    FnSpec {
        name: "LOG",
        arity: 1..=1,
        args: NUM,
        result: Real,
    },
    FnSpec {
        name: "MINUTE",
        arity: 1..=1,
        args: DT,
        result: Integer,
    },
    FnSpec {
        name: "MINUTES",
        arity: 2..=2,
        args: DT2,
        result: Real,
    },
    FnSpec {
        name: "MONTH",
        arity: 1..=1,
        args: DT,
        result: Integer,
    },
    FnSpec {
        name: "MONTHS",
        arity: 2..=2,
        args: DT2,
        result: Integer,
    },
    FnSpec {
        name: "NUMTODATE",
        arity: 3..=3,
        args: &[Num, Num, Num],
        result: Date,
    },
    FnSpec {
        name: "NUMTOTIME",
        arity: 3..=3,
        args: &[Num, Num, Num],
        result: Time,
    },
    FnSpec {
        name: "PFROMZ",
        arity: 1..=1,
        args: ANY,
        result: Real,
    },
    FnSpec {
        name: "POISSONLCL",
        arity: 1..=2,
        args: &[Num, Any],
        result: Real,
    },
    FnSpec {
        name: "POISSONUCL",
        arity: 1..=2,
        args: &[Num, Any],
        result: Real,
    },
    FnSpec {
        name: "RECORDCOUNT",
        arity: 0..=0,
        args: &[],
        result: Integer,
    },
    FnSpec {
        name: "RND",
        arity: 1..=2,
        args: NUM2,
        result: Integer,
    },
    FnSpec {
        name: "ROUND",
        arity: 1..=2,
        args: NUM2,
        result: Real,
    },
    FnSpec {
        name: "SECOND",
        arity: 1..=1,
        args: DT,
        result: Integer,
    },
    FnSpec {
        name: "SECONDS",
        arity: 2..=2,
        args: DT2,
        result: Real,
    },
    FnSpec {
        name: "SIN",
        arity: 1..=1,
        args: NUM,
        result: Real,
    },
    FnSpec {
        name: "SQRT",
        arity: 1..=1,
        args: NUM,
        result: Real,
    },
    FnSpec {
        name: "STEP",
        arity: 2..=2,
        args: NUM2,
        result: Real,
    },
    FnSpec {
        name: "STRLEN",
        arity: 1..=1,
        args: ANY,
        result: Integer,
    },
    FnSpec {
        name: "SUBSTRING",
        arity: 2..=3,
        args: &[Any, Num, Num],
        result: Text,
    },
    FnSpec {
        name: "SYSTEMDATE",
        arity: 0..=0,
        args: &[],
        result: Date,
    },
    FnSpec {
        name: "SYSTEMTIME",
        arity: 0..=0,
        args: &[],
        result: Time,
    },
    FnSpec {
        name: "TAN",
        arity: 1..=1,
        args: NUM,
        result: Real,
    },
    FnSpec {
        name: "TRUNC",
        arity: 1..=1,
        args: NUM,
        result: Real,
    },
    FnSpec {
        name: "TXTTODATE",
        arity: 1..=1,
        args: ANY,
        result: DateTime,
    },
    FnSpec {
        name: "TXTTONUM",
        arity: 1..=1,
        args: ANY,
        result: Real,
    },
    FnSpec {
        name: "UPPERCASE",
        arity: 1..=1,
        args: ANY,
        result: Text,
    },
    FnSpec {
        name: "YEAR",
        arity: 1..=1,
        args: DT,
        result: Integer,
    },
    FnSpec {
        name: "YEARS",
        arity: 2..=2,
        args: DT2,
        result: Integer,
    },
    FnSpec {
        name: "ZSCORE",
        arity: 1..=5,
        args: &[Any, Any, Any, Any, Any],
        result: Real,
    },
];

/// The `FnSpec` for `name` (upper-cased by the parser), or `None` —
/// EI7 reports the same failure as "not a recognized function" at
/// parse time.
pub(crate) fn function_spec(name: &str) -> Option<&'static FnSpec> {
    FUNCTIONS.iter().find(|f| f.name == name)
}

/// The arg kinds a `type_check` compares against, one per given arg.
pub(crate) fn arg_kind(spec: &FnSpec, n_args: usize) -> &[ArgKind] {
    &spec.args[..n_args.min(spec.args.len())]
}
