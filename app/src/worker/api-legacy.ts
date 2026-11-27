// Legacy import/re-import slice of the data-worker RPC surface (v1.1
// Task 15). Kept out of `api.ts` so the central contract stays within
// the repository file-size rule; `api.ts` spreads `legacyApi` into the
// one exposed object.

import { wire } from './rpc-wire';
import { moveSessionsToCurrent, type MovedSession } from './lang-move';
import * as legacyWorker from './legacy';
import type { LegacyPreview, PlainPreview, RefreshPreview, SqlServerTableInput } from './legacy';
import { withStore } from './store-backend';
import * as legacyRepo from '../store/repo/legacy';
import type { LegacyProject as LegacyProjectInfo } from '../store/repo/legacy';
import type { ImportProgress } from './api';

export type { LegacyProjectInfo, MovedSession };

export interface LegacyDataApi {
  /** Map picked legacy files to a project preview without writing. */
  inspectLegacy(files: File[], codePage?: string): Promise<LegacyPreview>;
  /** Desktop SQL Server tables already read as EPTB by the Tauri bridge. */
  inspectLegacySqlServer(tables: SqlServerTableInput[]): Promise<LegacyPreview>;
  /** Whether a project came from a legacy import. */
  getLegacyProject(projectId: string): Promise<LegacyProjectInfo | null>;
  /** Starts one cancellable project/plain-table import attempt. */
  beginLegacyImport(previewId: string): Promise<void>;
  /** Commits a `inspectLegacy` preview — one transaction. */
  commitLegacy(previewId: string, onProgress: (p: ImportProgress) => void): Promise<string>;
  /** Abandons an uncommitted legacy import preview. */
  discardLegacyPreview(previewId: string): Promise<void>;
  /** Diffs a newer copy of the file against the committed project. */
  planRefresh(projectId: string, files: File[], codePage?: string): Promise<RefreshPreview>;
  commitRefresh(previewId: string): Promise<void>;
  /** Abandons an uncommitted re-import preview. */
  discardRefreshPreview(previewId: string): Promise<void>;
  /** Unknown-layout file: list ordinary tables, then import chosen ones. */
  inspectPlainLegacy(files: File[], codePage?: string): Promise<PlainPreview>;
  commitPlainLegacy(
    previewId: string,
    tables: string[],
    onProgress: (p: ImportProgress) => void,
  ): Promise<string>;
  /** Abandons an uncommitted plain-table import preview. */
  discardPlainPreview(previewId: string): Promise<void>;
  /** Move persisted working views onto current dataset versions. */
  moveSessionsToCurrent(projectId: string): Promise<MovedSession[]>;
}

export const legacyApi: LegacyDataApi = {
  inspectLegacy: wire((files, codePage) => legacyWorker.inspectLegacy(files, codePage)),
  inspectLegacySqlServer: wire((tables) => legacyWorker.inspectLegacySqlServer(tables)),
  getLegacyProject: wire((projectId) =>
    withStore((s) => legacyRepo.project(s, projectId)),
  ),
  beginLegacyImport: wire(async (previewId) => {
    legacyWorker.beginLegacyImport(previewId);
  }),
  commitLegacy: wire((previewId, onProgress) =>
    withStore((s) => legacyWorker.commitLegacy(s, previewId, onProgress)),
  ),
  discardLegacyPreview: wire(async (previewId) => {
    legacyWorker.discardPreview(previewId);
  }),
  planRefresh: wire((projectId, files, codePage) =>
    withStore((s) => legacyWorker.planRefresh(s, projectId, files, codePage)),
  ),
  commitRefresh: wire((previewId) =>
    withStore((s) => legacyWorker.commitRefresh(s, previewId)),
  ),
  discardRefreshPreview: wire(async (previewId) => {
    legacyWorker.discardRefreshPreview(previewId);
  }),
  inspectPlainLegacy: wire((files, codePage) => legacyWorker.inspectPlainLegacy(files, codePage)),
  commitPlainLegacy: wire((previewId, tables, onProgress) =>
    withStore((s) => legacyWorker.commitPlainLegacy(s, previewId, tables, onProgress)),
  ),
  discardPlainPreview: wire(async (previewId) => {
    legacyWorker.discardPlainPreview(previewId);
  }),
  moveSessionsToCurrent: wire((projectId) =>
    withStore((s) => moveSessionsToCurrent(s, projectId)),
  ),
};
