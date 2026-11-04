//! The discrete distributions of spec §3.4: `dhyper`, `pbinom` and
//! the non-central hypergeometric behind R's `fisher.test`
//! (`dnhyper`/`mnhyper`/`pnhyper`). `dhyper` is ported from R 4.6.x
//! `src/nmath/dhyper.c`, which composes `dbinom_raw`
//! (`src/nmath/dbinom.c`, with `pow1p`) on `stirlerr`/`bd0` — the only
//! way `m = n = 2e6` meets `closed_form`; plain `lgamma` differences
//! lose ~7e-9 there.

use super::gamma::{bd0, stirlerr, M_LN_2PI};

/// `give_log` flavour of R's `R_D__0`/`R_D__1` boundary returns.
fn rd(ok: bool, give_log: bool) -> f64 {
    if give_log {
        if ok {
            0.0
        } else {
            f64::NEG_INFINITY
        }
    } else if ok {
        1.0
    } else {
        0.0
    }
}

/// `v − max < UNDERFLOW` ⇒ `libm::exp(v − max)` is exactly `+0.0`, so
/// the term contributes `0.0` and the `exp` call can be skipped — every
/// sum stays bit-identical, nothing is approximated or truncated.
/// libm 0.2.16 `src/math/exp.rs` returns `0.` for every
/// `x < -745.13321910194110842` (the `if x < -745.13321910194110842`
/// early return in `exp`, fdlibm's `u_threshold`; the constant below
/// is the same `f64`, spelled without digits `f64` cannot represent).
const UNDERFLOW: f64 = -745.133_219_101_941_1;

/// `(1+x)^y` accurately for `|x| << 1` — R `src/nmath/dbinom.c`'s
/// `pow1p`, ported verbatim.
fn pow1p(x: f64, y: f64) -> f64 {
    if y.is_nan() {
        return if x == 0.0 { 1.0 } else { y };
    }
    if y >= 0.0 && y == libm::trunc(y) && y <= 4.0 {
        match y as i32 {
            0 => return 1.0,
            1 => return x + 1.0,
            2 => return x * (x + 2.0) + 1.0,
            3 => return x * (x * (x + 3.0) + 3.0) + 1.0,
            _ => return x * (x * (x * (x + 4.0) + 6.0) + 4.0) + 1.0,
        }
    }
    let xp1 = x + 1.0;
    if xp1 - 1.0 == x || x.abs() > 0.5 || x.is_nan() {
        libm::pow(xp1, y)
    } else {
        libm::exp(y * libm::log1p(x))
    }
}

/// `dbinom_raw(x, n, p, q, give_log)` — R `src/nmath/dbinom.c`
/// verbatim (`stirlerr` + `bd0`; `n` is integral in every call here).
fn dbinom_raw(x: f64, n: f64, p: f64, q: f64, give_log: bool) -> f64 {
    if p == 0.0 {
        return rd(x == 0.0, give_log);
    }
    if q == 0.0 {
        return rd(x == n, give_log);
    }
    if x == 0.0 {
        if n == 0.0 {
            return rd(true, give_log);
        }
        return if p > q {
            if give_log {
                n * libm::log(q)
            } else {
                libm::pow(q, n)
            }
        } else if give_log {
            n * libm::log1p(-p)
        } else {
            pow1p(-p, n)
        };
    }
    if x == n {
        return if p > q {
            if give_log {
                n * libm::log1p(-q)
            } else {
                pow1p(-q, n)
            }
        } else if give_log {
            n * libm::log(p)
        } else {
            libm::pow(p, n)
        };
    }
    if x < 0.0 || x > n {
        return rd(false, give_log);
    }
    let lc = stirlerr(n) - stirlerr(x) - stirlerr(n - x) - bd0(x, n * p) - bd0(n - x, n * q);
    // log(2*pi*x*(n-x)/n) written so x << n stays accurate
    let lf = M_LN_2PI + libm::log(x) + libm::log1p(-x / n);
    if give_log {
        lc - 0.5 * lf
    } else {
        libm::exp(lc - 0.5 * lf)
    }
}

/// `dhyper(x, m, n, k)` — R's `dhyper` layout: `p = k/(m+n)`,
/// `pmf = dbinom_raw(x,m,·)·dbinom_raw(k−x,n,·)/dbinom_raw(k,m+n,·)`.
fn dhyper(x: f64, m: f64, n: f64, k: f64, give_log: bool) -> f64 {
    let p = k / (m + n);
    let q = (m + n - k) / (m + n);
    let p1 = dbinom_raw(x, m, p, q, give_log);
    let p2 = dbinom_raw(k - x, n, p, q, give_log);
    let p3 = dbinom_raw(k, m + n, p, q, give_log);
    if give_log {
        p1 + p2 - p3
    } else {
        p1 * p2 / p3
    }
}

/// `dhyper(x, m, n, k)` — pmf of drawing exactly `x` of the `m`
/// marked items in `k` draws from `m + n` (spec §3.4). Zero outside
/// the support `max(0, k−n) ..= min(k, m)`.
pub fn hypergeom_pmf(x: u64, m: u64, n: u64, k: u64) -> f64 {
    if k < x || m < x || k - x > n {
        return 0.0;
    }
    if k == 0 {
        return if x == 0 { 1.0 } else { 0.0 };
    }
    dhyper(x as f64, m as f64, n as f64, k as f64, false)
}

/// `pbinom(x, n, p, lower.tail)`: `P(X ≤ x)` for `lower`, `P(X > x)`
/// otherwise — summed in log space (spec §3.4) via `dbinom_raw`'s
/// log density, rescaled by the largest log term.
pub fn binomial_tail(x: u64, n: u64, p: f64, lower: bool) -> f64 {
    if p.is_nan() {
        return f64::NAN;
    }
    if p <= 0.0 {
        // all mass at 0
        return if lower { 1.0 } else { 0.0 };
    }
    if p >= 1.0 {
        // all mass at n
        return if lower {
            f64::from(x >= n)
        } else {
            f64::from(x < n)
        };
    }
    let (from, to) = if lower {
        (0u64, x.min(n))
    } else {
        (x.saturating_add(1), n)
    };
    if from > to {
        return 0.0;
    }
    let q = 1.0 - p;
    let nf = n as f64;
    let mut max = f64::NEG_INFINITY;
    let mut sum = 0.0;
    for i in from..=to {
        let l = dbinom_raw(i as f64, nf, p, q, true);
        if l > max {
            if max.is_finite() {
                sum *= libm::exp(max - l);
            }
            max = l;
        }
        sum += libm::exp(l - max);
    }
    if !max.is_finite() {
        return 0.0;
    }
    libm::exp(max) * sum
}

/// The non-central hypergeometric of `fisher.test`: `X | margins` is
/// the `[1,1]` cell of a 2×2 table with column totals `m`, `n` and
/// row-1 total `k`. Stores log `dhyper` over the support
/// `lo..=hi` and normalises in log space with the max subtracted —
/// exactly `fisher.test`'s `dnhyper`/`mnhyper`/`pnhyper`
/// (fixtures/R/nchyper.R).
pub struct NcHypergeom {
    lo: u64,
    hi: u64,
    logdc: Vec<f64>,
}

impl NcHypergeom {
    /// `lo = max(0, k−n)`, `hi = min(k, m)`.
    pub fn new(m: u64, n: u64, k: u64) -> NcHypergeom {
        let lo = k.saturating_sub(n);
        let hi = k.min(m);
        let (mf, nf, kf) = (m as f64, n as f64, k as f64);
        let logdc = if lo <= hi {
            (lo..=hi)
                .map(|x| dhyper(x as f64, mf, nf, kf, true))
                .collect()
        } else {
            Vec::new()
        };
        NcHypergeom { lo, hi, logdc }
    }

    /// `(lo, hi)` — the support.
    pub fn support(&self) -> (u64, u64) {
        (self.lo, self.hi)
    }

    /// `dnhyper(ncp)`: `exp(logdc + log(psi)·support − max) / sum`.
    fn weights(&self, psi: f64) -> Vec<f64> {
        self.weights_counted(psi).0
    }

    /// `weights` plus the number of `libm::exp` evaluations it makes,
    /// read by `tests/exact_window.rs` through `weights_probe`.
    fn weights_counted(&self, psi: f64) -> (Vec<f64>, usize) {
        let lpsi = libm::log(psi);
        let mut max = f64::NEG_INFINITY;
        let mut d = Vec::with_capacity(self.logdc.len());
        for (i, &lc) in self.logdc.iter().enumerate() {
            let v = lc + lpsi * (self.lo + i as u64) as f64;
            if v > max {
                max = v;
            }
            d.push(v);
        }
        let mut evals = 0_usize;
        let mut sum = 0.0;
        for v in &mut d {
            // `exp` returns exactly `0.0` below UNDERFLOW (above); the
            // left-to-right sum order of the evaluated terms is
            // unchanged, so the result is bit-identical.
            *v = if *v - max < UNDERFLOW {
                0.0
            } else {
                evals += 1;
                libm::exp(*v - max)
            };
            sum += *v;
        }
        for v in &mut d {
            *v /= sum;
        }
        (d, evals)
    }

    /// The full-support `weights` of the pre-Task-11a algorithm,
    /// verbatim — the bit-identity reference `weights_probe` returns.
    fn weights_full(&self, psi: f64) -> Vec<f64> {
        let lpsi = libm::log(psi);
        let mut max = f64::NEG_INFINITY;
        let mut d = Vec::with_capacity(self.logdc.len());
        for (i, &lc) in self.logdc.iter().enumerate() {
            let v = lc + lpsi * (self.lo + i as u64) as f64;
            if v > max {
                max = v;
            }
            d.push(v);
        }
        let mut sum = 0.0;
        for v in &mut d {
            *v = libm::exp(*v - max);
            sum += *v;
        }
        for v in &mut d {
            *v /= sum;
        }
        d
    }

    /// Test-only probe (`tests/exact_window.rs`, Task 11a): the
    /// pre-change full-support `weights` — the bit-identity reference —
    /// and the number of `libm::exp` evaluations `weights` makes at
    /// `psi`. `#[doc(hidden)]`, not public API.
    #[doc(hidden)]
    pub fn weights_probe(&self, psi: f64) -> (Vec<f64>, usize) {
        (self.weights_full(psi), self.weights_counted(psi).1)
    }

    /// `P(X = x | psi)` — `dnhyper` at `x`.
    pub fn pmf(&self, x: u64, psi: f64) -> f64 {
        if x < self.lo || x > self.hi {
            return 0.0;
        }
        if psi == 0.0 {
            return f64::from(x == self.lo);
        }
        if psi == f64::INFINITY {
            return f64::from(x == self.hi);
        }
        self.weights(psi)[(x - self.lo) as usize]
    }

    /// `mnhyper(psi)`: `E[X]` — `lo` for psi = 0, `hi` for psi = ∞.
    pub fn mean(&self, psi: f64) -> f64 {
        if psi == 0.0 {
            return self.lo as f64;
        }
        if psi == f64::INFINITY {
            return self.hi as f64;
        }
        self.weights(psi)
            .iter()
            .enumerate()
            .map(|(i, &w)| (self.lo + i as u64) as f64 * w)
            .sum()
    }

    /// `pnhyper(x, psi)` — `P(X ≤ x)`.
    pub fn lower_tail(&self, x: u64, psi: f64) -> f64 {
        if psi == 0.0 {
            return f64::from(x >= self.lo);
        }
        if psi == f64::INFINITY {
            return f64::from(x >= self.hi);
        }
        self.tail_sum(x, psi, false)
    }

    /// `pnhyper(x, psi, upper.tail = TRUE)` — `P(X ≥ x)`.
    pub fn upper_tail(&self, x: u64, psi: f64) -> f64 {
        if psi == 0.0 {
            return f64::from(x <= self.lo);
        }
        if psi == f64::INFINITY {
            return f64::from(x <= self.hi);
        }
        self.tail_sum(x, psi, true)
    }

    fn tail_sum(&self, x: u64, psi: f64, upper: bool) -> f64 {
        self.weights(psi)
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let xi = self.lo + *i as u64;
                if upper {
                    xi >= x
                } else {
                    xi <= x
                }
            })
            .map(|(_, &w)| w)
            .sum()
    }
}
