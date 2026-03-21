<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { emit } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useRoute } from 'vue-router';
import { useMessages } from '@/i18n';
import { toErrorMessage } from '@/lib/errors';
import { getRecordingAccountDetail, resolveRecordingAccountProfile, updateRecordingAccount } from '@/modules/recording/api/recording.api';
import { RECORDING_ACCOUNT_CREATED_EVENT } from '@/modules/recording/constants';
import type { RecordingAccountDraft, RecordingPlatform } from '@/modules/recording/types';

interface PlatformOption {
  value: RecordingPlatform;
  label: string;
}

interface RecordingRuleItem {
  key: 'autoStart' | 'retryOnDisconnect' | 'splitRecording' | 'saveSnapshot';
  enabled: boolean;
}

const text = useMessages((messages) => messages.pages.recordingAccountCreate);
const route = useRoute();

const platform = ref<RecordingPlatform>('douyin');
const accountInput = ref('');
const error = ref('');
const submitting = ref(false);
const loading = ref(false);
const recordingRules = ref<RecordingRuleItem[]>([
  { key: 'autoStart', enabled: true },
  { key: 'retryOnDisconnect', enabled: true },
  { key: 'splitRecording', enabled: false },
  { key: 'saveSnapshot', enabled: false },
]);

const platformOptions = computed<PlatformOption[]>(() => [
  { value: 'douyin', label: text.value.platformLabels.douyin },
  { value: 'tiktok', label: text.value.platformLabels.tiktok },
]);

const normalizedAccountInput = computed(() => accountInput.value.trim());
const accountId = computed(() => String(route.params.accountId ?? '').trim());
const isEditMode = computed(() => accountId.value.length > 0);
const titleText = computed(() => (isEditMode.value ? text.value.editTitle : text.value.title));
const submitText = computed(() => {
  if (submitting.value) {
    return text.value.resolving;
  }
  return isEditMode.value ? text.value.confirmEdit : text.value.confirm;
});

async function closeWindow(): Promise<void> {
  await getCurrentWindow().close();
}

function validateAccountUrl(input: string, currentPlatform: RecordingPlatform): string | null {
  let parsed: URL;
  try {
    parsed = new URL(input);
  } catch {
    return text.value.invalidUrl;
  }

  if (!['http:', 'https:'].includes(parsed.protocol) || !parsed.hostname.trim()) {
    return text.value.invalidUrl;
  }

  const hostname = parsed.hostname.toLowerCase();
  if (currentPlatform === 'douyin') {
    if (!hostname.endsWith('douyin.com') && !hostname.endsWith('iesdouyin.com')) {
      return text.value.invalidPlatformUrl;
    }
    return null;
  }

  if (!hostname.endsWith('tiktok.com')) {
    return text.value.invalidPlatformUrl;
  }

  return null;
}

async function submit(): Promise<void> {
  error.value = '';

  if (!platform.value) {
    error.value = text.value.platformRequired;
    return;
  }

  if (!normalizedAccountInput.value) {
    error.value = text.value.accountRequired;
    return;
  }

  const validationError = validateAccountUrl(normalizedAccountInput.value, platform.value);
  if (validationError) {
    error.value = validationError;
    return;
  }

  submitting.value = true;
  try {
    const resolved = await resolveRecordingAccountProfile(platform.value, normalizedAccountInput.value);
    const payload: RecordingAccountDraft = {
      ...resolved,
      autoStart: recordingRules.value.find((rule) => rule.key === 'autoStart')?.enabled ?? true,
      retryOnDisconnect: recordingRules.value.find((rule) => rule.key === 'retryOnDisconnect')?.enabled ?? true,
      splitRecording: recordingRules.value.find((rule) => rule.key === 'splitRecording')?.enabled ?? false,
      saveSnapshot: recordingRules.value.find((rule) => rule.key === 'saveSnapshot')?.enabled ?? false,
    };
    if (isEditMode.value) {
      await updateRecordingAccount(accountId.value, payload);
    } else {
      await emit(RECORDING_ACCOUNT_CREATED_EVENT, payload);
    }
    await closeWindow();
  } catch (submitError) {
    error.value = toErrorMessage(submitError, text.value.submitFailed);
  } finally {
    submitting.value = false;
  }
}

async function loadAccount(): Promise<void> {
  if (!isEditMode.value) {
    return;
  }

  loading.value = true;
  error.value = '';
  try {
    const detail = await getRecordingAccountDetail(accountId.value);
    platform.value = detail.platform;
    accountInput.value = detail.accountInput;
    recordingRules.value = [
      { key: 'autoStart', enabled: detail.autoStart },
      { key: 'retryOnDisconnect', enabled: detail.retryOnDisconnect },
      { key: 'splitRecording', enabled: detail.splitRecording },
      { key: 'saveSnapshot', enabled: detail.saveSnapshot },
    ];
  } catch (loadError) {
    error.value = toErrorMessage(loadError, text.value.loadFailed);
  } finally {
    loading.value = false;
  }
}

watch(
  accountId,
  () => {
    void loadAccount();
  },
  { immediate: true },
);
</script>

<template>
  <section class="recording-account-create-page">
    <transition name="recording-toast">
      <div v-if="submitting" class="recording-loading-toast" role="status" aria-live="polite">
        <span class="recording-loading-toast-spinner" aria-hidden="true"></span>
        <span>{{ text.resolvingToast }}</span>
      </div>
    </transition>

    <div class="recording-account-create-scroll-area">
      <header class="recording-account-create-header">
        <div class="recording-account-create-copy">
          <h2 class="recording-account-create-title">{{ titleText }}</h2>
          <p class="recording-account-create-subtitle">{{ text.subtitle }}</p>
        </div>
      </header>

      <article class="surface recording-account-create-panel">
        <section class="recording-account-create-block">
          <div class="card-head">
            <h3 class="panel-title">{{ text.platform }}</h3>
          </div>
          <div class="platform-grid recording-platform-grid">
            <button
              v-for="item in platformOptions"
              :key="item.value"
              class="platform-card"
              :class="{ active: platform === item.value }"
              type="button"
              @click="platform = item.value"
            >
              <span class="platform-card-name">{{ item.label }}</span>
            </button>
          </div>
        </section>

        <section class="recording-account-create-block">
          <div class="card-head">
            <h3 class="panel-title">{{ text.account }}</h3>
            <p class="settings-hint">{{ text.inputPlaceholder }}</p>
          </div>
          <input
            v-model="accountInput"
            class="input recording-account-input"
            :placeholder="text.inputPlaceholder"
            :disabled="submitting || loading"
            @keydown.enter.prevent="submit"
          />
        </section>

        <section class="recording-account-create-block">
          <div class="card-head">
            <h3 class="panel-title">{{ text.rules }}</h3>
            <p class="settings-hint">{{ text.rulesHint }}</p>
          </div>
          <div class="recording-rules-grid">
            <label v-for="rule in recordingRules" :key="rule.key" class="recording-rule-item">
              <input v-model="rule.enabled" type="checkbox" :disabled="submitting || loading" />
              <div class="recording-rule-copy">
                <span class="recording-rule-title">{{ text[rule.key] }}</span>
                <small class="settings-hint">{{ rule.enabled ? text.yes : text.no }}</small>
              </div>
            </label>
          </div>
        </section>

        <p v-if="loading">{{ text.loading }}</p>
        <p v-if="error" class="danger-text">{{ error }}</p>
      </article>
    </div>

    <div class="surface recording-account-create-actions">
      <button type="button" class="recording-secondary-btn" :disabled="submitting || loading" @click="closeWindow">{{ text.cancel }}</button>
      <button type="button" class="primary-btn recording-submit-btn" :disabled="submitting || loading || !normalizedAccountInput" @click="submit">
        {{ submitText }}
      </button>
    </div>
  </section>
</template>

<style scoped>
.recording-account-create-page {
  position: relative;
  display: grid;
  grid-template-rows: minmax(0, 1fr) auto;
  gap: 14px;
  height: 100%;
  min-height: 0;
}

.recording-account-create-scroll-area {
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-items: stretch;
  justify-content: flex-start;
  gap: 14px;
  padding-top: 2px;
  overflow: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.recording-account-create-scroll-area::-webkit-scrollbar {
  width: 0;
  height: 0;
}

.recording-loading-toast {
  position: absolute;
  top: 0;
  right: 0;
  z-index: 20;
  display: inline-flex;
  align-items: center;
  gap: 10px;
  min-height: 44px;
  padding: 0 14px;
  border: 1px solid color-mix(in srgb, var(--accent) 28%, var(--stroke-soft));
  border-radius: 12px;
  background: color-mix(in srgb, var(--bg-card) 88%, transparent);
  color: var(--text-main);
  box-shadow: 0 10px 24px rgba(20, 21, 26, 0.12);
  backdrop-filter: blur(14px) saturate(125%);
  -webkit-backdrop-filter: blur(14px) saturate(125%);
}

.recording-loading-toast-spinner {
  width: 16px;
  height: 16px;
  border: 2px solid color-mix(in srgb, var(--accent) 18%, transparent);
  border-top-color: var(--accent);
  border-radius: 999px;
  animation: recording-toast-spin 0.7s linear infinite;
}

.recording-toast-enter-active,
.recording-toast-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}

.recording-toast-enter-from,
.recording-toast-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

.recording-account-create-panel {
  background: var(--bg-card);
}

.recording-account-create-header {
  padding: 0 2px;
  align-self: start;
}

.recording-account-create-copy {
  display: block;
}

.recording-account-create-subtitle,
.settings-hint,
.danger-text {
  margin: 0;
}

.recording-account-create-title {
  margin: 0 0 8px;
  font-size: 24px;
  line-height: 1.1;
}

.recording-account-create-subtitle {
  color: var(--text-muted);
  line-height: 1.5;
}

.recording-account-create-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 20px;
  min-height: auto;
}

.recording-account-create-block,
.card-head {
  display: grid;
  gap: 10px;
}

.recording-rules-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
}

.panel-title {
  margin: 0;
  font-size: 18px;
  line-height: 1.2;
}

.platform-grid {
  display: grid;
  gap: 10px;
}

.recording-rule-item {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 52px;
  padding: 12px 14px;
  border: 1px solid var(--stroke-soft);
  border-radius: 14px;
  background: var(--bg-input);
}

.recording-rule-item input {
  margin: 0;
}

.recording-rule-copy {
  display: grid;
  gap: 4px;
  min-width: 0;
}

.recording-rule-title {
  line-height: 1.35;
}

.recording-platform-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.platform-card {
  appearance: none;
  -webkit-appearance: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  width: 100%;
  min-height: 52px;
  padding: 0 14px;
  border-radius: 14px;
  border: 1px solid var(--stroke-soft);
  background: var(--bg-input);
  color: var(--text-main);
  text-align: left;
  transition: border-color 0.2s ease, background-color 0.2s ease, box-shadow 0.2s ease, color 0.2s ease;
}

.platform-card:hover {
  border-color: var(--stroke-soft);
  background: var(--bg-input);
}

.platform-card.active {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 14%, var(--bg-input));
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 20%, transparent);
}

.platform-card-name {
  font-weight: 600;
}

.recording-account-input {
  min-height: 48px;
  padding: 0 14px;
  border-radius: 14px;
}

.recording-account-create-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  position: sticky;
  bottom: 0;
  z-index: 10;
  padding: 14px 20px;
  background: color-mix(in srgb, var(--bg-card) 92%, transparent);
  backdrop-filter: blur(14px) saturate(125%);
  -webkit-backdrop-filter: blur(14px) saturate(125%);
}

.recording-secondary-btn,
.recording-submit-btn {
  min-width: 120px;
  min-height: 42px;
  border-radius: 10px;
}

.recording-secondary-btn {
  background: var(--bg-input);
}

@keyframes recording-toast-spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 760px) {
}
</style>
