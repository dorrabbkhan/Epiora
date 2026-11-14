//! The `bgrat` half of R's `bratio` (toms708.c): the
//! Didonato–Morris asymptotic expansion `grat_accum`/`grat_r` plus the
//! shared special-function helpers (`gam1`, `gamln1`, `rexpm1`,
//! `digamma`, `erf__`) the `beta_small` series need. Ported verbatim
//! like the rest of the engine's R-derived special functions.

use super::beta_math::{algdiv, erfcx};

pub(crate) const EPS: f64 = 1e-15;
pub(crate) const ML_NEGINF: f64 = f64::NEG_INFINITY;
const M_SQRT_PI: f64 = 1.77245385090552; // sqrt(pi)

/// `1/Γ(a+1) − 1` for `-0.5 <= a <= 1.5` — `gam1`.
pub(crate) fn gam1(a: f64) -> f64 {
    let d = a - 0.5;
    let t = if d > 0.0 { d - 0.5 } else { a };
    if t < 0.0 {
        const R: [f64; 9] = [
            -0.422784335098468,
            -0.771330383816272,
            -0.244757765222226,
            0.118378989872749,
            9.30357293360349e-4,
            -0.0118290993445146,
            0.00223047661158249,
            2.66505979058923e-4,
            -1.32674909766242e-4,
        ];
        const S1: f64 = 0.273076135303957;
        const S2: f64 = 0.0559398236957378;
        let top = (((((((R[8] * t + R[7]) * t + R[6]) * t + R[5]) * t + R[4]) * t + R[3]) * t
            + R[2])
            * t
            + R[1])
            * t
            + R[0];
        let bot = (S2 * t + S1) * t + 1.0;
        let w = top / bot;
        if d > 0.0 {
            t * w / a
        } else {
            a * (w + 0.5 + 0.5)
        }
    } else if t == 0.0 {
        0.0
    } else {
        const P: [f64; 7] = [
            std::f64::consts::EULER_GAMMA,
            -0.409078193005776,
            -0.230975380857675,
            0.0597275330452234,
            0.0076696818164949,
            -0.00514889771323592,
            5.89597428611429e-4,
        ];
        const Q: [f64; 5] = [
            1.0,
            0.427569613095214,
            0.158451672430138,
            0.0261132021441447,
            0.00423244297896961,
        ];
        let top = (((((P[6] * t + P[5]) * t + P[4]) * t + P[3]) * t + P[2]) * t + P[1]) * t + P[0];
        let bot = (((Q[4] * t + Q[3]) * t + Q[2]) * t + Q[1]) * t + 1.0;
        let w = top / bot;
        if d > 0.0 {
            t / a * (w - 0.5 - 0.5)
        } else {
            a * w
        }
    }
}

/// `ln Γ(1+a)` for `-0.2 <= a <= 1.25` — `gamln1`.
pub(crate) fn gamln1(a: f64) -> f64 {
    if a < 0.6 {
        const P: [f64; 7] = [
            std::f64::consts::EULER_GAMMA,
            0.844203922187225,
            -0.168860593646662,
            -0.780427615533591,
            -0.402055799310489,
            -0.0673562214325671,
            -0.00271935708322958,
        ];
        const Q: [f64; 6] = [
            2.88743195473681,
            3.12755088914843,
            1.56875193295039,
            0.361951990101499,
            0.0325038868253937,
            6.67465618796164e-4,
        ];
        let w = ((((((P[6] * a + P[5]) * a + P[4]) * a + P[3]) * a + P[2]) * a + P[1]) * a + P[0])
            / ((((((Q[5] * a + Q[4]) * a + Q[3]) * a + Q[2]) * a + Q[1]) * a + Q[0]) * a + 1.0);
        -a * w
    } else {
        const R: [f64; 6] = [
            0.422784335098467,
            0.848044614534529,
            0.565221050691933,
            0.156513060486551,
            0.017050248402265,
            4.97958207639485e-4,
        ];
        const S: [f64; 5] = [
            1.24313399877507,
            0.548042109832463,
            0.10155218743983,
            0.00713309612391,
            1.16165475989616e-4,
        ];
        let x = a - 0.5 - 0.5;
        let w = (((((R[5] * x + R[4]) * x + R[3]) * x + R[2]) * x + R[1]) * x + R[0])
            / (((((S[4] * x + S[3]) * x + S[2]) * x + S[1]) * x + S[0]) * x + 1.0);
        x * w
    }
}

/// `exp(x) − 1` — `rexpm1`.
fn rexpm1(x: f64) -> f64 {
    const P1: f64 = 9.14041914819518e-10;
    const P2: f64 = 0.0238082361044469;
    const Q1: f64 = -0.499999999085958;
    const Q2: f64 = 0.107141568980644;
    const Q3: f64 = -0.0119041179760821;
    const Q4: f64 = 5.95130811860248e-4;
    if x.abs() <= 0.15 {
        x * (((P2 * x + P1) * x + 1.0) / ((((Q4 * x + Q3) * x + Q2) * x + Q1) * x + 1.0))
    } else {
        let w = libm::exp(x);
        if x > 0.0 {
            w * (0.5 - 1.0 / w + 0.5)
        } else {
            w - 0.5 - 0.5
        }
    }
}

/// Digamma `ψ(x)` for `x > 0` — recurrence up to 10 then the
/// asymptotic expansion; only called from `apser` (b ≤ 2e13).
pub(crate) fn digamma(x: f64) -> f64 {
    let mut x = x;
    let mut s = 0.0;
    while x < 10.0 {
        s -= 1.0 / x;
        x += 1.0;
    }
    let r = 1.0 / x;
    let r2 = r * r;
    // psi(x) ~ ln x − 1/(2x) − Σ B(2k)/(2k·x^{2k})
    s + libm::log(x)
        - 0.5 * r
        - r2 * (1.0 / 12.0 - r2 * (1.0 / 120.0 - r2 * (1.0 / 252.0 - r2 / 240.0)))
}

/// `erf(x)` — `erf__` (same rational fits as `erfc1`).
pub(crate) fn erf__(x: f64) -> f64 {
    const C: f64 = 0.564189583547756;
    const A: [f64; 5] = [
        7.7105849500132e-5,
        -0.00133733772997339,
        0.0323076579225834,
        0.0479137145607681,
        0.128379167095513,
    ];
    const B: [f64; 3] = [0.00301048631703895, 0.0538971687740286, 0.375795757275549];
    const P: [f64; 8] = [
        -1.36864857382717e-7,
        0.564195517478974,
        7.21175825088309,
        43.1622272220567,
        152.98928504694,
        339.320816734344,
        451.918953711873,
        300.459261020162,
    ];
    const Q: [f64; 8] = [
        1.0,
        12.7827273196294,
        77.0001529352295,
        277.585444743988,
        638.980264465631,
        931.35409485061,
        790.950925327898,
        300.459260956983,
    ];
    const R: [f64; 5] = [
        2.10144126479064,
        26.2370141675169,
        21.3688200555087,
        4.6580782871847,
        0.282094791773523,
    ];
    const S: [f64; 4] = [
        94.153775055546,
        187.11481179959,
        99.0191814623914,
        18.0124575948747,
    ];
    let ax = x.abs();
    if ax <= 0.5 {
        let t = x * x;
        let top = (((A[0] * t + A[1]) * t + A[2]) * t + A[3]) * t + A[4] + 1.0;
        let bot = ((B[0] * t + B[1]) * t + B[2]) * t + 1.0;
        return x * (top / bot);
    }
    if ax <= 4.0 {
        let top = ((((((P[0] * ax + P[1]) * ax + P[2]) * ax + P[3]) * ax + P[4]) * ax + P[5]) * ax
            + P[6])
            * ax
            + P[7];
        let bot = ((((((Q[0] * ax + Q[1]) * ax + Q[2]) * ax + Q[3]) * ax + Q[4]) * ax + Q[5]) * ax
            + Q[6])
            * ax
            + Q[7];
        let r = 0.5 - libm::exp(-x * x) * top / bot + 0.5;
        return if x < 0.0 { -r } else { r };
    }
    if ax >= 5.8 {
        return if x > 0.0 { 1.0 } else { -1.0 };
    }
    let x2 = x * x;
    let t = 1.0 / x2;
    let top = (((R[0] * t + R[1]) * t + R[2]) * t + R[3]) * t + R[4];
    let bot = (((S[0] * t + S[1]) * t + S[2]) * t + S[3]) * t + 1.0;
    let t2 = (C - top / (x2 * bot)) / ax;
    let r = 0.5 - libm::exp(-x2) * t2 + 0.5;
    if x < 0.0 {
        -r
    } else {
        r
    }
}

/// `w += I_x(a,b)` by the Didonato–Morris asymptotic expansion —
/// `bgrat` for `a >= 15`, `b <= 1`, non-log `w`. `w_in` is the value
/// accumulated so far (0 for a fresh sum, matching `*w = 0` in R's
/// `w1 := 0` before `bgrat`).
pub(crate) fn grat_accum(a: f64, b: f64, x: f64, y: f64, w_in: f64) -> f64 {
    const N_TERMS: usize = 30;
    let mut c = [0.0f64; N_TERMS];
    let mut d = [0.0f64; N_TERMS];
    let bm1 = b - 0.5 - 0.5;
    let nu = a + bm1 * 0.5;
    let lnx = if y > 0.375 {
        libm::log(x)
    } else {
        libm::log1p(-y)
    };
    let z = -nu * lnx;
    if b * z == 0.0 {
        return w_in; // ierr 1: expansion cannot be computed
    }
    // log(r) = log(b) + log1p(gam1(b)) + b·log(z) + nu·lnx
    let log_r = libm::log(b) + libm::log1p(gam1(b)) + b * libm::log(z) + nu * lnx;
    let log_u = log_r - (algdiv(b, a) + b * libm::log(nu));
    if log_u == ML_NEGINF {
        return w_in; // ierr 2
    }
    let u = libm::exp(log_u);
    let u_0 = u == 0.0;
    // l := w/u, careful when u underflowed to 0
    let l = if w_in == 0.0 {
        0.0
    } else {
        libm::exp(libm::log(w_in) - log_u)
    };
    let q_r = grat_r(b, z, log_r, EPS * 15.0);
    let v = 0.25 / (nu * nu);
    let t2 = lnx * 0.25 * lnx;
    let mut j = q_r;
    let mut sum = j;
    let (mut t, mut cn, mut n2) = (1.0f64, 1.0, 0.0);
    for n in 1..=N_TERMS {
        let bp2n = b + n2;
        j = (bp2n * (bp2n + 1.0) * j + (z + bp2n + 1.0) * t) * v;
        n2 += 2.0;
        t *= t2;
        cn /= n2 * (n2 + 1.0);
        let nm1 = n - 1;
        c[nm1] = cn;
        let mut s = 0.0;
        if n > 1 {
            let mut coef = b - n as f64;
            for i in 1..=nm1 {
                s += coef * c[i - 1] * d[nm1 - i];
                coef += b;
            }
        }
        d[nm1] = bm1 * cn + s / n as f64;
        let dj = d[nm1] * j;
        sum += dj;
        if sum <= 0.0 {
            return w_in; // ierr 3
        }
        if dj.abs() <= EPS * 15.0 * (sum + l) {
            break;
        }
        // n == N_TERMS without convergence: R warns (ierr 4) and
        // still uses the partial sum.
    }
    if u_0 {
        w_in + libm::exp(log_u + libm::log(sum))
    } else {
        w_in + u * sum
    }
}

/// `Q(a,x)/r` — `grat_r`, the scaled complement of the incomplete
/// gamma ratio for `a <= 1`.
fn grat_r(a: f64, x: f64, log_r: f64, eps: f64) -> f64 {
    if a * x == 0.0 {
        return if x <= a { libm::exp(-log_r) } else { 0.0 };
    }
    if a == 0.5 {
        // e.g. when called via pt()
        if x < 0.25 {
            let p = erf__(libm::sqrt(x));
            return (0.5 - p + 0.5) * libm::exp(-log_r);
        }
        let sx = libm::sqrt(x);
        return erfcx(sx) / sx * M_SQRT_PI;
    }
    if x < 1.1 {
        // Taylor series for P(a,x)/x^a
        let mut an = 3.0;
        let mut c = x;
        let mut sum = x / (a + 3.0);
        let tol = eps * 0.1 / (a + 1.0);
        loop {
            an += 1.0;
            c *= -(x / an);
            let t = c / (a + an);
            sum += t;
            if t.abs() <= tol || an > 1e7 {
                break;
            }
        }
        let j = a * x * ((sum / 6.0 - 0.5 / (a + 2.0)) * x + 1.0 / (a + 1.0));
        let z = a * libm::log(x);
        let h = gam1(a);
        let g = h + 1.0;
        if (x >= 0.25 && a < x / 2.59) || z > -0.13394 {
            let l = rexpm1(z);
            let q = ((l + 0.5 + 0.5) * j - l) * g - h;
            if q <= 0.0 {
                0.0
            } else {
                q * libm::exp(-log_r)
            }
        } else {
            let p = libm::exp(z) * g * (0.5 - j + 0.5);
            (0.5 - p + 0.5) * libm::exp(-log_r)
        }
    } else {
        // x >= 1.1: continued fraction expansion
        let (mut a2n_1, mut a2n) = (1.0f64, 1.0);
        let (mut b2n_1, mut b2n) = (x, x + (1.0 - a));
        let mut c = 1.0;
        let mut iters = 0;
        loop {
            a2n_1 = x * a2n + c * a2n_1;
            b2n_1 = x * b2n + c * b2n_1;
            let am0 = a2n_1 / b2n_1;
            c += 1.0;
            let c_a = c - a;
            a2n = a2n_1 + c_a * a2n;
            b2n = b2n_1 + c_a * b2n;
            let an0 = a2n / b2n;
            iters += 1;
            if (an0 - am0).abs() < eps * an0 || iters > 1_000_000 {
                break an0;
            }
        }
    }
}
