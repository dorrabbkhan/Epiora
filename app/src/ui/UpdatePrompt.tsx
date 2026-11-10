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
    let prompted = false;
    const showPrompt = () => {
      if (prompted) return;
      prompted = true;
      queue.add({ message: t.update.available });
    };
    let reloadArmed = false;
    /** When the prompt came from the fallback below (not workbox's
     *  'waiting' event), pwa-register's own 'controlling' → reload
     *  listener was never attached — arm ours instead. */
    const armReload = () => {
      if (reloadArmed) return;
      reloadArmed = true;
      navigator.serviceWorker.addEventListener(
        'controllerchange',
        () => window.location.reload(),
        { once: true },
      );
    };
    void import('virtual:pwa-register')
      .then(({ registerSW }) => {
        update.current = registerSW({
          immediate: true,
          onNeedRefresh: showPrompt,
          onRegisteredSW(_swUrl, registration) {
            // Workbox's 'waiting' event can miss an update whose
            // install the browser's own update check began before
            // workbox's listeners attached (a reload right after a
            // deploy): if one is mid-install or already parked now,
            // watch for it here too. `active !== null` is what makes it
            // an update — a first install also lands in `waiting`
            // momentarily before it activates (Firefox e2e showed the
            // toast on first load without this).
            const isUpdate = () =>
              registration !== undefined &&
              registration.waiting !== null &&
              registration.active !== null;
            registration?.installing?.addEventListener(
              'statechange',
              () => {
                if (isUpdate()) {
                  armReload();
                  showPrompt();
                }
              },
            );
            if (isUpdate()) {
              armReload();
              showPrompt();
            }
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
