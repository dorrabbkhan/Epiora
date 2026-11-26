import {
  createRootRoute,
  createRoute,
  createRouter,
  Link,
  Outlet,
} from '@tanstack/react-router';
import { AnalysisScreen } from './routes/analysis';
import { CalcGuideScreen } from './routes/calc-guide';
import { CalcIndexScreen } from './routes/calc-index';
import { CalcScreen } from './routes/calc';
import { DatasetScreen } from './routes/dataset';
import { ImportScreen } from './routes/import';
import { LegacyImportScreen, LegacyPreviewScreen, LegacyReimportScreen } from './routes/legacy';
import { ProjectsScreen } from './routes/index';
import { ProjectScreen } from './routes/project';
import { SettingsScreen } from './routes/settings';
import { ValidationScreen } from './routes/validation';
import { t } from './strings';
import shell from './shell.module.css';

// Spec §8.1 routes. Screens are placeholders until Tasks 10–12 fill
// them; they use only the Epiora component set and strings.ts.
function Shell() {
  return (
    <>
      <header className={shell.header}>
        <nav className={shell.nav} aria-label={t.nav.main}>
          <Link to="/" className={shell.brand}>
            {t.appName}
          </Link>
          <Link to="/calc" className={shell.link}>
            {t.nav.calculators}
          </Link>
          <Link to="/validation" className={shell.link}>
            {t.nav.validation}
          </Link>
          <Link to="/settings" className={shell.link}>
            {t.nav.settings}
          </Link>
        </nav>
      </header>
      <main className={shell.main}>
        <Outlet />
      </main>
    </>
  );
}

function NotFound() {
  return (
    <div>
      <h1>{t.appName}</h1>
      <p>Page not found.</p>
    </div>
  );
}

const rootRoute = createRootRoute({
  component: Shell,
  notFoundComponent: NotFound,
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: ProjectsScreen,
});

const projectRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/p/$project',
  component: function ProjectRoute() {
    const { project } = projectRoute.useParams();
    return <ProjectScreen project={project} />;
  },
});

const importRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/p/$project/import',
  component: ImportScreen,
});

const legacyRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/legacy',
  component: LegacyImportScreen,
});

const legacyPreviewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/legacy/preview/$previewId',
  component: function LegacyPreviewRoute() {
    const { previewId } = legacyPreviewRoute.useParams();
    return <LegacyPreviewScreen previewId={previewId} />;
  },
});

const legacyReimportRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/p/$project/reimport',
  component: function LegacyReimportRoute() {
    const { project } = legacyReimportRoute.useParams();
    return <LegacyReimportScreen project={project} />;
  },
});

const datasetRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/p/$project/d/$dataset',
  component: function DatasetRoute() {
    const { project, dataset } = datasetRoute.useParams();
    return <DatasetScreen project={project} dataset={dataset} />;
  },
});

const analysisRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/p/$project/analysis',
  component: function AnalysisRoute() {
    const { project } = analysisRoute.useParams();
    return <AnalysisScreen project={project} />;
  },
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/settings',
  component: SettingsScreen,
});

// v1.0 Task 14: calculators live on the same shell but run against
// the store-less calc worker — no `epiora-store` lock.
const calcIndexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/calc',
  component: CalcIndexScreen,
});

// v1.0 Task 23: the "Which calculator?" guide — static so it wins
// over the `$id` parameter for the literal `/calc/guide`.
const calcGuideRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/calc/guide',
  validateSearch: (search: Record<string, unknown>) => search,
  component: CalcGuideScreen,
});

const calcRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/calc/$id',
  validateSearch: (search: Record<string, unknown>) => search,
  component: function CalcRoute() {
    const { id } = calcRoute.useParams();
    return <CalcScreen id={id} />;
  },
});

// v1.0 Task 24: the public validation dossier — store-less and
// offline like `/calc` (static `validation.json` + the calc worker's
// registry fallback, spec §7.4).
const validationRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/validation',
  component: ValidationScreen,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  projectRoute,
  importRoute,
  legacyRoute,
  legacyPreviewRoute,
  legacyReimportRoute,
  datasetRoute,
  analysisRoute,
  settingsRoute,
  calcIndexRoute,
  calcGuideRoute,
  calcRoute,
  validationRoute,
]);

export const router = createRouter({
  routeTree,
  defaultPreload: 'intent',
  // v1.0 Task 14: calculator links carry raw input values
  // (`?strata=10,11;4,20`) — the query is a plain string map, not the
  // default JSON-per-value codec (`numerator=%2212%22`).
  parseSearch: (searchStr) =>
    Object.fromEntries(new URLSearchParams(searchStr)),
  stringifySearch: (search) => {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(search)) {
      if (value !== undefined && value !== null) {
        params.set(key, String(value));
      }
    }
    const s = params.toString();
    return s === '' ? '' : `?${s}`;
  },
});

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
