// Desktop-only SQL Server import form (v1.1 spec §4.1/§7.1): the
// Tauri bridge reads tables as EPTB batches with credentials scoped to
// each IPC call; only the batches cross into the data worker. Nothing
// from the form is written to settings, the store, or a log.
import { useState, type FormEvent } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { Button, Checkbox, FieldError, Input, Label, TextField } from '../ui/components';
import { dataApi } from '../worker/client';
import type { SqlServerAuth } from '../tauri/sqlserver';
import { rememberLegacyPreview } from './LegacyPreview';
import { legacyT, sqlErrorText } from './strings';
import styles from './legacy.module.css';

export function SqlServerForm() {
  const navigate = useNavigate();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [windowsAuth, setWindowsAuth] = useState(false);
  const [trust, setTrust] = useState(false);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const form = event.currentTarget;
    const data = new FormData(form);
    const host = String(data.get('server') ?? '').trim();
    const database = String(data.get('database') ?? '').trim();
    const user = String(data.get('user') ?? '');
    const password = String(data.get('password') ?? '');
    const portText = String(data.get('port') ?? '').trim();
    const auth: SqlServerAuth = windowsAuth ? 'windows' : { sql: { user, password } };
    const conn = {
      host,
      port: portText === '' ? 1433 : Number(portText),
      database,
      auth,
      trustCert: trust,
    };
    setBusy(true);
    setError(null);
    try {
      const bridge = await import('../tauri/sqlserver');
      const tables = await bridge.legacySqlServerTables(conn);
      const payloads = [];
      for (const name of tables) {
        const batches = await bridge.legacySqlServerBatches(conn, name, 50_000);
        payloads.push({ name, batches: batches.map((b) => new Uint8Array(b)) });
      }
      const preview = await dataApi.inspectLegacySqlServer(database, payloads);
      rememberLegacyPreview(preview);
      await navigate({ to: '/legacy/preview/$previewId', params: { previewId: preview.previewId } });
    } catch (e) {
      setError(sqlErrorText(e));
    } finally {
      form.reset();
      setWindowsAuth(false);
      setTrust(false);
      setBusy(false);
    }
  };

  return (
    <section className={styles.section}>
      <h2>{legacyT.sqlServerTitle}</h2>
      <p>{legacyT.sqlServerIntro}</p>
      {error !== null && <p role="alert">{error}</p>}
      <form className={styles.form} onSubmit={(e) => void submit(e)}>
        <TextField isRequired>
          <Label>{legacyT.server}</Label>
          <Input name="server" autoComplete="off" />
          <FieldError />
        </TextField>
        <TextField>
          <Label>{legacyT.port}</Label>
          <Input name="port" inputMode="numeric" autoComplete="off" />
          <FieldError />
        </TextField>
        <TextField isRequired>
          <Label>{legacyT.database}</Label>
          <Input name="database" autoComplete="off" />
          <FieldError />
        </TextField>
        <Checkbox isSelected={windowsAuth} onChange={setWindowsAuth}>
          {legacyT.windowsAuth}
        </Checkbox>
        {!windowsAuth && (
          <>
            <TextField>
              <Label>{legacyT.user}</Label>
              <Input name="user" autoComplete="username" />
              <FieldError />
            </TextField>
            <TextField>
              <Label>{legacyT.password}</Label>
              <Input name="password" type="password" autoComplete="current-password" />
              <FieldError />
            </TextField>
          </>
        )}
        <Checkbox isSelected={trust} onChange={setTrust}>
          {legacyT.trustCertificate}
        </Checkbox>
        <Button type="submit" isDisabled={busy}>
          {busy ? legacyT.readingTables : legacyT.connect}
        </Button>
      </form>
    </section>
  );
}
