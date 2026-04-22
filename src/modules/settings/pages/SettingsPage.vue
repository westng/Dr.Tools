<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { openExternalUrl } from '@/api/system.api';
import {
  downloadEnvironment,
  getEnvironmentStatus,
  selectExportDirectory,
  validateToken
} from '@/modules/settings/api/settings.api';
import { useSettingsStore } from '@/modules/settings/stores/settings.store';
import { useAppStore } from '@/stores/app.store';
import { useMessages } from '@/i18n';
import { toErrorMessage } from '@/lib/errors';
import { resolveTheme } from '@/theme/appearance';
import type {
  LocaleCode,
  ManagedEnvironmentStatus,
  ThemeMode,
  TokenCheckStatus,
  TokenPlatform
} from '@/modules/settings/types';

const settingsStore = useSettingsStore();
const appStore = useAppStore();
const { settings, saving, checkingUpdate, error, lastUpdateMessage } = storeToRefs(settingsStore);
const { systemInfo } = storeToRefs(appStore);

const accentColors = ['#8f96a3', '#2f6dff', '#a65dd9', '#f062a8', '#ff6a57', '#ffb020', '#f5dd00', '#33c96f'];

const exportDirInput = ref('');
const maxConcurrentDownloadsInput = ref('3');
const douyinCookieInput = ref('');
const tiktokCookieInput = ref('');
const expandedGuide = ref<TokenPlatform | null>(null);
const validatingPlatform = ref<TokenPlatform | null>(null);
const tokenActionError = ref('');
const environmentStatus = ref<ManagedEnvironmentStatus | null>(null);
const loadingEnvironment = ref(false);
const downloadingEnvironment = ref(false);
const environmentActionError = ref('');
const text = useMessages((messages) => messages.pages.settings);
const colorLabels = computed<Record<string, string>>(() => ({ ...text.value.colorLabels }));
const resolvedTheme = computed<'light' | 'dark'>(() => resolveTheme(settings.value.themeMode));

const glassPreviewThemeClass = computed(() => (resolvedTheme.value === 'light' ? 'preview-glass-light' : 'preview-glass-dark'));

watch(
  settings,
  (value) => {
    exportDirInput.value = value.exportDir;
    maxConcurrentDownloadsInput.value = String(value.maxConcurrentDownloads);
    douyinCookieInput.value = value.douyinCookie;
    tiktokCookieInput.value = value.tiktokCookie;
  },
  { immediate: true, deep: true }
);

onMounted(async () => {
  await settingsStore.ensureLoaded();
  if (!systemInfo.value) {
    await appStore.bootstrap();
  }
  await refreshEnvironmentStatus();
});

const environmentButtonLabel = computed(() => {
  if (downloadingEnvironment.value) {
    return text.value.environmentDownloading;
  }
  if (environmentStatus.value?.installed) {
    return text.value.environmentRedownload;
  }
  return text.value.environmentDownload;
});

const environmentStatusLabel = computed(() => {
  switch (environmentStatus.value?.status) {
    case 'ready':
      return text.value.environmentReady;
    case 'invalid':
      return text.value.environmentInvalid;
    default:
      return text.value.environmentMissing;
  }
});

async function setThemeMode(mode: ThemeMode): Promise<void> {
  if (settings.value.themeMode === mode) {
    return;
  }
  await settingsStore.savePatch({ themeMode: mode });
}

async function setLocale(locale: LocaleCode): Promise<void> {
  if (settings.value.locale === locale) {
    return;
  }
  await settingsStore.savePatch({ locale });
}

async function setGlassStyle(style: 'transparent' | 'tinted'): Promise<void> {
  if (settings.value.liquidGlassStyle === style) {
    return;
  }
  await settingsStore.savePatch({ liquidGlassStyle: style });
}

async function setAccentColor(color: string): Promise<void> {
  if (settings.value.accentColor === color) {
    return;
  }
  await settingsStore.savePatch({ accentColor: color });
}

async function chooseDir(): Promise<void> {
  const selected = await selectExportDirectory(exportDirInput.value);
  if (!selected) {
    return;
  }
  exportDirInput.value = selected;
  await settingsStore.savePatch({ exportDir: selected });
}

async function saveExportDir(): Promise<void> {
  const normalized = exportDirInput.value.trim();
  exportDirInput.value = normalized;

  if (normalized === settings.value.exportDir) {
    return;
  }

  await settingsStore.savePatch({ exportDir: normalized });
}

async function saveMaxConcurrentDownloads(): Promise<void> {
  const parsed = Number.parseInt(maxConcurrentDownloadsInput.value.trim(), 10);
  if (!Number.isInteger(parsed)) {
    maxConcurrentDownloadsInput.value = String(settings.value.maxConcurrentDownloads);
    return;
  }

  const clamped = Math.min(8, Math.max(1, parsed));
  maxConcurrentDownloadsInput.value = String(clamped);

  if (clamped === settings.value.maxConcurrentDownloads) {
    return;
  }

  await settingsStore.savePatch({ maxConcurrentDownloads: clamped });
}

async function setDownloadNotifications(enabled: boolean): Promise<void> {
  if (settings.value.downloadNotificationsEnabled === enabled) {
    return;
  }

  await settingsStore.savePatch({ downloadNotificationsEnabled: enabled });
}

function normalizeCookie(value: string): string {
  return value.replace(/[\r\n]+/g, ' ').trim();
}

function getCookieInput(platform: TokenPlatform): string {
  return platform === 'douyin' ? douyinCookieInput.value : tiktokCookieInput.value;
}

function setCookieInput(platform: TokenPlatform, value: string): void {
  if (platform === 'douyin') {
    douyinCookieInput.value = value;
    return;
  }
  tiktokCookieInput.value = value;
}

function getSavedCookie(platform: TokenPlatform): string {
  return platform === 'douyin' ? settings.value.douyinCookie : settings.value.tiktokCookie;
}

function getStatus(platform: TokenPlatform): TokenCheckStatus {
  return platform === 'douyin' ? settings.value.douyinLastCheckStatus : settings.value.tiktokLastCheckStatus;
}

function getUpdatedAt(platform: TokenPlatform): string | null | undefined {
  return platform === 'douyin' ? settings.value.douyinCookieUpdatedAt : settings.value.tiktokCookieUpdatedAt;
}

function getCheckedAt(platform: TokenPlatform): string | null | undefined {
  return platform === 'douyin' ? settings.value.douyinLastCheckedAt : settings.value.tiktokLastCheckedAt;
}

function getCheckMessage(platform: TokenPlatform): string | null | undefined {
  return platform === 'douyin' ? settings.value.douyinLastCheckMessage : settings.value.tiktokLastCheckMessage;
}

function getStatusLabel(platform: TokenPlatform): string {
  return text.value.statusLabels[getStatus(platform)];
}

function getStatusClass(platform: TokenPlatform): string {
  return `token-status-${getStatus(platform)}`;
}

function getGuideItems(platform: TokenPlatform): string[] {
  return [...(platform === 'douyin' ? text.value.douyinGuide : text.value.tiktokGuide)];
}

async function savePlatformCookie(platform: TokenPlatform): Promise<void> {
  const normalized = normalizeCookie(getCookieInput(platform));
  setCookieInput(platform, normalized);

  if (normalized === getSavedCookie(platform)) {
    return;
  }

  tokenActionError.value = '';
  if (platform === 'douyin') {
    await settingsStore.savePatch({ douyinCookie: normalized });
    return;
  }

  await settingsStore.savePatch({ tiktokCookie: normalized });
}

async function clearPlatformCookie(platform: TokenPlatform): Promise<void> {
  tokenActionError.value = '';
  setCookieInput(platform, '');
  if (platform === 'douyin') {
    await settingsStore.savePatch({ douyinCookie: '' });
    return;
  }

  await settingsStore.savePatch({ tiktokCookie: '' });
}

async function validatePlatform(platform: TokenPlatform): Promise<void> {
  const normalized = normalizeCookie(getCookieInput(platform));
  if (!normalized) {
    tokenActionError.value = text.value.saveCookieBeforeValidate;
    return;
  }

  tokenActionError.value = '';
  if (normalized !== getSavedCookie(platform)) {
    await savePlatformCookie(platform);
    if (getSavedCookie(platform) !== normalized) {
      tokenActionError.value = text.value.saveFailedBeforeValidate;
      return;
    }
  }

  validatingPlatform.value = platform;
  try {
    await validateToken({ platform, cookie: normalized });
    await settingsStore.ensureLoaded(true);
  } catch (validateError) {
    tokenActionError.value = typeof validateError === 'object' && validateError && 'message' in validateError && typeof validateError.message === 'string'
      ? validateError.message
      : String(validateError);
  } finally {
    validatingPlatform.value = null;
  }
}

function toggleGuide(platform: TokenPlatform): void {
  expandedGuide.value = expandedGuide.value === platform ? null : platform;
}

async function openSite(platform: TokenPlatform): Promise<void> {
  const url = platform === 'douyin' ? 'https://www.douyin.com/' : 'https://www.tiktok.com/';
  await openExternalUrl(url);
}

async function setAutoCheck(enabled: boolean): Promise<void> {
  if (settings.value.autoCheckUpdates === enabled) {
    return;
  }
  await settingsStore.savePatch({ autoCheckUpdates: enabled });
}

async function checkUpdatesNow(): Promise<void> {
  await settingsStore.checkForUpdates();
}

async function refreshEnvironmentStatus(): Promise<void> {
  loadingEnvironment.value = true;
  environmentActionError.value = '';
  try {
    environmentStatus.value = await getEnvironmentStatus();
  } catch (error) {
    environmentActionError.value = toErrorMessage(error);
  } finally {
    loadingEnvironment.value = false;
  }
}

async function downloadManagedEnvironment(): Promise<void> {
  downloadingEnvironment.value = true;
  environmentActionError.value = '';
  try {
    environmentStatus.value = await downloadEnvironment();
  } catch (error) {
    environmentActionError.value = toErrorMessage(error);
    await refreshEnvironmentStatus();
  } finally {
    downloadingEnvironment.value = false;
  }
}
</script>

<template>
  <section class="settings-page">
    <div class="settings-group">
      <h3 class="settings-group-title">{{ text.appearance }}</h3>
      <article class="surface settings-block">
        <div class="setting-row">
          <span class="settings-label">{{ text.themeMode }}</span>
          <div class="setting-control">
            <div class="preview-grid preview-grid-3">
              <button class="preview-card" :class="{ active: settings.themeMode === 'auto' }" @click="setThemeMode('auto')">
                <span class="preview-art preview-theme preview-theme-auto"></span>
                <span class="preview-label">{{ text.auto }}</span>
              </button>
              <button class="preview-card" :class="{ active: settings.themeMode === 'light' }" @click="setThemeMode('light')">
                <span class="preview-art preview-theme preview-theme-light"></span>
                <span class="preview-label">{{ text.light }}</span>
              </button>
              <button class="preview-card" :class="{ active: settings.themeMode === 'dark' }" @click="setThemeMode('dark')">
                <span class="preview-art preview-theme preview-theme-dark"></span>
                <span class="preview-label">{{ text.dark }}</span>
              </button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <span class="settings-label">{{ text.liquidGlass }}</span>
          <div class="setting-control">
            <div class="preview-grid preview-grid-2">
              <button class="preview-card" :class="{ active: settings.liquidGlassStyle === 'transparent' }" @click="setGlassStyle('transparent')">
                <span class="preview-art preview-glass preview-glass-transparent" :class="glassPreviewThemeClass"></span>
                <span class="preview-label">{{ text.transparent }}</span>
              </button>
              <button class="preview-card" :class="{ active: settings.liquidGlassStyle === 'tinted' }" @click="setGlassStyle('tinted')">
                <span class="preview-art preview-glass preview-glass-tinted" :class="glassPreviewThemeClass"></span>
                <span class="preview-label">{{ text.tinted }}</span>
              </button>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <span class="settings-label">{{ text.locale }}</span>
          <div class="setting-control">
            <select :value="settings.locale" @change="setLocale(($event.target as HTMLSelectElement).value as LocaleCode)">
              <option value="zh-CN">{{ text.localeZh }}</option>
              <option value="en-US">{{ text.localeEn }}</option>
            </select>
          </div>
        </div>
      </article>
    </div>

    <div class="settings-group">
      <h3 class="settings-group-title">{{ text.themeSection }}</h3>
      <article class="surface settings-block">
        <div class="setting-row setting-row-color">
          <span class="settings-label">{{ text.color }}</span>
          <div class="setting-control">
            <div class="color-row">
              <div v-for="color in accentColors" :key="color" class="color-option">
                <button
                  class="color-dot"
                  :style="{ background: color }"
                  :class="{ active: settings.accentColor.toLowerCase() === color }"
                  @click="setAccentColor(color)"
                ></button>
                <span v-if="settings.accentColor.toLowerCase() === color" class="color-option-label">{{ colorLabels[color as keyof typeof colorLabels] }}</span>
              </div>
            </div>
          </div>
        </div>
      </article>
    </div>

    <div class="settings-group">
      <h3 class="settings-group-title">{{ text.downloadSection }}</h3>
      <article class="surface settings-block">
        <div class="setting-row setting-row-with-description">
          <span class="settings-label">{{ text.maxConcurrentDownloads }}</span>
          <div class="setting-control">
            <input v-model="maxConcurrentDownloadsInput" class="input" inputmode="numeric" @blur="saveMaxConcurrentDownloads" />
          </div>
        </div>

        <div class="setting-row setting-row-description">
          <p class="settings-hint">{{ text.maxConcurrentDownloadsHint }}</p>
        </div>

        <div class="setting-row setting-row-with-description">
          <span class="settings-label">{{ text.downloadNotifications }}</span>
          <div class="setting-control switch-row">
            <label class="switch">
              <input
                type="checkbox"
                :checked="settings.downloadNotificationsEnabled"
                @change="setDownloadNotifications(($event.target as HTMLInputElement).checked)"
              />
              <span></span>
            </label>
          </div>
        </div>

        <div class="setting-row setting-row-description">
          <p class="settings-hint">{{ text.downloadNotificationsHint }}</p>
        </div>

        <div class="setting-row">
          <span class="settings-label">{{ text.downloadPath }}</span>
          <div class="setting-control">
            <div class="row-inline">
              <input v-model="exportDirInput" class="input" @blur="saveExportDir" />
              <button @click="chooseDir">{{ text.choose }}</button>
            </div>
          </div>
        </div>
      </article>
    </div>

    <div class="settings-group">
      <h3 class="settings-group-title">{{ text.environmentSection }}</h3>
      <article class="surface settings-block">
        <div class="setting-row setting-row-with-description">
          <span class="settings-label">{{ text.environmentItem }}</span>
          <div class="setting-control">
            <button class="primary-btn" :disabled="downloadingEnvironment" @click="downloadManagedEnvironment">
              {{ environmentButtonLabel }}
            </button>
          </div>
        </div>

        <div class="setting-row setting-row-description">
          <p class="settings-hint">{{ environmentStatus?.message || text.environmentDescription }}</p>
        </div>

        <div class="setting-row">
          <span class="settings-label">{{ text.environmentStatus }}</span>
          <div class="setting-control">
            <span class="settings-hint">{{ loadingEnvironment ? text.environmentLoading : environmentStatusLabel }}</span>
          </div>
        </div>

        <div class="setting-row">
          <span class="settings-label">{{ text.environmentVersion }}</span>
          <div class="setting-control">
            <span class="settings-hint">{{ environmentStatus?.pythonVersion || '3.12' }}</span>
          </div>
        </div>

        <div class="setting-row">
          <span class="settings-label">{{ text.environmentSource }}</span>
          <div class="setting-control">
            <a class="settings-link" :href="environmentStatus?.sourceUrl || '#'" target="_blank" rel="noreferrer">
              {{ environmentStatus?.sourceLabel || text.environmentSourceDefault }}
            </a>
          </div>
        </div>
      </article>

      <p v-if="environmentActionError" class="danger-text">{{ environmentActionError }}</p>
    </div>

    <div class="settings-group">
      <h3 class="settings-group-title">{{ text.tokenSection }}</h3>
      <article class="surface token-overview-card">
        <h4 class="token-overview-title">{{ text.tokenOverviewTitle }}</h4>
        <p class="settings-hint">{{ text.tokenOverviewBody }}</p>
        <p class="settings-hint">{{ text.tokenOverviewRisk }}</p>
        <p class="settings-hint">{{ text.tokenOverviewHelp }}</p>
      </article>

      <div class="token-grid">
        <article class="surface token-card">
          <div class="token-card-head">
            <div>
              <h4 class="token-card-title">{{ text.douyinToken }}</h4>
              <span class="token-status" :class="getStatusClass('douyin')">{{ getStatusLabel('douyin') }}</span>
            </div>
          </div>
          <textarea
            v-model="douyinCookieInput"
            class="textarea token-textarea"
            spellcheck="false"
            :placeholder="text.tokenPlaceholder"
          ></textarea>
          <div class="token-actions">
            <button class="primary-btn" @click="savePlatformCookie('douyin')">{{ text.save }}</button>
            <button @click="clearPlatformCookie('douyin')">{{ text.clear }}</button>
            <button :disabled="validatingPlatform === 'douyin'" @click="validatePlatform('douyin')">
              {{ validatingPlatform === 'douyin' ? text.validating : text.validate }}
            </button>
            <button @click="openSite('douyin')">{{ text.openSite }}</button>
            <button @click="toggleGuide('douyin')">{{ expandedGuide === 'douyin' ? text.hideGuide : text.guide }}</button>
          </div>
          <div class="token-meta">
            <span><strong>{{ text.updatedAt }}:</strong> {{ getUpdatedAt('douyin') || text.notUpdated }}</span>
            <span><strong>{{ text.checkedAt }}:</strong> {{ getCheckedAt('douyin') || text.notChecked }}</span>
            <span><strong>{{ text.checkMessage }}:</strong> {{ getCheckMessage('douyin') || text.notChecked }}</span>
          </div>
          <div v-if="expandedGuide === 'douyin'" class="token-guide">
            <p class="settings-hint">{{ text.tokenGuideHint }}</p>
            <ol class="token-guide-list">
              <li v-for="item in getGuideItems('douyin')" :key="item">{{ item }}</li>
            </ol>
          </div>
        </article>

        <article class="surface token-card">
          <div class="token-card-head">
            <div>
              <h4 class="token-card-title">{{ text.tiktokToken }}</h4>
              <span class="token-status" :class="getStatusClass('tiktok')">{{ getStatusLabel('tiktok') }}</span>
            </div>
          </div>
          <textarea
            v-model="tiktokCookieInput"
            class="textarea token-textarea"
            spellcheck="false"
            :placeholder="text.tokenPlaceholder"
          ></textarea>
          <div class="token-actions">
            <button class="primary-btn" @click="savePlatformCookie('tiktok')">{{ text.save }}</button>
            <button @click="clearPlatformCookie('tiktok')">{{ text.clear }}</button>
            <button :disabled="validatingPlatform === 'tiktok'" @click="validatePlatform('tiktok')">
              {{ validatingPlatform === 'tiktok' ? text.validating : text.validate }}
            </button>
            <button @click="openSite('tiktok')">{{ text.openSite }}</button>
            <button @click="toggleGuide('tiktok')">{{ expandedGuide === 'tiktok' ? text.hideGuide : text.guide }}</button>
          </div>
          <div class="token-meta">
            <span><strong>{{ text.updatedAt }}:</strong> {{ getUpdatedAt('tiktok') || text.notUpdated }}</span>
            <span><strong>{{ text.checkedAt }}:</strong> {{ getCheckedAt('tiktok') || text.notChecked }}</span>
            <span><strong>{{ text.checkMessage }}:</strong> {{ getCheckMessage('tiktok') || text.notChecked }}</span>
          </div>
          <div v-if="expandedGuide === 'tiktok'" class="token-guide">
            <p class="settings-hint">{{ text.tokenGuideHint }}</p>
            <ol class="token-guide-list">
              <li v-for="item in getGuideItems('tiktok')" :key="item">{{ item }}</li>
            </ol>
          </div>
        </article>
      </div>

      <p v-if="tokenActionError" class="danger-text">{{ tokenActionError }}</p>
    </div>

    <div class="settings-group">
      <h3 class="settings-group-title">{{ text.updateSection }}</h3>
      <article class="surface settings-block">
        <div class="setting-row">
          <span class="settings-label">{{ text.currentVersion }}</span>
          <div class="setting-control">
            <strong>v{{ systemInfo?.appVersion || '0.1.0' }}</strong>
          </div>
        </div>

        <div class="setting-row">
          <span class="settings-label">{{ text.autoCheck }}</span>
          <div class="setting-control switch-row">
            <label class="switch">
              <input
                type="checkbox"
                :checked="settings.autoCheckUpdates"
                @change="setAutoCheck(($event.target as HTMLInputElement).checked)"
              />
              <span></span>
            </label>
          </div>
        </div>

        <div class="setting-row">
          <span class="settings-label">{{ text.checkNow }}</span>
          <div class="setting-control">
            <button :disabled="checkingUpdate" @click="checkUpdatesNow">
              {{ checkingUpdate ? text.checking : text.checkNow }}
            </button>
          </div>
        </div>

        <div class="setting-row">
          <span class="settings-label">{{ text.updateStatus }}</span>
          <div class="setting-control">
            <span class="settings-hint">{{ lastUpdateMessage || text.upToDate }}</span>
          </div>
        </div>
      </article>
    </div>

    <p v-if="saving" class="settings-hint">Saving...</p>
    <p v-if="error" class="danger-text">{{ error }}</p>

    <footer class="settings-footer">
      <p>© 2026 Dr.Tools. {{ text.footer }}</p>
      <p>Author: west ng</p>
    </footer>
  </section>
</template>

<style scoped>
.settings-page {
  display: grid;
  gap: 24px;
}

.settings-group {
  display: grid;
  gap: 6px;
}

.settings-group-title {
  margin: 0;
  padding: 0 14px;
  font-size: 18px;
  color: var(--text-main);
}

.settings-block {
  padding: 0;
  overflow: hidden;
  border: none;
  --surface-bg: var(--bg-card);
}

:global(:root[data-theme='light']) .settings-block,
:global(:root[data-theme='light']) .token-overview-card,
:global(:root[data-theme='light']) .token-card {
  --surface-bg: #f7f7f7;
}

:global(:root[data-theme='dark']) .settings-block,
:global(:root[data-theme='dark']) .token-overview-card,
:global(:root[data-theme='dark']) .token-card {
  --surface-bg: #333332;
}

.settings-label {
  display: flex;
  align-items: center;
  font-size: 15px;
  color: var(--text-main);
}

.setting-row {
  display: grid;
  grid-template-columns: minmax(140px, 1fr) minmax(0, 50%);
  align-items: center;
  gap: 12px;
  margin: 0;
  padding: 10px 14px;
  position: relative;
}

.setting-row::after {
  content: '';
  position: absolute;
  left: 14px;
  right: 14px;
  bottom: 0;
  height: 1px;
  background: var(--divider-soft);
}

.settings-block .setting-row:last-child::after {
  display: none;
}

.setting-control {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  width: 100%;
  min-width: 0;
  min-height: 40px;
}

.setting-row-description {
  grid-template-columns: 1fr;
  padding-top: 6px;
  padding-bottom: 12px;
}

.setting-row-with-description::after {
  display: none;
}

.preview-grid {
  display: grid;
  gap: 12px;
}

.preview-grid-3 {
  grid-template-columns: repeat(3, 100px);
}

.preview-grid-2 {
  grid-template-columns: repeat(2, 100px);
}

.preview-card {
  border: none;
  background: transparent;
  padding: 0;
  display: grid;
  justify-items: center;
  gap: 6px;
}

.preview-card:hover {
  border: none;
  background: transparent;
}

.preview-label {
  font-size: 13px;
  color: var(--text-muted);
}

.preview-art {
  width: 100px;
  height: 56px;
  border-radius: 10px;
  border: 1px solid color-mix(in srgb, var(--stroke-soft) 85%, transparent);
  position: relative;
  overflow: hidden;
}

.preview-theme::before {
  content: '';
  position: absolute;
  left: 8px;
  right: 8px;
  top: 8px;
  height: 8px;
  border-radius: 99px;
  background: rgba(255, 255, 255, 0.7);
}

.preview-theme::after {
  content: '';
  position: absolute;
  left: 8px;
  right: 8px;
  bottom: 8px;
  height: 18px;
  border-radius: 7px;
  background: rgba(255, 255, 255, 0.36);
}

.preview-theme-auto {
  background: linear-gradient(138deg, #ffffff 0%, #f3f5f8 48%, #2d2d2c 52%, #333332 100%);
}

.preview-theme-light {
  background: linear-gradient(138deg, #ffffff 0%, #f3f5f8 100%);
}

.preview-theme-light::before {
  background: #ffffff;
  box-shadow: inset 0 0 0 1px rgba(214, 218, 224, 0.78);
}

.preview-theme-light::after {
  background: #f7f7f7;
  box-shadow: inset 0 0 0 1px rgba(214, 218, 224, 0.92);
}

.preview-theme-dark {
  background: linear-gradient(138deg, #2d2d2c 0%, #333332 100%);
}

.preview-glass-dark.preview-glass-transparent {
  background: linear-gradient(140deg, rgba(45, 45, 44, 0.78) 0%, rgba(51, 51, 50, 0.68) 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.18),
    inset 0 0 0 1px rgba(255, 255, 255, 0.08);
}

.preview-glass-dark.preview-glass-tinted {
  background: linear-gradient(140deg, #2d2d2c 0%, #333332 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.18),
    inset 0 0 0 1px rgba(255, 255, 255, 0.08);
}

.preview-glass-light.preview-glass-transparent {
  background:
    linear-gradient(140deg, rgba(255, 255, 255, 0.66) 0%, rgba(255, 255, 255, 0.54) 100%),
    linear-gradient(140deg, #e9edf2 0%, #dfe5ec 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.92),
    inset 0 0 0 1px rgba(206, 212, 220, 0.96);
}

.preview-glass-light.preview-glass-tinted {
  background: linear-gradient(140deg, #ffffff 0%, #ffffff 100%);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.9),
    inset 0 0 0 1px rgba(214, 218, 224, 0.82);
}

.preview-glass::before {
  content: '';
  position: absolute;
  left: 8px;
  right: 8px;
  top: 8px;
  height: 8px;
  border-radius: 99px;
  background: rgba(255, 255, 255, 0.52);
}

.preview-glass::after {
  content: '';
  position: absolute;
  left: 8px;
  right: 8px;
  bottom: 8px;
  height: 18px;
  border-radius: 7px;
  background: rgba(255, 255, 255, 0.16);
}

.preview-glass-light.preview-glass::before {
  background: rgba(255, 255, 255, 0.96);
  box-shadow: inset 0 0 0 1px rgba(214, 218, 224, 0.72);
}

.preview-glass-light.preview-glass::after {
  background: rgba(228, 232, 238, 0.9);
  box-shadow: inset 0 0 0 1px rgba(214, 218, 224, 0.84);
}

.preview-card.active .preview-art {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 36%, transparent);
}

.preview-card.active .preview-label {
  color: var(--text-main);
  font-weight: 600;
}

.switch-row {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  width: 100%;
}

.switch {
  position: relative;
  width: 44px;
  height: 24px;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.switch span {
  position: absolute;
  inset: 0;
  border-radius: 999px;
  background: #8b90a0;
  transition: 0.2s;
}

.switch span::after {
  content: '';
  position: absolute;
  left: 3px;
  top: 3px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #fff;
  transition: 0.2s;
}

.switch input:checked + span {
  background: var(--accent);
}

.switch input:checked + span::after {
  transform: translateX(20px);
}

.color-row {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  justify-content: flex-end;
  align-items: flex-start;
}

.color-option {
  display: grid;
  justify-items: center;
  gap: 4px;
  min-width: 30px;
}

.color-option-label {
  font-size: 12px;
  color: var(--text-main);
  line-height: 1;
}

.color-dot {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid transparent;
  padding: 0;
}

.color-dot.active {
  border-color: color-mix(in srgb, var(--accent) 80%, white);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 35%, transparent);
}

.row-inline {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
}

.row-inline .input {
  flex: 1;
}

.token-overview-card,
.token-card {
  display: grid;
  gap: 10px;
  padding: 16px;
}

.token-overview-title,
.token-card-title {
  margin: 0;
  color: var(--text-main);
}

.token-grid {
  display: grid;
  gap: 12px;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
}

.token-card-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 12px;
}

.token-status {
  display: inline-flex;
  align-items: center;
  min-height: 28px;
  padding: 0 10px;
  border-radius: 999px;
  font-size: 12px;
  border: 1px solid var(--stroke-soft);
}

.token-status-not_configured,
.token-status-unchecked {
  color: var(--text-muted);
  background: color-mix(in srgb, var(--bg-input) 88%, transparent);
}

.token-status-valid {
  color: #1f8b4c;
  background: color-mix(in srgb, #1f8b4c 14%, transparent);
  border-color: color-mix(in srgb, #1f8b4c 40%, var(--stroke-soft));
}

.token-status-invalid,
.token-status-expired {
  color: var(--danger);
  background: color-mix(in srgb, var(--danger) 14%, transparent);
  border-color: color-mix(in srgb, var(--danger) 40%, var(--stroke-soft));
}

.token-textarea {
  min-height: 132px;
}

.token-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.token-meta {
  display: grid;
  gap: 4px;
  font-size: 12px;
  color: var(--text-muted);
}

.token-guide {
  padding-top: 6px;
  border-top: 1px solid var(--divider-soft);
}

.token-guide-list {
  margin: 8px 0 0;
  padding-left: 18px;
  color: var(--text-main);
  display: grid;
  gap: 6px;
}

.settings-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-muted);
}

.settings-link {
  color: var(--accent);
  text-decoration: none;
}

.settings-link:hover {
  text-decoration: underline;
}

.settings-footer {
  margin-top: 14px;
  text-align: center;
  color: var(--text-muted);
  border-top: 1px solid var(--divider-soft);
  padding-top: 14px;
  font-size: 12px;
}

@media (max-width: 900px) {
  .setting-row {
    grid-template-columns: 1fr;
    gap: 8px;
  }

  .setting-control {
    justify-content: stretch;
  }

  .switch-row,
  .color-row {
    justify-content: flex-start;
  }

  .preview-grid {
    justify-content: flex-start;
  }

  .row-inline {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
