import type { ReactNode } from 'react';

// Mirrors react-aria-components' ClassNameOrFunction<T> contract.
type ClassNameOrFunction<T> =
  | string
  | ((values: T & { defaultClassName: string | undefined }) => string);

export function cx(...parts: (string | undefined | false)[]): string {
  return parts.filter(Boolean).join(' ');
}

// For components whose className accepts a render-prop function:
// merges the Epiora class with RAC's default class and any caller
// class. Components with a string-only className use `cx` directly.
export function renderClass<T>(
  base: string | undefined,
  className: ClassNameOrFunction<T> | undefined,
): (values: T & { defaultClassName: string | undefined }) => string {
  return (values) =>
    cx(
      base,
      values.defaultClassName,
      typeof className === 'function' ? className(values) : className,
    );
}

// Mirrors ChildrenOrFunction<T> for components that prepend chrome
// (e.g. the Checkbox indicator) before the caller's children.
export type RenderChildren<T> =
  | ReactNode
  | ((values: T & { defaultChildren: ReactNode | undefined }) => ReactNode);
