// Empty / not-found states (spec §8.2: empty states offer the next
// action). `children` holds the offered action (a Button or Link).
import type { ReactNode } from 'react';

export function EmptyState({
  message,
  children,
}: {
  message: string;
  children?: ReactNode;
}) {
  return (
    <div>
      <p>{message}</p>
      {children}
    </div>
  );
}
