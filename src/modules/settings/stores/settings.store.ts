import { defineStore } from 'pinia';
import { emit, listen } from '@tauri-apps/api/event';
import { checkUpdate, getSettings, updateSettings } from '@/modules/settings/api/settings.api';
import type { AppSettings, AppSettingsPatch } from '@/modules/settings/types';
import { toErrorMessage } from '@/lib/errors';
import { applyAppearance } from '@/theme/appearance';

const SETTINGS_SYNC_EVENT = 'drtools:settings-updated';
let settingsSyncStarted = false;

const defaultSettings: AppSettings = {
  themeMode: 'auto',
  liquidGlassStyle: 'transparent',
  accentColor: '#2f6dff',
  locale: 'zh-CN',
  exportDir: '',
  maxConcurrentDownloads: 3,
  downloadNotificationsEnabled: true,
  douyinCookie: '',
  douyinCookieUpdatedAt: null,
  douyinLastCheckedAt: null,
  douyinLastCheckStatus: 'not_configured',
  douyinLastCheckMessage: null,
  tiktokCookie: '',
  tiktokCookieUpdatedAt: null,
  tiktokLastCheckedAt: null,
  tiktokLastCheckStatus: 'not_configured',
  tiktokLastCheckMessage: null,
  autoCheckUpdates: true,
  lastUpdateCheckAt: null,
  lastUpdateStatus: null
};

interface SettingsState {
  loaded: boolean;
  loading: boolean;
  saving: boolean;
  checkingUpdate: boolean;
  settings: AppSettings;
  lastUpdateMessage: string;
  error: string;
}

export const useSettingsStore = defineStore('settings', {
  state: (): SettingsState => ({
    loaded: false,
    loading: false,
    saving: false,
    checkingUpdate: false,
    settings: defaultSettings,
    lastUpdateMessage: '',
    error: ''
  }),
  actions: {
    async startSync(): Promise<void> {
      if (settingsSyncStarted) {
        return;
      }

      settingsSyncStarted = true;
      try {
        await listen<AppSettings>(SETTINGS_SYNC_EVENT, (event) => {
          this.settings = event.payload;
          this.loaded = true;
          applyAppearance(this.settings);
        });
      } catch {
        settingsSyncStarted = false;
      }
    },
    async ensureLoaded(force = false): Promise<void> {
      if (this.loaded && !force) {
        applyAppearance(this.settings);
        return;
      }

      this.loading = true;
      this.error = '';
      try {
        this.settings = await getSettings();
        this.loaded = true;
        applyAppearance(this.settings);
        if (this.settings.autoCheckUpdates && !this.settings.lastUpdateCheckAt) {
          await this.checkForUpdates();
        }
      } catch (error) {
        this.error = toErrorMessage(error);
      } finally {
        this.loading = false;
      }
    },
    async savePatch(patch: AppSettingsPatch): Promise<void> {
      this.saving = true;
      this.error = '';
      const previous = this.settings;
      this.settings = {
        ...this.settings,
        ...patch
      };
      applyAppearance(this.settings);

      try {
        this.settings = await updateSettings(patch);
        applyAppearance(this.settings);
        try {
          await emit(SETTINGS_SYNC_EVENT, this.settings);
        } catch {
          // ignore sync broadcast failure and keep local update successful
        }
      } catch (error) {
        this.settings = previous;
        applyAppearance(this.settings);
        this.error = toErrorMessage(error);
      } finally {
        this.saving = false;
      }
    },
    async checkForUpdates(): Promise<void> {
      this.checkingUpdate = true;
      this.error = '';
      try {
        const result = await checkUpdate();
        this.lastUpdateMessage = result.message;
        this.settings = {
          ...this.settings,
          lastUpdateCheckAt: result.checkedAt,
          lastUpdateStatus: result.status
        };
      } catch (error) {
        this.error = toErrorMessage(error);
      } finally {
        this.checkingUpdate = false;
      }
    }
  }
});
