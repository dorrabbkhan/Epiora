import { useState } from 'react';
import { Label, Radio, RadioGroup } from '../components';
import {
  applyTheme,
  THEME_STORAGE_KEY,
  type Theme,
} from '../../design/theme';
import { useEngineVersion, useStorageEstimate } from '../queries';
import { t } from '../strings';
import styles from './routes.module.css';

const THEMES: { value: Theme; label: string }[] = [
  { value: 'system', label: t.settings.themeSystem },
  { value: 'light', label: t.settings.themeLight },
  { value: 'dark', label: t.settings.themeDark },
  { value: 'hc-light', label: t.settings.themeHcLight },
  { value: 'hc-dark', label: t.settings.themeHcDark },
];

function readStoredTheme(): Theme {
  const stored = localStorage.getItem(THEME_STORAGE_KEY);
  return THEMES.some((theme) => theme.value === stored)
    ? (stored as Theme)
    : 'system';
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MiB`;
}

export function SettingsScreen() {
  const [theme, setTheme] = useState<Theme>(readStoredTheme);
  const engine = useEngineVersion();
  const storage = useStorageEstimate();
  const est = storage.data;
  return (
    <div className={styles.page}>
      <h1>{t.settings.title}</h1>
      <RadioGroup
        value={theme}
        onChange={(value) => {
          const next = value as Theme;
          setTheme(next);
          applyTheme(next);
          localStorage.setItem(THEME_STORAGE_KEY, next);
        }}
      >
        <Label>{t.settings.theme}</Label>
        {THEMES.map((theme) => (
          <Radio key={theme.value} value={theme.value}>
            {theme.label}
          </Radio>
        ))}
      </RadioGroup>
      <section>
        <h2>{t.settings.storage}</h2>
        {est === undefined ? (
          <p>{t.common.loading}</p>
        ) : (
          <>
            <p>
              {t.settings.storageUsed}: {formatBytes(est.usage)} ·{' '}
              {t.settings.storageQuota}: {formatBytes(est.quota)}
            </p>
            <p>
              {t.settings.persisted}:{' '}
              {est.persisted
                ? t.settings.persistedYes
                : t.settings.persistedNo}
            </p>
          </>
        )}
      </section>
      <section>
        <h2>{t.settings.engineVersion}</h2>
        <p>{engine.data ?? t.common.loading}</p>
      </section>
    </div>
  );
}
