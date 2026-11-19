// Assumption notes beneath a result table (v1.0 plan Task 23, spec
// §6.1): each engine `Note` renders its plain-language `words` from
// `NOTE_TEXT` — never the raw code — plus a link to the suggested
// alternative the caller resolved (`suggests` → an in-page anchor on
// the statistic's row, or a calculator link), the literature source
// and the "pending adviser review" marker. Notes never alter, hide
// or reorder the statistics themselves.

import type { Note } from '@engine-types/Note';
import { NOTE_TEXT } from '../../explain/notes';
import { t } from '../strings';
import styles from './NotesList.module.css';

export interface NoteLink {
  href: string;
  label: string;
}

export function NotesList({
  notes,
  linkFor,
}: {
  notes: Note[];
  /** Resolves a note's `suggests` id to a link; returning null drops
   *  the suggestion line. Optional — a list without it shows words
   *  and source only. */
  linkFor?: ((suggests: string) => NoteLink | null) | undefined;
}) {
  const items = notes.filter((note) => NOTE_TEXT[note.code] !== undefined);
  if (items.length === 0) return null;
  return (
    <div className={styles.notes}>
      <ul>
        {items.map((note, i) => {
          const text = NOTE_TEXT[note.code];
          const link =
            note.suggests !== undefined &&
            note.suggests !== null &&
            linkFor !== undefined
              ? linkFor(note.suggests)
              : null;
          return (
            <li key={i}>
              <section role="note">
                <p className={styles.words}>
                  {text?.words}
                  {link !== null && (
                    <>
                      {' '}
                      {t.notes.suggest} <a href={link.href}>{link.label}</a>.
                    </>
                  )}
                </p>
                <p className={styles.meta}>
                  {t.notes.source}: {text?.source}
                </p>
              </section>
            </li>
          );
        })}
      </ul>
      <p className={styles.review}>{t.notes.pendingReview}</p>
    </div>
  );
}
