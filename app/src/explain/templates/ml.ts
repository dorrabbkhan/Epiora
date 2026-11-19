// MathML string builders for the formula templates (v1.0 plan Task
// 16, spec §6): `math` wraps each template's expression in a
// `<math>` element carrying the `aria-label` the a11y gate looks
// for; the rest assemble rows, fractions, roots and scripts. Values
// arrive pre-formatted — these builders never do arithmetic.

const el = (name: string, ...children: string[]): string =>
  `<${name}>${children.join('')}</${name}>`;

/** The formula's `<math>` element; `label` describes the formula in
 *  words (the accessible name). */
export const math = (label: string, ...children: string[]): string =>
  `<math xmlns="http://www.w3.org/1998/Math/MathML" aria-label="${label}">${children.join('')}</math>`;

export const mn = (value: string): string => el('mn', value);
export const mi = (name: string): string => el('mi', name);
export const mo = (op: string): string => el('mo', op);
export const mrow = (...children: string[]): string => el('mrow', ...children);
export const mfrac = (num: string, den: string): string => el('mfrac', num, den);
export const msqrt = (child: string): string => el('msqrt', child);
export const msup = (base: string, exp: string): string => el('msup', base, exp);
export const msub = (base: string, sub: string): string => el('msub', base, sub);
export const msubsup = (base: string, sub: string, sup: string): string =>
  el('msubsup', base, sub, sup);

/** `a op b` inside an `<mrow>` — the common binary shape. */
export const bin = (a: string, op: string, b: string): string =>
  mrow(a, mo(op), b);
