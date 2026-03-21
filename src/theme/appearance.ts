import { getCurrentWindow, type Theme } from '@tauri-apps/api/window';
import type { AppSettings, ThemeMode } from '@/modules/settings/types';

export function resolveTheme(mode: ThemeMode): 'light' | 'dark' {
  if (mode === 'light' || mode === 'dark') {
    return mode;
  }

  if (typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches) {
    return 'dark';
  }

  return 'light';
}

export function resolveAccentContrast(color: string): string {
  const hex = color.trim().replace('#', '');
  if (!/^[0-9a-fA-F]{6}$/.test(hex)) {
    return '#ffffff';
  }

  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  const luminance = (r * 299 + g * 587 + b * 114) / 1000;

  return luminance >= 160 ? '#14151a' : '#ffffff';
}

export function applyAppearance(settings: AppSettings): void {
  if (typeof document === 'undefined') {
    return;
  }

  const root = document.documentElement;
  const resolvedTheme = resolveTheme(settings.themeMode);
  root.dataset.theme = resolvedTheme;
  root.dataset.glassStyle = settings.liquidGlassStyle;
  root.style.setProperty('--accent', settings.accentColor);
  root.style.setProperty('--accent-contrast', resolveAccentContrast(settings.accentColor));
  void syncWindowTheme(resolvedTheme);
}

async function syncWindowTheme(theme: Theme): Promise<void> {
  try {
    await getCurrentWindow().setTheme(theme);
  } catch {
    // ignore native theme sync failures and keep CSS theme as the source of truth
  }
}
