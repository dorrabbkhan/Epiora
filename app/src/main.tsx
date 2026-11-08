import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { RouterProvider } from '@tanstack/react-router';
import './design/tokens.css';
import {
  applyTheme,
  THEME_STORAGE_KEY,
  type Theme,
} from './design/theme';
import { router } from './ui/router';

const stored = localStorage.getItem(THEME_STORAGE_KEY);
const theme: Theme =
  stored === 'light' ||
  stored === 'dark' ||
  stored === 'hc-light' ||
  stored === 'hc-dark' ||
  stored === 'system'
    ? stored
    : 'system';
applyTheme(theme);

const rootEl = document.getElementById('root');
if (rootEl === null) {
  throw new Error('missing #root element');
}
createRoot(rootEl).render(
  <StrictMode>
    <RouterProvider router={router} />
  </StrictMode>,
);
