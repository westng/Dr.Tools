import { computed } from 'vue';
import { storeToRefs } from 'pinia';
import { useSettingsStore } from '@/modules/settings/stores/settings.store';
import type { LocaleCode } from '@/modules/settings/types';
import zhCN from './locales/zh-CN';
import enUS from './locales/en-US';

const localeMap = {
  'zh-CN': zhCN,
  'en-US': enUS,
} as const;

export type AppMessages = typeof zhCN;

export function getMessages(locale: LocaleCode): AppMessages {
  return (localeMap[locale] ?? zhCN) as AppMessages;
}

export function translate(locale: LocaleCode, path: string): string {
  const segments = path.split('.');
  let current: unknown = getMessages(locale);

  for (const segment of segments) {
    if (!current || typeof current !== 'object' || !(segment in current)) {
      return path;
    }
    current = (current as Record<string, unknown>)[segment];
  }

  return typeof current === 'string' ? current : path;
}

export function useMessages<T>(selector: (messages: AppMessages) => T) {
  const settingsStore = useSettingsStore();
  const { settings } = storeToRefs(settingsStore);
  return computed(() => selector(getMessages(settings.value.locale)));
}
