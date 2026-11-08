// All UI strings live here (spec §9.3: one English module, no i18n
// framework yet).
export const t = {
  appName: 'Epiora',
  nav: {
    main: 'Main navigation',
    home: 'Projects',
    settings: 'Settings',
  },
  projects: {
    title: 'Projects',
    empty: 'No projects yet. Create one to start importing data.',
    create: 'New project',
  },
  project: {
    datasets: 'Datasets',
    emptyDatasets: 'No datasets yet.',
    outputLog: 'Output log',
    emptyLog: 'No results yet.',
    importData: 'Import data',
    analysis: 'Analysis workspace',
  },
  import: {
    title: 'Import data',
    placeholder: 'The import wizard arrives with the data worker.',
    chooseFile: 'Choose file',
  },
  dataset: {
    title: 'Dataset',
    placeholder: 'The data grid arrives with the store and worker.',
  },
  analysis: {
    title: 'Analysis',
    placeholder: 'FREQ and TABLES arrive with the data worker.',
  },
  settings: {
    title: 'Settings',
    theme: 'Theme',
    themeSystem: 'System',
    themeLight: 'Light',
    themeDark: 'Dark',
    themeHcLight: 'High contrast light',
    themeHcDark: 'High contrast dark',
    storage: 'Storage',
    storagePlaceholder: 'Storage use appears once the store exists.',
    engineVersion: 'Engine version',
    enginePlaceholder: 'Reported by the data worker.',
  },
} as const;
