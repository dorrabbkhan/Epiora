// Service-worker update prompt (spec §7.1): `vite-plugin-pwa` is
// configured `registerType: 'prompt'`, `injectRegister: null`, so
// UpdatePrompt registers the sw itself and shows the "New version
// available" toast when an update waits — the app never swaps
// silently mid-session; Reload applies it.
import { useEffect, useRef } from 'react';
import {
  Button,
  Toast,
  ToastContent,
  ToastList,
  ToastQueue,
  ToastRegion,
} from './components';
import { t } from './strings';

interface UpdateToast {
  message: string;
}

const queue = new ToastQueue<UpdateToast>({ maxVisibleToasts: 1 });

export function UpdatePrompt() {
  const update = useRef<(() => Promise<void>) | null>(null);

  useEffect(() => {
    if (!import.meta.env.PROD || !('serviceWorker' in navigator)) return;
    void import('virtual:pwa-register')
      .then(({ registerSW }) => {
        update.current = registerSW({
          immediate: true,
          onNeedRefresh() {
            queue.add({ message: t.update.available });
          },
        });
      })
      .catch(() => {
        // Registration is best-effort; the app still works without it.
      });
  }, []);

  return (
    <ToastRegion queue={queue} aria-label={t.update.regionLabel}>
      <ToastList<UpdateToast>>
        {({ toast }) => (
          <Toast toast={toast} aria-label={toast.content.message}>
            <ToastContent>{toast.content.message}</ToastContent>
            <Button onPress={() => void update.current?.()}>
              {t.update.reload}
            </Button>
          </Toast>
        )}
      </ToastList>
    </ToastRegion>
  );
}
