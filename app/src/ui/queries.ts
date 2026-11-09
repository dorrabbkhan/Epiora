// TanStack Query hooks over the data worker RPC (v0.3 plan Task 10):
// one hook per `DataApi` call the screens use, with the plan's query
// keys; mutations invalidate the matching keys.
import {
  useMutation,
  useQueries,
  useQuery,
  useQueryClient,
  type UseQueryOptions,
} from '@tanstack/react-query';
import { dataApi } from '../worker/client';
import type { GridPage } from '../worker/api';

export function useProjects() {
  return useQuery({ queryKey: ['projects'], queryFn: () => dataApi.listProjects() });
}

function useProjectsInvalidator() {
  const qc = useQueryClient();
  return () => qc.invalidateQueries({ queryKey: ['projects'] });
}

export function useCreateProject() {
  const invalidate = useProjectsInvalidator();
  return useMutation({
    mutationFn: (name: string) => dataApi.createProject(name),
    onSuccess: invalidate,
  });
}

export function useRenameProject() {
  const invalidate = useProjectsInvalidator();
  return useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) =>
      dataApi.renameProject(id, name),
    onSuccess: invalidate,
  });
}

export function useDeleteProject() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => dataApi.deleteProject(id),
    // A delete cascades to datasets, versions, pages and runs.
    onSuccess: () => qc.invalidateQueries(),
  });
}

export function useDatasets(projectId: string) {
  return useQuery({
    queryKey: ['datasets', projectId],
    queryFn: () => dataApi.listDatasets(projectId),
  });
}

export function useDataset(datasetId: string) {
  return useQuery({
    queryKey: ['dataset', datasetId],
    queryFn: () => dataApi.getDataset(datasetId),
    retry: false,
  });
}

export function useVersions(datasetId: string) {
  return useQuery({
    queryKey: ['versions', datasetId],
    queryFn: () => dataApi.listVersions(datasetId),
  });
}

/** Query options for one 100-row grid page (shared by
 *  `useVersionPage` and `useVersionPages`). */
export function versionPageQuery(
  datasetId: string,
  version: number,
  offset: number,
  limit: number,
): UseQueryOptions<GridPage> {
  return {
    queryKey: ['page', datasetId, version, offset],
    queryFn: () => dataApi.getVersionPage(datasetId, version, offset, limit),
  };
}

export function useVersionPage(
  datasetId: string,
  version: number,
  offset: number,
  limit: number,
) {
  return useQuery(versionPageQuery(datasetId, version, offset, limit));
}

/** One `useVersionPage` per requested page offset (DataGrid keeps the
 *  visible range loaded). */
export function useVersionPages(
  datasetId: string,
  version: number,
  offsets: number[],
  limit: number,
) {
  return useQueries({
    queries: offsets.map((offset) =>
      versionPageQuery(datasetId, version, offset, limit),
    ),
  });
}

export function useRuns(projectId: string) {
  return useQuery({
    queryKey: ['runs', projectId],
    queryFn: () => dataApi.listRuns(projectId),
  });
}

export function useEngineVersion() {
  return useQuery({
    queryKey: ['engineVersion'],
    queryFn: () => dataApi.engineVersion(),
    staleTime: Infinity,
  });
}

export function useStorageEstimate() {
  return useQuery({
    queryKey: ['storageEstimate'],
    queryFn: () => dataApi.storageEstimate(),
  });
}
