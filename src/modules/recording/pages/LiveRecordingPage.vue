<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { storeToRefs } from 'pinia';
import { listen } from '@tauri-apps/api/event';
import { translate, useMessages } from '@/i18n';
import { toErrorMessage } from '@/lib/errors';
import {
  checkRecordingAccounts,
  createRecordingAccount,
  deleteRecordingAccount as deleteRecordingAccountRequest,
  getRecordingSnapshot,
  openRecordingAccountEditWindow,
  openRecordingAccountCreateWindow,
  openRecordingAccountLogsWindow,
  setRecordingAccountEnabled,
} from '@/modules/recording/api/recording.api';
import { RECORDING_ACCOUNT_CREATED_EVENT } from '@/modules/recording/constants';
import { useSettingsStore } from '@/modules/settings/stores/settings.store';
import { openTaskDetailWindow } from '@/modules/tasks/api/tasks.api';
import type { RecordingAccount, RecordingAccountDraft, RecordingRunItem, RecordingSnapshot } from '@/modules/recording/types';

const text = useMessages((messages) => messages.pages.liveRecording);
const settingsStore = useSettingsStore();
const { settings } = storeToRefs(settingsStore);

const openError = ref('');
const accounts = ref<RecordingAccount[]>([]);
const recordingRuns = ref<RecordingRunItem[]>([]);
const pendingToggleAccountIds = ref<string[]>([]);

const displayedRuns = computed(() => [...recordingRuns.value].sort((left, right) => right.updatedAt.localeCompare(left.updatedAt)));
const enabledAccountCount = computed(() => accounts.value.filter((account) => account.enabled).length);
let unlistenAccountCreated: (() => void) | null = null;
let refreshTimer: number | null = null;

function applySnapshot(snapshot: RecordingSnapshot): void {
  accounts.value = snapshot.accounts.map((account) => ({ ...account }));
  recordingRuns.value = snapshot.runs.map((item) => ({ ...item }));
}

async function loadSnapshot(): Promise<void> {
  const snapshot = await getRecordingSnapshot();
  applySnapshot(snapshot);
}

async function handleCreatedAccount(payload: RecordingAccountDraft): Promise<void> {
  openError.value = '';
  try {
    const created = await createRecordingAccount(payload);
    const snapshot = await checkRecordingAccounts([created.id]);
    applySnapshot(snapshot);
  } catch (error) {
    openError.value = toErrorMessage(error, text.value.duplicateAccount);
  }
}

async function openCreateWindow(): Promise<void> {
  openError.value = '';
  try {
    await openRecordingAccountCreateWindow(text.value.createWindowTitle);
  } catch (error) {
    openError.value = toErrorMessage(error, text.value.openCreateWindowFailed);
  }
}

async function toggleAccount(account: RecordingAccount): Promise<void> {
  if (pendingToggleAccountIds.value.includes(account.id)) {
    return;
  }

  openError.value = '';
  pendingToggleAccountIds.value = [...pendingToggleAccountIds.value, account.id];
  try {
    const updated = await setRecordingAccountEnabled(account.id, !account.enabled);
    if (updated.enabled) {
      const snapshot = await checkRecordingAccounts([updated.id]);
      applySnapshot(snapshot);
      return;
    }

    await loadSnapshot();
  } catch (error) {
    openError.value = toErrorMessage(error);
  } finally {
    pendingToggleAccountIds.value = pendingToggleAccountIds.value.filter((id) => id !== account.id);
  }
}

async function deleteAccount(accountId: string): Promise<void> {
  openError.value = '';
  try {
    await deleteRecordingAccountRequest(accountId);
    await loadSnapshot();
  } catch (error) {
    openError.value = toErrorMessage(error);
  }
}

async function openEditWindow(account: RecordingAccount): Promise<void> {
  openError.value = '';
  try {
    await openRecordingAccountEditWindow(account.id, translate(settings.value.locale, 'routes.recordingAccountEdit'));
  } catch (error) {
    openError.value = toErrorMessage(error);
  }
}

async function openLogsWindow(account: RecordingAccount): Promise<void> {
  openError.value = '';
  try {
    await openRecordingAccountLogsWindow(account.id, translate(settings.value.locale, 'routes.recordingAccountLogs'));
  } catch (error) {
    openError.value = toErrorMessage(error);
  }
}

function formatPlatform(value: RecordingAccount['platform']): string {
  return text.value.platformLabels[value] ?? value;
}

function resolveMonitorState(account: RecordingAccount): 'watching' | 'disabled' | 'error' {
  if (account.status === 'error') {
    return 'error';
  }

  if (!account.enabled) {
    return 'disabled';
  }

  return 'watching';
}

function formatMonitorState(account: RecordingAccount): string {
  const state = resolveMonitorState(account);
  return text.value.watchStateMap[state];
}

function resolveStreamState(account: RecordingAccount): 'not-live' | 'live' | 'recording' | 'error' | null {
  if (!account.enabled) {
    return null;
  }

  if (account.status === 'recording') {
    return 'recording';
  }

  if (account.status === 'live') {
    return 'live';
  }

  if (account.status === 'error') {
    return 'error';
  }

  return 'not-live';
}

function formatStreamState(account: RecordingAccount): string | null {
  const state = resolveStreamState(account);
  if (!state) {
    return null;
  }

  if (state === 'not-live') {
    return text.value.streamStateMap.notLive;
  }

  if (state === 'live') {
    return text.value.streamStateMap.live;
  }

  if (state === 'recording') {
    return text.value.streamStateMap.recording;
  }

  return text.value.streamStateMap.error;
}

function resolveAvatarLiveTone(account: RecordingAccount): 'live' | 'recording' | null {
  if (resolveMonitorState(account) !== 'watching') {
    return null;
  }

  const streamState = resolveStreamState(account);
  if (streamState === 'live' || streamState === 'recording') {
    return streamState;
  }

  return null;
}

function getAccountAvatarFallback(account: RecordingAccount): string {
  const label = account.accountName.trim();
  if (label) {
    return label.slice(0, 1).toUpperCase();
  }

  return formatPlatform(account.platform).slice(0, 1).toUpperCase();
}

function buildAccountSummary(account: RecordingAccount): string {
  if (account.lastError) {
    return account.lastError;
  }

  const streamState = resolveStreamState(account);
  if (streamState === 'recording') {
    return '账号正在录制中，可在录制任务中查看进度与结果。';
  }

  if (streamState === 'live') {
    return '账号当前正在直播，系统已保持监控并等待录制流程触发。';
  }

  if (streamState === 'error') {
    return '账号检测出现异常，请查看账号日志或任务明细定位原因。';
  }

  if (!account.enabled) {
    return '账号监控已停用，当前不会进行开播检测。';
  }

  return '账号已接入直播监控，当前未开播。';
}

function getEnabledRuleCount(account: RecordingAccount): number {
  return [account.autoStart, account.retryOnDisconnect, account.splitRecording, account.saveSnapshot].filter(Boolean).length;
}

function formatRuleSummary(account: RecordingAccount): string {
  return text.value.rulesEnabledSummary.replace('{count}', String(getEnabledRuleCount(account)));
}

function isTogglingAccount(accountId: string): boolean {
  return pendingToggleAccountIds.value.includes(accountId);
}

function formatDateTime(value: string | null): string {
  if (!value) {
    return '-';
  }

  const locale = settings.value.locale === 'en-US' ? 'en-US' : 'zh-CN';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(locale, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(date);
}

function formatRunStatus(value: RecordingRunItem['status']): string {
  return text.value.runStatusMap[value] ?? value;
}

function isActiveRun(item: RecordingRunItem): boolean {
  return item.status === 'running';
}

async function openRunDetail(runId: string): Promise<void> {
  try {
    await openTaskDetailWindow(runId, translate(settings.value.locale, 'routes.taskDetail'));
  } catch (error) {
    openError.value = toErrorMessage(error);
  }
}

onMounted(async () => {
  try {
    await loadSnapshot();
    if (accounts.value.some((account) => account.enabled)) {
      const snapshot = await checkRecordingAccounts();
      applySnapshot(snapshot);
    }
  } catch (error) {
    openError.value = toErrorMessage(error);
  }

  unlistenAccountCreated = await listen<RecordingAccountDraft>(RECORDING_ACCOUNT_CREATED_EVENT, (event) => {
    void handleCreatedAccount(event.payload);
  });

  refreshTimer = window.setInterval(() => {
    void loadSnapshot().catch((error) => {
      openError.value = toErrorMessage(error);
    });
  }, 5000);
});

onUnmounted(() => {
  if (unlistenAccountCreated) {
    unlistenAccountCreated();
    unlistenAccountCreated = null;
  }

  if (refreshTimer !== null) {
    window.clearInterval(refreshTimer);
    refreshTimer = null;
  }
});
</script>

<template>
  <section class="recording-page">
    <div class="recording-stack">
      <button type="button" class="recording-launch-entry" @click="openCreateWindow">
        <span class="recording-launch-entry-text">{{ text.openCreateWindow }}</span>
      </button>
      <p v-if="openError" class="danger-text">{{ openError }}</p>

      <article class="surface recording-card">
        <div class="card-head card-head-row card-head-top">
          <div class="card-head-copy">
            <h3 class="panel-title">{{ text.sourceTitle }}</h3>
            <p class="settings-hint">{{ text.sourceHint }}</p>
          </div>
          <span class="recording-chip">{{ enabledAccountCount }} / {{ accounts.length }}</span>
        </div>

        <div v-if="accounts.length === 0" class="recording-empty">
          {{ text.emptyAccounts }}
        </div>
        <div v-else class="recording-account-list">
          <article
            v-for="account in accounts"
            :key="account.id"
            class="recording-account-item"
            :data-monitor-state="resolveMonitorState(account)"
          >
            <div
              class="recording-account-avatar-shell"
              :data-live-tone="resolveAvatarLiveTone(account)"
            >
              <span
                v-if="resolveAvatarLiveTone(account)"
                class="recording-account-avatar-ring recording-account-avatar-ring-outer"
              ></span>
              <span
                v-if="resolveAvatarLiveTone(account)"
                class="recording-account-avatar-ring recording-account-avatar-ring-inner"
              ></span>
              <div class="recording-account-avatar" aria-hidden="true">
                <img
                  v-if="account.accountAvatarUrl"
                  :src="account.accountAvatarUrl"
                  :alt="account.accountName"
                  class="recording-account-avatar-image"
                />
                <span v-else class="recording-account-avatar-fallback">{{ getAccountAvatarFallback(account) }}</span>
              </div>
            </div>

            <div class="recording-account-main">
              <div class="recording-account-header">
                <div class="recording-account-headline">
                  <a :href="account.accountInput" class="recording-account-link" target="_blank" rel="noreferrer">
                    {{ account.accountName }}
                  </a>
                  <div class="recording-account-meta-line">
                    <span>{{ formatPlatform(account.platform) }}</span>
                    <span>{{ text.uid }}: {{ account.accountUid }}</span>
                    <span>{{ text.lastChecked }} {{ formatDateTime(account.lastCheckedAt) }}</span>
                  </div>
                </div>
                <div class="recording-account-tags">
                  <span class="recording-status-pill recording-monitor-pill" :data-monitor-state="resolveMonitorState(account)">
                    {{ formatMonitorState(account) }}
                  </span>
                  <span
                    v-if="resolveStreamState(account)"
                    class="recording-status-pill recording-stream-pill"
                    :data-stream-state="resolveStreamState(account)"
                  >
                    {{ formatStreamState(account) }}
                  </span>
                </div>
              </div>
            </div>

              <p class="recording-account-summary">{{ buildAccountSummary(account) }}</p>

              <div class="recording-account-footer">
                <div class="recording-account-footnote">
                  <span>{{ formatRuleSummary(account) }}</span>
                  <span>{{ text.lastRecorded }} {{ formatDateTime(account.lastRecordedAt) }}</span>
                </div>
                <div class="recording-account-actions">
                <button
                  class="recording-action-btn"
                  type="button"
                  :disabled="isTogglingAccount(account.id)"
                  :aria-busy="isTogglingAccount(account.id)"
                  @click="toggleAccount(account)"
                >
                  <span v-if="isTogglingAccount(account.id)" class="recording-action-spinner" aria-hidden="true"></span>
                  {{ isTogglingAccount(account.id) ? text.processingAction : (account.enabled ? text.disabled : text.enabled) }}
                </button>
                <button class="recording-action-btn" type="button" @click="openEditWindow(account)">
                  {{ text.edit }}
                </button>
                <button class="recording-action-btn" type="button" @click="openLogsWindow(account)">
                  {{ text.viewLogs }}
                </button>
                <button class="recording-action-btn danger" type="button" @click="deleteAccount(account.id)">
                  {{ text.delete }}
                </button>
              </div>
            </div>
          </article>
        </div>
      </article>

      <article class="surface recording-card">
        <div class="card-head card-head-row card-head-top">
          <div class="card-head-copy">
            <h3 class="panel-title">{{ text.runsTitle }}</h3>
          </div>
          <span class="recording-chip">{{ displayedRuns.length }}</span>
        </div>

        <div v-if="displayedRuns.length === 0" class="recording-empty">
          {{ text.runsEmpty }}
        </div>
        <div v-else class="recording-run-list">
          <article v-for="item in displayedRuns" :key="item.id" class="recording-run-item">
            <div class="recording-run-top">
              <div class="recording-run-copy">
                <div class="recording-run-headline">
                  <span class="recording-run-name">{{ item.accountName }}</span>
                  <span class="recording-status-pill recording-stream-pill" :data-stream-state="item.status">
                    {{ formatRunStatus(item.status) }}
                  </span>
                </div>
                <div class="recording-run-meta">
                  <span>{{ item.id }}</span>
                  <span>{{ formatDateTime(item.updatedAt) }}</span>
                </div>
              </div>
              <button class="recording-action-btn" type="button" @click="openRunDetail(item.id)">
                {{ text.viewDetail }}
              </button>
            </div>

            <p v-if="item.outputPath" class="recording-run-path">{{ item.outputPath }}</p>
            <p v-else-if="item.errorText" class="recording-run-path danger-text">{{ item.errorText }}</p>
            <p v-else class="recording-run-path">
              {{ isActiveRun(item) ? text.runActiveHint : text.runPendingHint }}
            </p>
          </article>
        </div>
      </article>
    </div>
  </section>
</template>

<style scoped>
.recording-page {
  display: block;
}

.recording-stack {
  display: grid;
  gap: 14px;
}

.card-head,
.card-head-copy,
.recording-card {
  display: grid;
  gap: 14px;
}

.card-head {
  gap: 4px;
}

.card-head-copy {
  gap: 6px;
  min-width: 0;
}

.card-head-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.card-head-top {
  align-items: center;
}

.panel-title {
  margin: 0;
  font-size: 18px;
  line-height: 1.2;
}

.settings-hint,
.danger-text {
  margin: 0;
}

.recording-card {
  padding: 18px;
  background: var(--bg-card);
}

.recording-launch-entry {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 104px;
  padding: 20px;
  border: 1px dashed var(--stroke-soft);
  border-radius: 14px;
  background: color-mix(in srgb, var(--bg-input) 88%, transparent);
  color: var(--text-main);
  transition: border-color 0.2s ease, background-color 0.2s ease, color 0.2s ease;
}

.recording-launch-entry:hover {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 8%, var(--bg-input));
}

.recording-launch-entry-text {
  font-size: 15px;
  font-weight: 600;
  line-height: 1.4;
  text-align: center;
}

.recording-chip {
  display: inline-flex;
  align-items: center;
  min-height: 28px;
  padding: 0 10px;
  border: 1px solid var(--stroke-soft);
  border-radius: 999px;
  background: var(--bg-input);
  color: var(--text-muted);
  font-size: 12px;
  font-weight: 600;
}

.recording-empty {
  padding: 14px;
  border: 1px dashed var(--stroke-soft);
  border-radius: 14px;
  color: var(--text-muted);
  background: color-mix(in srgb, var(--bg-input) 90%, transparent);
}

.recording-account-list,
.recording-run-list {
  display: grid;
  gap: 8px;
}

.recording-account-list {
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.recording-account-item {
  display: grid;
  grid-template-columns: 72px minmax(0, 1fr);
  gap: 16px;
  align-items: start;
  padding: 18px;
  border: 1px solid var(--stroke-soft);
  border-radius: 14px;
  background: var(--bg-input);
}

.recording-account-main {
  display: grid;
  min-width: 0;
}

.recording-account-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  min-width: 0;
}

.recording-account-avatar-shell {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 72px;
  height: 72px;
  flex: 0 0 72px;
}

.recording-account-avatar {
  position: relative;
  z-index: 2;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 56px;
  height: 56px;
  flex: 0 0 56px;
  overflow: hidden;
  border: 1px solid var(--stroke-soft);
  border-radius: 999px;
  background: color-mix(in srgb, var(--accent) 8%, var(--bg-card));
  color: var(--text-main);
  box-shadow: 0 2px 10px rgba(20, 21, 26, 0.06);
}

.recording-account-avatar-ring {
  position: absolute;
  inset: 8px;
  border-radius: 999px;
  pointer-events: none;
}

.recording-account-avatar-shell[data-live-tone='live'] .recording-account-avatar {
  border-color: color-mix(in srgb, #ff3b7a 50%, #ffffff);
}

.recording-account-avatar-shell[data-live-tone='recording'] .recording-account-avatar {
  border-color: color-mix(in srgb, #ff2f66 62%, #ffffff);
}

.recording-account-avatar-shell[data-live-tone='live'] .recording-account-avatar-ring-outer,
.recording-account-avatar-shell[data-live-tone='recording'] .recording-account-avatar-ring-outer {
  inset: 0;
  border: 2px solid rgba(255, 56, 118, 0.96);
  box-shadow: 0 0 0 3px rgba(255, 56, 118, 0.12);
  animation: recording-live-ring-pulse 1.55s ease-out infinite;
}

.recording-account-avatar-shell[data-live-tone='live'] .recording-account-avatar-ring-inner,
.recording-account-avatar-shell[data-live-tone='recording'] .recording-account-avatar-ring-inner {
  inset: 4px;
  border: 2px solid rgba(255, 92, 148, 0.78);
  animation: recording-live-ring-pulse-soft 1.55s ease-out infinite;
}

.recording-account-avatar-shell[data-live-tone='recording'] .recording-account-avatar-ring-outer {
  border-color: rgba(255, 42, 102, 0.96);
  animation-duration: 1.3s;
}

.recording-account-avatar-shell[data-live-tone='recording'] .recording-account-avatar-ring-inner {
  border-color: rgba(255, 112, 156, 0.8);
  animation-duration: 1.3s;
}

.recording-account-avatar-image {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.recording-account-avatar-fallback {
  font-size: 14px;
  font-weight: 700;
  line-height: 1;
}

.recording-account-headline {
  display: grid;
  gap: 4px;
  flex: 1 1 auto;
  min-width: 0;
}

.recording-account-link {
  display: block;
  color: var(--text-main);
  font-size: 15px;
  font-weight: 700;
  line-height: 1.2;
  text-decoration: none;
  white-space: nowrap;
}

.recording-account-link:hover {
  color: var(--accent);
}

.recording-account-link:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--accent) 48%, transparent);
  outline-offset: 3px;
  border-radius: 8px;
}

.recording-account-link,
 .recording-account-meta-line,
 .recording-account-summary {
  overflow: hidden;
  text-overflow: ellipsis;
}

.recording-account-meta-line {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  white-space: nowrap;
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.5;
}

.recording-account-meta-line span {
  display: inline-flex;
  align-items: center;
}

.recording-account-meta-line span:not(:last-child)::after {
  content: '';
  width: 3px;
  height: 3px;
  margin-left: 10px;
  border-radius: 999px;
  background: currentColor;
}

.recording-account-tags {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  flex: 0 0 auto;
  flex-wrap: nowrap;
  justify-content: flex-end;
  white-space: nowrap;
}

.recording-platform-badge,
.recording-status-pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 28px;
  padding: 0 10px;
  border: 1px solid var(--stroke-soft);
  border-radius: 999px;
  background: var(--bg-card);
  font-size: 12px;
  font-weight: 600;
  white-space: nowrap;
}

.recording-platform-badge {
  color: var(--text-muted);
}

.recording-monitor-pill[data-monitor-state='watching'] {
  color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 35%, var(--stroke-soft));
  background: color-mix(in srgb, var(--accent) 10%, var(--bg-card));
}

.recording-monitor-pill[data-monitor-state='disabled'] {
  color: var(--text-muted);
}

.recording-monitor-pill[data-monitor-state='error'] {
  color: var(--danger);
  border-color: color-mix(in srgb, var(--danger) 40%, var(--stroke-soft));
  background: color-mix(in srgb, var(--danger) 8%, var(--bg-card));
}

.recording-stream-pill[data-stream-state='not-live'] {
  color: var(--text-muted);
  border-color: var(--stroke-soft);
  background: color-mix(in srgb, var(--bg-card) 82%, transparent);
}

.recording-stream-pill[data-stream-state='live'] {
  color: #d28b12;
  border-color: color-mix(in srgb, #d28b12 42%, var(--stroke-soft));
  background: color-mix(in srgb, #d28b12 8%, var(--bg-card));
}

.recording-stream-pill[data-stream-state='recording'] {
  color: #2aa36b;
  border-color: color-mix(in srgb, #2aa36b 38%, var(--stroke-soft));
  background: color-mix(in srgb, #2aa36b 8%, var(--bg-card));
}

.recording-stream-pill[data-stream-state='success'] {
  color: #2aa36b;
  border-color: color-mix(in srgb, #2aa36b 38%, var(--stroke-soft));
  background: color-mix(in srgb, #2aa36b 8%, var(--bg-card));
}

.recording-stream-pill[data-stream-state='failed'] {
  color: var(--danger);
  border-color: color-mix(in srgb, var(--danger) 40%, var(--stroke-soft));
  background: color-mix(in srgb, var(--danger) 8%, var(--bg-card));
}

.recording-stream-pill[data-stream-state='queued'] {
  color: var(--text-muted);
  border-color: var(--stroke-soft);
  background: color-mix(in srgb, var(--bg-card) 82%, transparent);
}

.recording-stream-pill[data-stream-state='error'] {
  color: var(--danger);
  border-color: color-mix(in srgb, var(--danger) 40%, var(--stroke-soft));
  background: color-mix(in srgb, var(--danger) 8%, var(--bg-card));
}

.recording-account-summary {
  margin: 0;
  grid-column: 1 / -1;
  color: var(--text-muted);
  font-size: 13px;
  line-height: 1.5;
  white-space: nowrap;
}

.recording-account-footer {
  display: flex;
  grid-column: 1 / -1;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-width: 0;
}

.recording-account-footnote {
  min-width: 0;
  display: inline-flex;
  align-items: center;
  gap: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.recording-account-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  justify-content: flex-end;
  min-width: 0;
}

.recording-action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 32px;
  padding: 0 10px;
  border-radius: 999px;
  background: var(--bg-card);
  white-space: nowrap;
  font-size: 12px;
  cursor: pointer;
}

.recording-action-btn:disabled {
  cursor: not-allowed;
  opacity: 0.72;
}

.recording-action-btn.danger {
  color: var(--danger);
  border-color: color-mix(in srgb, var(--danger) 32%, var(--stroke-soft));
  background: color-mix(in srgb, var(--danger) 8%, var(--bg-card));
}

.recording-action-spinner {
  width: 12px;
  height: 12px;
  border: 1.5px solid color-mix(in srgb, var(--text-main) 18%, transparent);
  border-top-color: currentColor;
  border-radius: 999px;
  animation: recording-action-spin 0.72s linear infinite;
}

.recording-account-footnote {
  color: var(--text-muted);
  font-size: 12px;
}

.recording-run-item {
  display: grid;
  gap: 8px;
  padding: 12px 14px;
  border: 1px solid var(--stroke-soft);
  border-radius: 12px;
  background: var(--bg-input);
}

.recording-run-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.recording-run-copy {
  display: grid;
  gap: 4px;
  min-width: 0;
  flex: 1 1 auto;
}

.recording-run-headline,
.recording-run-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.recording-run-name,
.recording-run-path,
.recording-run-meta span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recording-run-name {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-main);
}

.recording-run-meta {
  color: var(--text-muted);
  font-size: 12px;
}

.recording-run-meta span:not(:last-child)::after {
  content: '';
  width: 3px;
  height: 3px;
  margin-left: 8px;
  border-radius: 999px;
  background: currentColor;
}

.recording-run-path {
  margin: 0;
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.4;
}

@media (max-width: 980px) {
  .recording-account-list {
    grid-template-columns: 1fr;
  }

  .recording-account-footer {
    flex-direction: column;
    align-items: flex-start;
  }

  .recording-account-tags,
  .recording-account-actions {
    justify-content: flex-start;
  }
}

@media (max-width: 760px) {
  .card-head-row,
  .recording-account-header,
  .recording-run-top {
    flex-direction: column;
    align-items: stretch;
  }
}

@keyframes recording-live-ring-pulse {
  0% {
    opacity: 0.95;
    transform: scale(0.9);
  }

  70% {
    opacity: 0;
    transform: scale(1.08);
  }

  100% {
    opacity: 0;
    transform: scale(1.12);
  }
}

@keyframes recording-live-ring-pulse-soft {
  0% {
    opacity: 0.9;
    transform: scale(0.96);
  }

  65% {
    opacity: 0.14;
    transform: scale(1.02);
  }

  100% {
    opacity: 0;
    transform: scale(1.05);
  }
}

@keyframes recording-action-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
