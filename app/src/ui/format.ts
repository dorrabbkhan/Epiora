// The one number format (spec §8.2), shared by the result renderers,
// the explain panel and the e2e specs. Kept in a leaf module with no
// React or stylesheet imports so tests and specs can import it
// without pulling a CSS module into their transform graph.
//
// Exactly 4 significant digits with trailing zeros kept, never
// exponent notation, locale-independent via `toPrecision` (ECMA-262)
// plus `expandExponential` for the magnitudes it renders in
// exponential form. `kind: 'p'` adds the open bound: p-values in
// [0, 0.0001) render "< 0.0001", including an exact 0 (or -0)
// produced when an upper tail underflows; estimates of the same
// magnitude keep their digits. Non-finite input renders "—".
export function formatNumber(
  x: number,
  kind: 'estimate' | 'p' = 'estimate',
): string {
  if (!Number.isFinite(x)) return '—';
  if (kind === 'p' && x >= 0 && x < 0.0001) return '< 0.0001';
  return expandExponential(x.toPrecision(4));
}

const EXPONENTIAL = /^(-?)(\d)(?:\.(\d+))?e([+-]\d+)$/;

/** `toPrecision` switches to exponential form outside [1e-6, 1e4);
 *  expanding it by hand keeps the "no exponent" rule for every
 *  finite input ("1.235e+4" → "12350", "3.000e-7" → "0.0000003000"). */
function expandExponential(s: string): string {
  const m = EXPONENTIAL.exec(s);
  if (m === null) return s;
  const [, sign, head, tail = '', expText] = m;
  const digits = head + tail;
  const point = Number(expText) + 1; // '.' sits `point` digits into `digits`
  if (point <= 0) return `${sign}0.${'0'.repeat(-point)}${digits}`;
  if (point >= digits.length) {
    return `${sign}${digits}${'0'.repeat(point - digits.length)}`;
  }
  return `${sign}${digits.slice(0, point)}.${digits.slice(point)}`;
}
