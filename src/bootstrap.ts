import { createApp } from 'vue';
import { createPinia } from 'pinia';
import { invoke } from '@tauri-apps/api/core';
import AppRoot from '@/App.vue';
import router from '@/router';
import { useSettingsStore } from '@/modules/settings/stores/settings.store';

function stringifyFrontendError(error: unknown): string {
  if (typeof error === 'string') {
    return error;
  }

  if (error instanceof Error) {
    return error.stack || error.message || error.name;
  }

  if (error && typeof error === 'object') {
    try {
      return JSON.stringify(error);
    } catch {
      return String(error);
    }
  }

  return String(error);
}

function reportFrontendError(scope: string, error: unknown): void {
  const message = stringifyFrontendError(error).trim();
  if (!scope.trim() || !message) {
    return;
  }

  void invoke('frontend_log_error', {
    scope,
    message,
  }).catch(() => {
    // ignore runtime log failures
  });
}

export async function bootstrapApplication(): Promise<void> {
  const app = createApp(AppRoot);
  const pinia = createPinia();
  app.use(pinia);
  const settingsStore = useSettingsStore(pinia);
  await settingsStore.startSync();
  await settingsStore.ensureLoaded();
  app.config.errorHandler = (error, _instance, info) => {
    reportFrontendError(`vue:${info}`, error);
  };
  window.addEventListener('error', (event) => {
    reportFrontendError('window.error', event.error ?? event.message);
  });
  window.addEventListener('unhandledrejection', (event) => {
    reportFrontendError('window.unhandledrejection', event.reason);
  });
  router.onError((error) => {
    reportFrontendError('router.error', error);
  });
  app.use(router);
  app.mount('#app');
}
