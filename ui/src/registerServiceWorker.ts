import { registerSW } from 'virtual:pwa-register';

export function initServiceWorker(onUpdateFound?: () => void) {
  const updateSW = registerSW({
    immediate: true,
    onNeedRefresh() {
      console.log('[PWA] New version available, updating cache...');
      if (onUpdateFound) onUpdateFound();
      updateSW(true);
    },
    onOfflineReady() {
      console.log('[PWA] App ready for offline exploration.');
    },
  });

  // Poll for updates every 60 seconds
  setInterval(() => {
    updateSW();
  }, 60 * 1000);
}
