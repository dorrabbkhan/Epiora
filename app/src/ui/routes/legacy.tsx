// Legacy import routes (v1.1 plan Task 15): the picker, the write-free
// preview keyed by preview id, and the project re-import flow.
import { ImportLegacy } from '../../legacy/ImportLegacy';
import { LegacyPreview } from '../../legacy/LegacyPreview';
import { ReimportDialog } from '../../legacy/ReimportDialog';
import styles from './routes.module.css';

export function LegacyImportScreen() {
  return (
    <div className={styles.page}>
      <ImportLegacy />
    </div>
  );
}

export function LegacyPreviewScreen({ previewId }: { previewId: string }) {
  return (
    <div className={styles.page}>
      <LegacyPreview previewId={previewId} />
    </div>
  );
}

export function LegacyReimportScreen({ project }: { project: string }) {
  return (
    <div className={styles.page}>
      <ReimportDialog project={project} />
    </div>
  );
}
