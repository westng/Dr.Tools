import { translate } from '@/i18n';
import type { LocaleCode } from '@/modules/settings/types';

export type PrimaryNavKey = 'download' | 'record' | 'tasks' | 'settings';

interface PrimaryNavDefinition {
  key: PrimaryNavKey;
  labelKey: string;
  placement: 'main' | 'bottom';
  to: string;
}

const navigationDefinitions: PrimaryNavDefinition[] = [
  {
    key: 'download',
    labelKey: 'navigation.download',
    placement: 'main',
    to: '/download/video'
  },
  {
    key: 'record',
    labelKey: 'navigation.record',
    placement: 'main',
    to: '/record/live'
  },
  {
    key: 'tasks',
    labelKey: 'navigation.tasks',
    placement: 'main',
    to: '/tasks/history'
  },
  {
    key: 'settings',
    labelKey: 'navigation.settings',
    placement: 'bottom',
    to: '/settings'
  }
];

export function getPrimaryNavigation(locale: LocaleCode): {
  main: Array<{ key: PrimaryNavKey; label: string; to: string }>;
  bottom: Array<{ key: PrimaryNavKey; label: string; to: string }>;
} {
  const items = navigationDefinitions.map((item) => ({
    key: item.key,
    label: translate(locale, item.labelKey),
    to: item.to
  }));

  return {
    main: items.filter((item) => navigationDefinitions.find((definition) => definition.key === item.key)?.placement === 'main'),
    bottom: items.filter((item) => navigationDefinitions.find((definition) => definition.key === item.key)?.placement === 'bottom')
  };
}

export function resolvePrimaryNavKey(value: unknown): PrimaryNavKey {
  if (value === 'download' || value === 'record' || value === 'tasks' || value === 'settings') {
    return value;
  }
  return 'download';
}
