<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import { storeToRefs } from 'pinia';
import { getDownloadBatchDetail, openTaskDetailWindow } from '@/modules/tasks/api/tasks.api';
import { useSettingsStore } from '@/modules/settings/stores/settings.store';
import { translate, useMessages } from '@/i18n';
import { toErrorMessage } from '@/lib/errors';
import type { DownloadBatchDetail, DownloadBatchTaskItem } from '@/modules/tasks/types';

type StatusTone = 'neutral' | 'info' | 'success' | 'danger' | 'warning';
type OverviewItem = {
  label: string;
  value: string;
  mono?: boolean;
};

const route = useRoute();
const settingsStore = useSettingsStore();
const { settings } = storeToRefs(settingsStore);

const loading = ref(false);
const error = ref('');
const openError = ref('');
const detail = ref<DownloadBatchDetail | null>(null);
let loadRequestId = 0;

const batchId = computed(() => String(route.params.batchId ?? '').trim());
const text = useMessages((messages) => messages.pages.batchDetail);

const statusTone = computed<StatusTone>(() => {
  switch (detail.value?.status) {
    case 'running':
      return 'info';
    case 'success':
      return 'success';
    case 'failed':
      return 'danger';
    case 'partial':
      return 'warning';
    default:
      return 'neutral';
  }
});

const statusLabel = computed(() => {
  const status = detail.value?.status ?? 'queued';
  return text.value.statusMap[status as keyof typeof text.value.statusMap] ?? status;
});

const overviewItems = computed<OverviewItem[]>(() => {
  if (!detail.value) {
    return [];
  }

  return [
    { label: text.value.batchId, value: detail.value.id, mono: true },
    { label: text.value.platform, value: formatPlatform(detail.value.platform) },
    { label: text.value.status, value: statusLabel.value },
    { label: text.value.total, value: String(detail.value.totalCount) },
    { label: text.value.success, value: String(detail.value.successCount) },
    { label: text.value.failed, value: String(detail.value.failedCount) },
    { label: text.value.running, value: String(detail.value.runningCount) },
    { label: text.value.created, value: formatDateTime(detail.value.createdAt) },
    { label: text.value.updated, value: formatDateTime(detail.value.updatedAt) },
    { label: text.value.completed, value: detail.value.completedAt ? formatDateTime(detail.value.completedAt) : '-' }
  ];
});

watch(
  batchId,
  async () => {
    await settingsStore.ensureLoaded();
    await loadDetail();
  },
  { immediate: true }
);

async function loadDetail(): Promise<void> {
  if (!batchId.value) {
    detail.value = null;
    error.value = text.value.missingBatchId;
    return;
  }

  const requestId = ++loadRequestId;
  loading.value = true;
  error.value = '';
  try {
    const nextDetail = await getDownloadBatchDetail(batchId.value);
    if (requestId !== loadRequestId) {
      return;
    }
    detail.value = nextDetail;
  } catch (loadError) {
    if (requestId === loadRequestId) {
      detail.value = null;
      error.value = toErrorMessage(loadError, text.value.loadFailed);
    }
  } finally {
    if (requestId === loadRequestId) {
      loading.value = false;
    }
  }
}

async function openTaskDetail(item: DownloadBatchTaskItem): Promise<void> {
  openError.value = '';
  try {
    await openTaskDetailWindow(item.id, translate(settings.value.locale, 'routes.taskDetail'));
  } catch (openDetailError) {
    openError.value = toErrorMessage(openDetailError, text.value.openWindowFailed);
  }
}

function formatDateTime(value: string): string {
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
    hour12: false
  }).format(date);
}

function formatPlatform(value: string): string {
  if (value in text.value.platformLabels) {
    return text.value.platformLabels[value as keyof typeof text.value.platformLabels];
  }
  return value || '-';
}

function formatTaskType(value: string): string {
  return text.value.taskTypeMap[value as keyof typeof text.value.taskTypeMap] ?? value;
}

function formatTaskStatus(value: string): string {
  return text.value.statusMap[value as keyof typeof text.value.statusMap] ?? value;
}

function formatOptionalValue(value?: string | null): string {
  const normalized = value?.trim();
  return normalized ? normalized : '-';
}
</script>

<template>
  <section class="batch-detail-page">
    <header class="surface batch-detail-header">
      <div class="batch-detail-header-copy">
        <p class="batch-detail-caption">{{ detail ? formatPlatform(detail.platform) : '-' }}</p>
        <h2 class="batch-detail-title">{{ text.title }}</h2>
        <p class="batch-detail-subtitle">{{ detail ? `${text.batchId}: ${detail.id}` : text.subtitle }}</p>
      </div>
      <span class="batch-status-badge" :data-tone="statusTone">{{ statusLabel }}</span>
    </header>

    <p v-if="loading">{{ text.loading }}</p>
    <p v-else-if="error" class="danger-text">{{ error }}</p>
    <p v-else-if="openError" class="danger-text">{{ openError }}</p>
    <div v-else-if="detail" class="batch-detail-scroll">
      <article class="surface batch-detail-panel">
        <header class="batch-detail-panel-head">
          <h3>{{ text.overview }}</h3>
        </header>
        <dl class="batch-overview-grid">
          <div v-for="item in overviewItems" :key="item.label" class="batch-overview-item">
            <dt>{{ item.label }}</dt>
            <dd :class="{ mono: item.mono }">{{ item.value }}</dd>
          </div>
        </dl>
      </article>

      <article class="surface batch-detail-panel">
        <header class="batch-detail-panel-head">
          <h3>{{ text.tasks }}</h3>
        </header>
        <p v-if="detail.tasks.length === 0" class="settings-hint">{{ text.emptyTasks }}</p>
        <div v-else class="batch-task-list">
          <article v-for="item in detail.tasks" :key="item.id" class="batch-task-item">
            <div class="batch-task-main">
              <div class="batch-task-top">
                <span class="batch-task-type">{{ formatTaskType(item.taskType) }}</span>
                <span class="batch-task-status" :data-status="item.status">{{ formatTaskStatus(item.status) }}</span>
              </div>
              <dl class="batch-task-extra-grid">
                <div class="batch-task-extra-item">
                  <dt>{{ text.authorName }}</dt>
                  <dd>{{ formatOptionalValue(item.authorName) }}</dd>
                </div>
                <div class="batch-task-extra-item">
                  <dt>{{ text.authorUid }}</dt>
                  <dd class="mono">{{ formatOptionalValue(item.authorUid) }}</dd>
                </div>
              </dl>
              <p class="batch-task-id mono">{{ item.id }}</p>
              <p class="batch-task-url mono">{{ formatOptionalValue(item.sourceUrl) }}</p>
              <p v-if="item.errorText" class="danger-text batch-task-error">{{ item.errorText }}</p>
              <p class="batch-task-meta">{{ formatDateTime(item.updatedAt) }}</p>
            </div>
            <button class="detail-link-btn" type="button" @click="openTaskDetail(item)">{{ text.viewTask }}</button>
          </article>
        </div>
      </article>
    </div>
  </section>
</template>

<style scoped>
.batch-detail-page {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: 12px;
  height: 100%;
  min-height: 0;
}

.batch-detail-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 16px 18px;
  background: var(--bg-card);
}

.batch-detail-header-copy {
  min-width: 0;
}

.batch-detail-caption {
  margin: 0 0 6px;
  color: var(--text-muted);
  font-size: 12px;
}

.batch-detail-title {
  margin: 0;
  color: var(--text-main);
  font-size: 22px;
  line-height: 1.2;
}

.batch-detail-subtitle {
  margin: 8px 0 0;
  color: var(--text-muted);
  font-size: 13px;
  line-height: 1.5;
  word-break: break-all;
}

.batch-status-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  min-width: 88px;
  padding: 7px 12px;
  border: 1px solid var(--stroke-soft);
  border-radius: 999px;
  background: var(--bg-input);
  color: var(--text-main);
  font-size: 12px;
  font-weight: 600;
}

.batch-status-badge[data-tone='info'] {
  color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 38%, var(--stroke-soft));
}

.batch-status-badge[data-tone='success'] {
  color: #2aa36b;
  border-color: color-mix(in srgb, #2aa36b 42%, var(--stroke-soft));
}

.batch-status-badge[data-tone='danger'] {
  color: var(--danger);
  border-color: color-mix(in srgb, var(--danger) 44%, var(--stroke-soft));
}

.batch-status-badge[data-tone='warning'] {
  color: #d28b12;
  border-color: color-mix(in srgb, #d28b12 44%, var(--stroke-soft));
}

.batch-detail-scroll {
  display: grid;
  gap: 12px;
  overflow: auto;
  min-height: 0;
  padding-right: 2px;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.batch-detail-scroll::-webkit-scrollbar {
  width: 0;
  height: 0;
}

.batch-detail-panel {
  display: grid;
  gap: 12px;
  padding: 16px;
  background: var(--bg-card);
}

.batch-detail-panel-head h3 {
  margin: 0;
  color: var(--text-main);
  font-size: 15px;
}

.batch-overview-grid {
  display: grid;
  gap: 10px;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  margin: 0;
}

.batch-overview-item {
  display: grid;
  gap: 6px;
  padding: 12px;
  border: 1px solid var(--divider-soft);
  border-radius: 10px;
  background: var(--bg-input);
}

.batch-overview-item dt {
  color: var(--text-muted);
  font-size: 12px;
}

.batch-overview-item dd {
  margin: 0;
  color: var(--text-main);
  font-size: 14px;
  line-height: 1.55;
}

.batch-task-list {
  display: grid;
  gap: 10px;
}

.batch-task-item {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  padding: 12px;
  border: 1px solid var(--divider-soft);
  border-radius: 10px;
  background: var(--bg-input);
}

.batch-task-main {
  min-width: 0;
  flex: 1;
}

.batch-task-top {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  margin-bottom: 8px;
}

.batch-task-type {
  color: var(--text-main);
  font-size: 13px;
  font-weight: 600;
}

.batch-task-status {
  display: inline-flex;
  align-items: center;
  min-height: 24px;
  padding: 0 9px;
  border: 1px solid var(--stroke-soft);
  border-radius: 999px;
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 700;
}

.batch-task-status[data-status='running'] {
  color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 36%, var(--stroke-soft));
}

.batch-task-status[data-status='success'] {
  color: #2aa36b;
  border-color: color-mix(in srgb, #2aa36b 40%, var(--stroke-soft));
}

.batch-task-status[data-status='failed'] {
  color: var(--danger);
  border-color: color-mix(in srgb, var(--danger) 42%, var(--stroke-soft));
}

.batch-task-id,
.batch-task-url,
.batch-task-meta,
.batch-task-error {
  margin: 0;
}

.batch-task-extra-grid {
  display: grid;
  gap: 8px;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  margin: 0 0 8px;
}

.batch-task-extra-item {
  display: grid;
  gap: 4px;
  min-width: 0;
}

.batch-task-extra-item dt {
  color: var(--text-muted);
  font-size: 12px;
}

.batch-task-extra-item dd {
  margin: 0;
  color: var(--text-main);
  line-height: 1.5;
  word-break: break-all;
}

.batch-task-id,
.batch-task-url {
  color: var(--text-main);
  line-height: 1.55;
  word-break: break-all;
}

.batch-task-url {
  margin-top: 6px;
}

.batch-task-error {
  margin-top: 6px;
  white-space: pre-wrap;
  word-break: break-word;
}

.batch-task-meta {
  margin-top: 8px;
  color: var(--text-muted);
  font-size: 12px;
}

.detail-link-btn {
  flex-shrink: 0;
  padding: 6px 10px;
}

.mono {
  font-family: 'SF Mono', 'Menlo', monospace;
}

@media (max-width: 1040px) {
  .batch-overview-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 760px) {
  .batch-detail-header,
  .batch-task-item {
    flex-direction: column;
  }

  .batch-overview-grid {
    grid-template-columns: 1fr;
  }

  .batch-task-extra-grid {
    grid-template-columns: 1fr;
  }

  .detail-link-btn {
    width: 100%;
  }

}
</style>
