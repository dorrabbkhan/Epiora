import {
  createRootRoute,
  createRoute,
  createRouter,
  Link,
  Outlet,
} from '@tanstack/react-router';
import { AnalysisScreen } from './routes/analysis';
import { DatasetScreen } from './routes/dataset';
import { ImportScreen } from './routes/import';
import { ProjectsScreen } from './routes/index';
import { ProjectScreen } from './routes/project';
import { SettingsScreen } from './routes/settings';
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

const routeTree = rootRoute.addChildren([
  indexRoute,
  projectRoute,
  importRoute,
  datasetRoute,
  analysisRoute,
  settingsRoute,
]);

export const router = createRouter({
  routeTree,
  defaultPreload: 'intent',
});

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
