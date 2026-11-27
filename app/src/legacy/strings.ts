// English copy for the legacy import flow (v1.1 plan Task 15). Kept
// out of `ui/strings.ts` because the routes pass whole fragments to
// one another; global chrome/errors stay in the shared table.
export const legacyT = {
  openProject: 'Open an Epi Info project',
  reimport: 'Re-import a newer copy',
  title: 'Open an Epi Info project',
  chooseFiles: 'Choose project files',
  reading: 'Reading project…',
  browserNotice:
    'Browser storage is not encrypted. Do not import identifiable data unless you accept this risk.',
  sqlServerTitle: 'Connect to SQL Server',
  sqlServerIntro: 'Credentials are used for this import only and are never stored.',
  server: 'Server',
  port: 'Port',
  database: 'Database',
  user: 'User',
  password: 'Password',
  windowsAuth: 'Use Windows authentication',
  trustCertificate: 'Trust the server certificate',
  connect: 'Read project',
  readingTables: 'Reading SQL Server tables…',
  previewTitle: (name: string) => `Preview ${name}`,
  previewIntro:
    'Review every view, field, link, program, check-code block and migration report entry before writing.',
  views: 'Views',
  fields: (view: string) => `${view} fields`,
  links: 'Linked datasets',
  programs: 'Saved programs',
  checkCode: 'Check code blocks',
  report: 'Migration report entries',
  confirm: 'Confirm import',
  cancel: 'Cancel import',
  back: 'Back to project picker',
  missingPreview: 'This import preview is no longer available. Choose the project files again.',
  writeProgress: 'Write project',
  cancelled: 'The import was cancelled; nothing was written.',
  plainOfferTitle: 'This file is not a recognised Epi Info project.',
  plainOffer:
    'Import its tables as plain datasets instead of an Epi Info project.',
  chooseTables: 'Choose tables to import',
  plainTablesTitle: 'Tables to import',
  plainWarnings: 'Plain import warnings',
  plainBinaryWarning: 'Binary columns are not imported as plain datasets.',
  importTables: 'Import selected tables',
  importingTables: 'Importing tables…',
  reimportTitle: 'Re-import a newer copy',
  chooseNewer: 'Choose newer copy',
  reimportIntro:
    'Diff each keyed view against its latest imported version before creating new versions.',
  diffTitle: 'Re-import changes',
  confirmReimport: 'Confirm re-import',
  close: 'Close',
  noChanges: 'The newer copy has no record or schema changes; no new versions will be created.',
  sessionBanner: 'Open working views stayed on their source versions.',
  moveSessions: 'Move working views to new versions',
  movingSessions: 'Moving working views…',
  movedSessions: 'Working views moved to the new versions.',
  moveSessionsFailed: 'A working view could not be replayed on the new version.',
  workingViewsLabel: 'Working views',
  workingViewsBanner: 'Open working views stayed on their source versions.',
  moveWorkingViews: 'Move working views to new versions',
  movingWorkingViews: 'Moving working views…',
  movedWorkingViews: 'Working views moved to the new versions.',
  moveWorkingViewsFailed: 'A working view could not be replayed on the new version.',
  sqlServerNotRefreshable: 'SQL Server projects are not refreshable; choose a newer `.mdb` or `.db` copy of a file-backed project.',
  notLegacyProject: 'Only a project imported from Epi Info can be re-imported.',
  recordSamplesTitle: 'Record samples',
  addedRecordsTitle: 'Added records',
  removedRecordsTitle: 'Removed records',
  changedRecordsTitle: 'Changed fields',
  addedRow: (row: number) => `new row ${row + 1}`,
  headers: {
    field: 'Field',
    legacyType: 'Legacy type',
    epioraType: 'Epiora type',
    view: 'View',
    records: 'Records',
    deletedRecords: 'Deleted records',
    keyColumn: 'Key column',
    parent: 'Parent',
    child: 'Child',
    childKey: 'Child key',
    parentKey: 'Parent key',
    kind: 'Kind',
    name: 'Name',
    blocks: 'Blocks',
    table: 'Table',
    column: 'Column',
    item: 'Item',
    reason: 'Reason',
  },
  tableColumnView: 'View',
  tableColumnOutcome: 'Outcome',
  tableColumnAdded: 'Added',
  tableColumnRemoved: 'Removed',
  tableColumnChanged: 'Changed',
} as const;

/** User-facing copy for the Tauri SQL Server bridge's stable error codes. */
export function sqlErrorText(e: unknown): string {
  const code = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
  if (code.includes('sqlserver_auth')) return 'SQL Server authentication failed.';
  if (code.includes('sqlserver_connect')) return 'Could not connect to SQL Server.';
  if (code.includes('sqlserver_query')) return 'SQL Server rejected the table read.';
  return code;
}

export const legacyErrorT = {
  reimport_not_legacy: 'Only a project imported from Epi Info can be re-imported.',
  reimport_source_mismatch: 'That file is not a newer copy of this Epi Info project.',
} as const;
