// Special functions for template `compute` implementations (v1.0
// plan Task 16, spec §6). These mirror the engine's `special` crate
// module closely enough to reproduce step results inside the
// closed-form tolerance class: Lanczos `lgamma`, the regularized
// incomplete gamma functions (χ² and normal tails), the regularized
// incomplete beta (t and F tails), plus the small helpers the
// proportion and Poisson templates share.

const LANCZOS = [
  0.99999999999980993, 676.5203681218851, -1259.1392167224028,
  771.32342877765313, -176.61502916214059, 12.507343278686905,
  -0.13857109526572012, 9.9843695780195716e-6, 1.5056327351493116e-7,
];

export function lgamma(z: number): number {
  if (z < 0.5) return Math.log(Math.PI / Math.sin(Math.PI * z)) - lgamma(1 - z);
  const w = z - 1;
  let x = LANCZOS[0] ?? 0;
  for (let i = 1; i < LANCZOS.length; i += 1) x += (LANCZOS[i] ?? 0) / (w + i);
  const t = w + 7.5;
  return (
    0.5 * Math.log(2 * Math.PI) + (w + 0.5) * Math.log(t) - t + Math.log(x)
  );
}

/** Regularized lower incomplete gamma P(a, x). */
export function gammP(a: number, x: number): number {
  if (!(x > 0) || !(a > 0)) return x === 0 ? 0 : NaN;
  if (x >= a + 1) return 1 - gamQ(a, x);
  let sum = 1 / a;
  let del = sum;
  let ap = a;
  for (let n = 0; n < 300; n += 1) {
    ap += 1;
    del *= x / ap;
    sum += del;
    if (Math.abs(del) < Math.abs(sum) * 3e-16) break;
  }
  return sum * Math.exp(-x + a * Math.log(x) - lgamma(a));
}

/** Regularized upper incomplete gamma Q(a, x). */
export function gamQ(a: number, x: number): number {
  if (!(x > 0) || !(a > 0)) return x === 0 ? 1 : NaN;
  if (x < a + 1) return 1 - gammP(a, x);
  const fpmin = 1e-300;
  let b = x + 1 - a;
  let c = 1 / fpmin;
  let d = 1 / b;
  let h = d;
  for (let i = 1; i < 300; i += 1) {
    const an = -i * (i - a);
    b += 2;
    d = an * d + b;
    if (Math.abs(d) < fpmin) d = fpmin;
    c = b + an / c;
    if (Math.abs(c) < fpmin) c = fpmin;
    d = 1 / d;
    const del = d * c;
    h *= del;
    if (Math.abs(del - 1) < 3e-16) break;
  }
  return Math.exp(-x + a * Math.log(x) - lgamma(a)) * h;
}

function betacf(a: number, b: number, x: number): number {
  const fpmin = 1e-300;
  const qab = a + b;
  const qap = a + 1;
  const qam = a - 1;
  let c = 1;
  let d = 1 - (qab * x) / qap;
  if (Math.abs(d) < fpmin) d = fpmin;
  d = 1 / d;
  let h = d;
  for (let m = 1; m < 300; m += 1) {
    const m2 = 2 * m;
    let aa = (m * (b - m) * x) / ((qam + m2) * (a + m2));
    d = 1 + aa * d;
    if (Math.abs(d) < fpmin) d = fpmin;
    c = 1 + aa / c;
    if (Math.abs(c) < fpmin) c = fpmin;
    d = 1 / d;
    h *= d * c;
    aa = (-(a + m) * (qab + m) * x) / ((a + m2) * (qap + m2));
    d = 1 + aa * d;
    if (Math.abs(d) < fpmin) d = fpmin;
    c = 1 + aa / c;
    if (Math.abs(c) < fpmin) c = fpmin;
    d = 1 / d;
    const del = d * c;
    h *= del;
    if (Math.abs(del - 1) < 3e-16) break;
  }
  return h;
}

/** Regularized incomplete beta I_x(a, b) — the t and F tail CDFs. */
export function ibeta(x: number, a: number, b: number): number {
  if (!(x >= 0 && x <= 1)) return NaN;
  if (x === 0 || x === 1) return x;
  const bt = Math.exp(
    lgamma(a + b) - lgamma(a) - lgamma(b) + a * Math.log(x) + b * Math.log(1 - x),
  );
  if (x < (a + 1) / (a + b + 2)) return (bt * betacf(a, b, x)) / a;
  return 1 - (bt * betacf(b, a, 1 - x)) / b;
}

/** Standard normal CDF Φ(z) via the χ²₁ upper tail. */
export function phi(z: number): number {
  return z < 0 ? 0.5 * gamQ(0.5, (z * z) / 2) : 1 - 0.5 * gamQ(0.5, (z * z) / 2);
}

/** Clamp to [-1, 1] — the etiologic-fraction guard. */
export const clamp1 = (x: number): number => Math.min(Math.max(x, -1), 1);

/** The Wilson score interval for x/n at level z (unscaled 0–1). */
export function wilson(x: number, n: number, z: number): [number, number] {
  const zz = z * z;
  const p = x / n;
  const den = 1 + zz / n;
  const c = p + zz / (2 * n);
  const h = z * Math.sqrt((p * (1 - p)) / n + zz / (4 * n * n));
  return [(c - h) / den, (c + h) / den];
}

/** Byar's interval basis: [lower factor, upper factor, x + ½]. */
export function byarBase(x: number, z: number): [number, number, number] {
  const ap = x + 0.5;
  const zin = (z / 3) * Math.sqrt(1 / ap);
  const b = 1 - 1 / (9 * ap);
  return [b - zin, b + zin, ap];
}

/** Binary entropy of a probability (screening entropy reduction). */
export const entropy = (p: number): number =>
  -(p * Math.log(p) + (1 - p) * Math.log(1 - p));

/** The chi-square cell sum Σ (|o−e| − adj)²/e over paired `o_*`/`e_*`
 *  value keys; NaN when an `o_*` has no matching `e_*`. */
export function chiCells(v: Record<string, number>, adj: number): number {
  let s = 0;
  for (const k of Object.keys(v)) {
    if (!/^o_/.test(k)) continue;
    const e = v[`e_${k.slice(2)}`];
    if (e === undefined) return NaN;
    s += (Math.abs((v[k] ?? 0) - e) - adj) ** 2 / e;
  }
  return s;
}
