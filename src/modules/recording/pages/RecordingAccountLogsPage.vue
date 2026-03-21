<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import { storeToRefs } from 'pinia';
import { getRecordingAccountDetail, getRecordingAccountLogs } from '@/modules/recording/api/recording.api';
import { useSettingsStore } from '@/modules/settings/stores/settings.store';
import { useMessages } from '@/i18n';
import { toErrorMessage } from '@/lib/errors';
import type { RecordingAccount, RecordingLogEntry } from '@/modules/recording/types';

const route = useRoute();
const settingsStore = useSettingsStore();
const { settings } = storeToRefs(settingsStore);

const loading = ref(false);
const error = ref('');
const account = ref<RecordingAccount | null>(null);
const logs = ref<RecordingLogEntry[]>([]);
let loadRequestId = 0;

const accountId = computed(() => String(route.params.accountId ?? '').trim());
const text = useMessages((messages) => messages.pages.recordingAccountLogs);
const displayedLogs = computed(() => [...logs.value].sort((left, right) => right.ts.localeCompare(left.ts)));

watch(
  accountId,
  async () => {
    await settingsStore.ensureLoaded();
    await loadData();
  },
  { immediate: true },
);

async function loadData(): Promise<void> {
  if (!accountId.value) {
    account.value = null;
    logs.value = [];
    error.value = text.value.loadFailed;
    return;
  }

  const requestId = ++loadRequestId;
  loading.value = true;
  error.value = '';
  try {
    const [detail, accountLogs] = await Promise.all([
      getRecordingAccountDetail(accountId.value),
      getRecordingAccountLogs(accountId.value, 200),
    ]);
    if (requestId !== loadRequestId) {
      return;
    }
    account.value = detail;
    logs.value = accountLogs.map((entry, index) => ({
      ...entry,
      id: `${entry.accountId ?? accountId.value}-${entry.ts}-${index}`,
    }));
  } catch (loadError) {
    if (requestId === loadRequestId) {
      account.value = null;
      logs.value = [];
      error.value = toErrorMessage(loadError, text.value.loadFailed);
    }
  } finally {
    if (requestId === loadRequestId) {
      loading.value = false;
    }
  }
}

function formatDateTime(value: string | null): string {
  if (!value) {
    return '-';
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(settings.value.locale === 'en-US' ? 'en-US' : 'zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  }).format(date);
}

function formatPlatform(value?: string): string {
  if (!value) {
    return '-';
  }
  return text.value.platformLabels[value as keyof typeof text.value.platformLabels] ?? value;
}

function formatLevel(value: RecordingLogEntry['level']): string {
  return text.value.statusMap[value] ?? value;
}
</script>

<template>
  <section class="recording-account-logs-page">
    <header class="surface recording-account-logs-header">
      <div class="recording-account-logs-copy">
        <h2 class="recording-account-logs-title">{{ text.title }}</h2>
        <p class="recording-account-logs-subtitle">{{ account ? account.accountName : text.subtitle }}</p>
      </div>
    </header>

    <p v-if="loading">{{ text.loading }}</p>
    <p v-else-if="error" class="danger-text">{{ error }}</p>
    <div v-else class="recording-account-logs-scroll">
      <article v-if="account" class="surface recording-account-logs-panel">
        <dl class="recording-account-logs-overview">
          <div class="recording-account-logs-overview-item">
            <dt>{{ text.accountId }}</dt>
            <dd class="mono">{{ account.id }}</dd>
          </div>
          <div class="recording-account-logs-overview-item">
            <dt>{{ text.platform }}</dt>
            <dd>{{ formatPlatform(account.platform) }}</dd>
          </div>
          <div class="recording-account-logs-overview-item">
            <dt>{{ text.uid }}</dt>
            <dd class="mono">{{ account.accountUid }}</dd>
          </div>
          <div class="recording-account-logs-overview-item">
            <dt>{{ text.updated }}</dt>
            <dd>{{ formatDateTime(account.updatedAt) }}</dd>
          </div>
        </dl>
      </article>

      <article class="surface recording-account-logs-panel">
        <p v-if="displayedLogs.length === 0" class="settings-hint">{{ text.empty }}</p>
        <div v-else class="recording-account-log-list">
          <div v-for="entry in displayedLogs" :key="entry.id" class="recording-account-log-item">
            <div class="recording-account-log-head">
              <span class="recording-account-log-time">{{ formatDateTime(entry.ts) }}</span>
              <span class="recording-account-log-level" :data-level="entry.level">{{ formatLevel(entry.level) }}</span>
            </div>
            <p class="recording-account-log-message">{{ entry.message }}</p>
          </div>
        </div>
      </article>
    </div>
  </section>
</template>

<style scoped>
.recording-account-logs-page {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: 12px;
  height: 100%;
  min-height: 0;
}

.recording-account-logs-header,
.recording-account-logs-panel {
  background: var(--bg-card);
}

.recording-account-logs-header {
  padding: 18px;
}

.recording-account-logs-copy {
  display: grid;
  gap: 6px;
}

.recording-account-logs-title,
.recording-account-logs-subtitle,
.recording-account-log-message {
  margin: 0;
}

.recording-account-logs-title {
  font-size: 22px;
  line-height: 1.1;
}

.recording-account-logs-subtitle {
  color: var(--text-muted);
}

.recording-account-logs-scroll {
  display: grid;
  gap: 12px;
  min-height: 0;
  overflow: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.recording-account-logs-scroll::-webkit-scrollbar {
  width: 0;
  height: 0;
}

.recording-account-logs-panel {
  padding: 16px;
}

.recording-account-logs-overview {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
  margin: 0;
}

.recording-account-logs-overview-item {
  display: grid;
  gap: 4px;
}

.recording-account-logs-overview-item dt {
  color: var(--text-muted);
  font-size: 12px;
}

.recording-account-logs-overview-item dd {
  margin: 0;
  color: var(--text-main);
}

.recording-account-log-list {
  display: grid;
  gap: 8px;
}

.recording-account-log-item {
  display: grid;
  gap: 6px;
  padding: 10px 12px;
  border: 1px solid var(--stroke-soft);
  border-radius: 12px;
  background: var(--bg-input);
}

.recording-account-log-head {
  display: flex;
  align-items: center;
  gap: 8px;
}

.recording-account-log-time {
  color: var(--text-muted);
  font-size: 12px;
}

.recording-account-log-level {
  font-size: 12px;
  font-weight: 700;
}

.recording-account-log-level[data-level='success'] {
  color: #2aa36b;
}

.recording-account-log-level[data-level='warning'] {
  color: #d28b12;
}

.recording-account-log-level[data-level='error'] {
  color: var(--danger);
}

.recording-account-log-message {
  color: var(--text-main);
  line-height: 1.45;
  word-break: break-word;
}

@media (max-width: 760px) {
  .recording-account-logs-overview {
    grid-template-columns: 1fr;
  }
}
</style>
