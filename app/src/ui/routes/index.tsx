import { useState } from 'react';
import { Link } from '@tanstack/react-router';
import { Heading } from 'react-aria-components';
import {
  Button,
  Cell,
  Column,
  Dialog,
  DialogTrigger,
  Input,
  Label,
  Modal,
  Row,
  Table,
  TableBody,
  TableHeader,
  TextField,
} from '../components';
import {
  useCreateProject,
  useDeleteProject,
  useProjects,
  useRenameProject,
} from '../queries';
import { EmptyState } from '../screens/EmptyState';
import { t } from '../strings';
import styles from './routes.module.css';
import { RpcError } from '../../worker/errors';

/** The name dialog shared by create and rename. */
function ProjectNameDialog({
  title,
  submitLabel,
  defaultName,
  triggerLabel,
  onSubmit,
}: {
  title: string;
  submitLabel: string;
  defaultName?: string;
  triggerLabel: string;
  onSubmit: (name: string) => Promise<unknown>;
}) {
  const [error, setError] = useState<string | null>(null);
  return (
    <DialogTrigger>
      <Button>{triggerLabel}</Button>
      <Modal isDismissable>
        <Dialog>
          {({ close }) => (
            <>
              <Heading slot="title">{title}</Heading>
              <form
                onSubmit={async (e) => {
                  e.preventDefault();
                  const name = String(
                    new FormData(e.currentTarget).get('name') ?? '',
                  ).trim();
                  if (name === '') return;
                  try {
                    await onSubmit(name);
                    setError(null);
                    close();
                  } catch (err) {
                    setError(
                      err instanceof RpcError ? err.userMessage : String(err),
                    );
                  }
                }}
              >
                <TextField name="name" defaultValue={defaultName ?? ''} isRequired autoFocus>
                  <Label>{t.projects.nameLabel}</Label>
                  <Input />
                </TextField>
                {error !== null && <p role="alert">{error}</p>}
                <div className={styles.actions}>
                  <Button type="submit">{submitLabel}</Button>
                  <Button slot="close">{t.common.cancel}</Button>
                </div>
              </form>
            </>
          )}
        </Dialog>
      </Modal>
    </DialogTrigger>
  );
}

function DeleteProjectDialog({
  onDelete,
}: {
  onDelete: () => Promise<unknown>;
}) {
  const [error, setError] = useState<string | null>(null);
  return (
    <DialogTrigger>
      <Button>{t.common.delete}</Button>
      <Modal isDismissable>
        <Dialog>
          {({ close }) => (
            <>
              <Heading slot="title">{t.projects.deleteTitle}</Heading>
              <p>{t.projects.deleteConfirm}</p>
              {error !== null && <p role="alert">{error}</p>}
              <div className={styles.actions}>
                <Button
                  onPress={async () => {
                    try {
                      await onDelete();
                      setError(null);
                      close();
                    } catch (err) {
                      setError(
                        err instanceof RpcError ? err.userMessage : String(err),
                      );
                    }
                  }}
                >
                  {t.common.delete}
                </Button>
                <Button slot="close">{t.common.cancel}</Button>
              </div>
            </>
          )}
        </Dialog>
      </Modal>
    </DialogTrigger>
  );
}

export function ProjectsScreen() {
  const projects = useProjects();
  const create = useCreateProject();
  const rename = useRenameProject();
  const del = useDeleteProject();
  const list = projects.data;
  return (
    <div className={styles.page}>
      <h1>{t.projects.title}</h1>
      <div className={styles.actions}>
        <ProjectNameDialog
          title={t.projects.createTitle}
          submitLabel={t.common.create}
          triggerLabel={t.projects.create}
          onSubmit={(name) => create.mutateAsync(name)}
        />
      </div>
      {list === undefined ? (
        <p>{t.common.loading}</p>
      ) : list.length === 0 ? (
        <EmptyState message={t.projects.empty} />
      ) : (
        <Table caption={t.projects.title} aria-label={t.projects.title}>
          <TableHeader>
            <Column isRowHeader>{t.projects.nameColumn}</Column>
            <Column>{t.projects.createdColumn}</Column>
            <Column>{t.common.actions}</Column>
          </TableHeader>
          <TableBody items={list}>
            {(project) => (
              <Row id={project.id}>
                <Cell>
                  <Link to="/p/$project" params={{ project: project.id }}>
                    {project.name}
                  </Link>
                </Cell>
                <Cell>{project.createdAt}</Cell>
                <Cell>
                  <div className={styles.actions}>
                    <ProjectNameDialog
                      title={t.projects.renameTitle}
                      submitLabel={t.common.save}
                      triggerLabel={t.common.rename}
                      defaultName={project.name}
                      onSubmit={(name) =>
                        rename.mutateAsync({ id: project.id, name })
                      }
                    />
                    <DeleteProjectDialog
                      onDelete={() => del.mutateAsync(project.id)}
                    />
                  </div>
                </Cell>
              </Row>
            )}
          </TableBody>
        </Table>
      )}
    </div>
  );
}
