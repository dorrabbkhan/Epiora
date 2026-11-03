//! `normal_cdf`/`normal_sf` and `normal_quantile` (spec §3.4), ported
//! from R 4.6.x `src/nmath/pnorm.c` (`pnorm_both`: W. J. Cody's
//! rational Chebyshev approximations, Alg. 715) and
//! `src/nmath/qnorm.c` (Wichura AS 241 with the far-tail refinement).
//! Porting — rather than composing `erfc` — is what reproduces R's
//! denormal `pnorm(-38) = 2.88542835e-316` bit for bit (fixture
//! `normal_cdf`, `closed_form`).

const M_SQRT_32: f64 = 5.656854249492381;
const M_1_SQRT_2PI: f64 = 0.3989422804014327;
const M_SQRT2: f64 = std::f64::consts::SQRT_2;
const M_2PI: f64 = std::f64::consts::TAU;
const HALF_EPS: f64 = f64::EPSILON * 0.5;

const A: [f64; 5] = [
    2.2352520354606837,
    161.02823106855587,
    1067.6894854603709,
    18154.98125334356,
    0.06568233791820745,
];
const B: [f64; 4] = [
    47.202581904688245,
    976.0985517377767,
    10260.932208618979,
    45507.78933502673,
];
const C: [f64; 9] = [
    0.39894151208813466,
    8.883149794388377,
    93.50665613217785,
    597.2702763948002,
    2494.5375852903726,
    6848.190450536283,
    11602.65143764735,
    9842.714838383978,
    1.0765576773720192e-08,
];
const D: [f64; 8] = [
    22.266688044328117,
    235.387901782625,
    1519.3775994075547,
    6485.558298266761,
    18615.571640885097,
    34900.95272114598,
    38912.00328609327,
    19685.429676859992,
];
const P: [f64; 6] = [
    0.215898534057957,
    0.12740116116024736,
    0.022235277870649807,
    0.0014216191932278934,
    2.9112874951168793e-05,
    0.023073441764940174,
];
const Q: [f64; 5] = [
    1.284260096144911,
    0.4682382124808651,
    0.06598813786892856,
    0.0037823963320275824,
    7.297515550839662e-05,
];

/// `pnorm(x)` for `lower` = CDF, `lower` = false for the survival
/// function — R's `pnorm_both` restricted to the non-log case.
fn pnorm(x: f64, lower: bool) -> f64 {
    if x.is_nan() {
        return x;
    }
    let y = x.abs();
    if y <= 0.67448975 {
        // qnorm(3/4): central rational approximation
        let (mut xnum, mut xden) = (0.0, 0.0);
        if y > HALF_EPS {
            let xsq = x * x;
            xnum = A[4] * xsq;
            xden = xsq;
            for i in 0..3 {
                xnum = (xnum + A[i]) * xsq;
                xden = (xden + B[i]) * xsq;
            }
        }
        let temp = x * (xnum + A[3]) / (xden + B[3]);
        return if lower { 0.5 + temp } else { 0.5 - temp };
    }
    if y <= M_SQRT_32 {
        let mut xnum = C[8] * y;
        let mut xden = y;
        for i in 0..7 {
            xnum = (xnum + C[i]) * y;
            xden = (xden + D[i]) * y;
        }
        let temp = (xnum + C[7]) / (xden + D[7]);
        return do_del(y, temp, x, lower);
    }
    // |x| > sqrt(32): rational tail where R computes it, else 0/1
    let in_range = if lower {
        -38.4674 < x && x < 8.2924
    } else {
        -8.2924 < x && x < 38.4674
    };
    if !in_range {
        return if (x > 0.0) == lower { 1.0 } else { 0.0 };
    }
    let xsq = 1.0 / (x * x);
    let mut xnum = P[5] * xsq;
    let mut xden = xsq;
    for i in 0..4 {
        xnum = (xnum + P[i]) * xsq;
        xden = (xden + Q[i]) * xsq;
    }
    let temp = (M_1_SQRT_2PI - xsq * (xnum + P[4]) / (xden + Q[4])) / y;
    do_del(x, temp, x, lower)
}

/// `pnorm_both`'s `do_del` + `swap_tail`: splits off `v`'s multiple of
/// 1/16 so `exp(-v^2/2) = exp(-xsq^2/2)·exp(-del/2)` stays accurate,
/// then places the tail on the asked-for side.
fn do_del(v: f64, temp: f64, x: f64, lower: bool) -> f64 {
    let xsq = libm::trunc(v * 16.0) / 16.0;
    let del = (v - xsq) * (v + xsq);
    let tail = libm::exp(-xsq * xsq * 0.5) * libm::exp(-del * 0.5) * temp;
    if (x > 0.0) == lower {
        1.0 - tail
    } else {
        tail
    }
}

/// `pnorm(x)` — the standard-normal CDF (R `pnorm`, spec §3.4).
pub fn normal_cdf(x: f64) -> f64 {
    pnorm(x, true)
}

/// `pnorm(x, lower.tail = FALSE)` — the survival function.
pub fn normal_sf(x: f64) -> f64 {
    pnorm(x, false)
}

/// `qnorm(p)` — Wichura AS 241 as R implements it, including the
/// asymptotic tail refinement. `None` unless `0 < p < 1` (R returns
/// ±inf at the boundaries; the engine keeps them out, spec §3.4).
pub fn normal_quantile(p: f64) -> Option<f64> {
    if !(0.0 < p && p < 1.0) {
        return None;
    }
    let q = p - 0.5;
    let val = if q.abs() <= 0.425 {
        // 0.075 <= p <= 0.925
        let r = 0.180625 - q * q;
        q * horner(
            r,
            &[
                2509.0809287301227,
                33430.57558358813,
                67265.7709270087,
                45921.95393154987,
                13731.69376550946,
                1971.5909503065513,
                133.14166789178438,
                3.3871328727963665,
            ],
        ) / horner(
            r,
            &[
                5226.495278852854,
                28729.085735721943,
                39307.89580009271,
                21213.794301586597,
                5394.196021424751,
                687.1870074920579,
                42.31333070160091,
                1.0,
            ],
        )
    } else {
        // r = sqrt(-log(min(p, 1-p)))
        let lp = libm::log(if q > 0.0 { 1.0 - p } else { p });
        let mut r = libm::sqrt(-lp);
        let mut val = if r <= 5.0 {
            // min(p,1-p) >= exp(-25) ~= 1.3888e-11
            r -= 1.6;
            horner(
                r,
                &[
                    0.0007745450142783414,
                    0.022723844989269184,
                    0.2417807251774506,
                    1.2704582524523684,
                    3.6478483247632045,
                    5.769497221460691,
                    4.630337846156546,
                    1.4234371107496835,
                ],
            ) / horner(
                r,
                &[
                    1.0507500716444169e-09,
                    0.0005475938084995345,
                    0.015198666563616457,
                    0.14810397642748008,
                    0.6897673349851,
                    1.6763848301838038,
                    2.053191626637759,
                    1.0,
                ],
            )
        } else if r <= 27.0 {
            // min(p,1-p) >= exp(-729) ~= 2.5e-317
            r -= 5.0;
            horner(
                r,
                &[
                    2.0103343992922881e-07,
                    2.7115555687434876e-05,
                    0.0012426609473880784,
                    0.026532189526576124,
                    0.29656057182850487,
                    1.7848265399172913,
                    5.463784911164114,
                    6.657904643501103,
                ],
            ) / horner(
                r,
                &[
                    2.0442631033899397e-15,
                    1.421511758316446e-07,
                    1.8463183175100548e-05,
                    0.0007868691311456133,
                    0.014875361290850615,
                    0.1369298809227358,
                    0.599832206555888,
                    1.0,
                ],
            )
        } else if r >= 640000000.0 {
            r * M_SQRT2
        } else {
            // asymptotic tail: x2 iterates toward -log(phi(x)/x)
            let s2 = -2.0 * lp;
            let mut x2 = s2 - libm::log(M_2PI * s2);
            if r < 36000.0 {
                x2 = s2 - libm::log(M_2PI * x2) - 2.0 / (2.0 + x2);
                if r < 840.0 {
                    x2 = s2 - libm::log(M_2PI * x2)
                        + 2.0 * libm::log1p(-(1.0 - 1.0 / (4.0 + x2)) / (2.0 + x2));
                    if r < 109.0 {
                        x2 = s2 - libm::log(M_2PI * x2)
                            + 2.0
                                * libm::log1p(
                                    -(1.0 - (1.0 - 5.0 / (6.0 + x2)) / (4.0 + x2)) / (2.0 + x2),
                                );
                        if r < 55.0 {
                            x2 = s2 - libm::log(M_2PI * x2)
                                + 2.0
                                    * libm::log1p(
                                        -(1.0
                                            - (1.0 - (5.0 - 9.0 / (8.0 + x2)) / (6.0 + x2))
                                                / (4.0 + x2))
                                            / (2.0 + x2),
                                    );
                        }
                    }
                }
            }
            libm::sqrt(x2)
        };
        if q < 0.0 {
            val = -val;
        }
        val
    };
    Some(val)
}

/// `c[0]·r^k + c[1]·r^(k-1) + … + c[k]` by Horner — the exact order
/// R evaluates its AS241 polynomials.
fn horner(r: f64, c: &[f64]) -> f64 {
    let mut acc = c[0];
    for &ck in &c[1..] {
        acc = acc * r + ck;
    }
    acc
}
