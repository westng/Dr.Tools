<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute } from 'vue-router';
import { storeToRefs } from 'pinia';
import { getTaskDetail } from '@/modules/tasks/api/tasks.api';
import { useSettingsStore } from '@/modules/settings/stores/settings.store';
import { useMessages } from '@/i18n';
import { toErrorMessage } from '@/lib/errors';
import type { TaskRecordDetail } from '@/modules/tasks/types';

type StatusTone = 'neutral' | 'info' | 'success' | 'danger';
type InfoItem = {
  label: string;
  value: string;
  mono?: boolean;
  multiline?: boolean;
  tone?: 'default' | 'danger';
};

const route = useRoute();
const settingsStore = useSettingsStore();
const { settings } = storeToRefs(settingsStore);

const loading = ref(false);
const error = ref('');
const detail = ref<TaskRecordDetail | null>(null);
let loadRequestId = 0;

const taskId = computed(() => String(route.params.taskId ?? '').trim());
const inputData = computed(() => detail.value?.input ?? null);
const outputData = computed(() => detail.value?.output ?? null);
const logs = computed(() => detail.value?.logs ?? []);
const text = useMessages((messages) => messages.pages.taskDetail);

const statusTone = computed<StatusTone>(() => {
  switch (detail.value?.status) {
    case 'running':
      return 'info';
    case 'success':
      return 'success';
    case 'failed':
      return 'danger';
    default:
      return 'neutral';
  }
});

const statusLabel = computed(() => {
  const status = detail.value?.status ?? 'queued';
  return text.value.statusMap[status as keyof typeof text.value.statusMap] ?? status;
});

const taskTypeLabel = computed(() => {
  const taskType = detail.value?.taskType ?? '';
  return (
    text.value.taskTypeMap[taskType as keyof typeof text.value.taskTypeMap] ??
    (taskType || text.value.unknownTask)
  );
});

const headerDescription = computed(() => {
  if (!detail.value) {
    return text.value.subtitle;
  }

  return `${text.value.taskId}: ${detail.value.id}`;
});

const overviewItems = computed<InfoItem[]>(() => {
  if (!detail.value) {
    return [];
  }

  return [
    { label: text.value.taskId, value: detail.value.id, mono: true },
    { label: text.value.type, value: taskTypeLabel.value },
    { label: text.value.status, value: statusLabel.value },
    { label: text.value.totalLogs, value: String(logs.value.length) },
    { label: text.value.created, value: formatDateTime(detail.value.createdAt) },
    { label: text.value.updated, value: formatDateTime(detail.value.updatedAt) }
  ];
});

const sourceItems = computed<InfoItem[]>(() => {
  const sourceUrl = readString(inputData.value, 'sourceUrl');
  const platform = readPlatform();
  const downloadCover = readBoolean(inputData.value, 'downloadCover');

  return [
    { label: text.value.platform, value: platform || '-' },
    { label: text.value.downloadCover, value: downloadCover ? text.value.yes : text.value.no },
    {
      label: text.value.sourceUrl,
      value: sourceUrl || text.value.emptyData,
      mono: true,
      multiline: true
    }
  ];
});

const resultItems = computed<InfoItem[]>(() => {
  const items: InfoItem[] = [];
  const outputPath = readString(outputData.value, 'outputPath');
  const coverPath = readString(outputData.value, 'coverPath');
  const metadataPath = readString(outputData.value, 'metadataPath');
  const message = readString(outputData.value, 'message');

  if (outputPath) {
    items.push({ label: text.value.outputPath, value: outputPath, mono: true, multiline: true });
  }
  if (coverPath) {
    items.push({ label: text.value.coverPath, value: coverPath, mono: true, multiline: true });
  }
  if (metadataPath) {
    items.push({ label: text.value.metadataPath, value: metadataPath, mono: true, multiline: true });
  }
  if (message) {
    items.push({ label: text.value.message, value: message, multiline: true });
  }
  if (detail.value?.errorText) {
    items.push({ label: text.value.error, value: detail.value.errorText, multiline: true, tone: 'danger' });
  }

  if (items.length === 0) {
    items.push({ label: text.value.message, value: text.value.emptyData });
  }

  return items;
});

const formattedInput = computed(() => formatJson(inputData.value, text.value.emptyData));
const formattedOutput = computed(() => formatJson(outputData.value, text.value.emptyData));

watch(
  taskId,
  async () => {
    await settingsStore.ensureLoaded();
    await loadDetail();
  },
  { immediate: true }
);

async function loadDetail(): Promise<void> {
  if (!taskId.value) {
    detail.value = null;
    error.value = text.value.missingTaskId;
    return;
  }

  const requestId = ++loadRequestId;
  loading.value = true;
  error.value = '';
  try {
    const nextDetail = await getTaskDetail(taskId.value);
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

function formatJson(value: Record<string, unknown> | null, emptyText: string): string {
  if (!value || Object.keys(value).length === 0) {
    return emptyText;
  }
  return JSON.stringify(value, null, 2);
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

function readString(source: Record<string, unknown> | null, key: string): string {
  const value = source?.[key];
  return typeof value === 'string' ? value : '';
}

function readBoolean(source: Record<string, unknown> | null, key: string): boolean {
  return source?.[key] === true;
}

function readPlatform(): string {
  const outputPlatform = readString(outputData.value, 'platform');
  const inputPlatform = readString(inputData.value, 'platform');
  const platform = outputPlatform || inputPlatform;
  if (platform && platform in text.value.platformLabels) {
    return text.value.platformLabels[platform as keyof typeof text.value.platformLabels];
  }
  return platform;
}
</script>

<template>
  <section class="task-detail-page">
    <header class="surface task-detail-header">
      <div class="task-detail-header-copy">
        <p class="task-detail-caption">{{ taskTypeLabel }}</p>
        <h2 class="task-detail-title">{{ text.title }}</h2>
        <p class="task-detail-subtitle">{{ headerDescription }}</p>
      </div>
      <span class="task-status-badge" :data-tone="statusTone">{{ statusLabel }}</span>
    </header>

    <p v-if="loading">{{ text.loading }}</p>
    <p v-else-if="error" class="danger-text">{{ error }}</p>
    <div v-else-if="detail" class="task-detail-scroll">
      <article class="surface task-detail-panel">
        <header class="task-detail-panel-head">
          <h3>{{ text.overview }}</h3>
        </header>
        <dl class="task-overview-grid">
          <div v-for="item in overviewItems" :key="item.label" class="task-overview-item">
            <dt>{{ item.label }}</dt>
            <dd :class="{ mono: item.mono }">{{ item.value }}</dd>
          </div>
        </dl>
      </article>

      <article class="surface task-detail-panel">
        <header class="task-detail-panel-head">
          <h3>{{ text.source }}</h3>
        </header>
        <dl class="task-info-stack">
          <div v-for="item in sourceItems" :key="item.label" class="task-info-item">
            <dt>{{ item.label }}</dt>
            <dd :class="{ mono: item.mono, multiline: item.multiline }">{{ item.value }}</dd>
          </div>
        </dl>
      </article>

      <article class="surface task-detail-panel">
        <header class="task-detail-panel-head">
          <h3>{{ text.result }}</h3>
        </header>
        <dl class="task-info-stack">
          <div v-for="item in resultItems" :key="item.label" class="task-info-item">
            <dt>{{ item.label }}</dt>
            <dd :class="[{ mono: item.mono, multiline: item.multiline }, item.tone === 'danger' ? 'danger-text' : '']">
              {{ item.value }}
            </dd>
          </div>
        </dl>
      </article>

      <article class="surface task-detail-panel">
        <header class="task-detail-panel-head">
          <h3>{{ text.logs }}</h3>
        </header>
        <p v-if="logs.length === 0" class="settings-hint">{{ text.emptyLogs }}</p>
        <div v-else class="task-log-list">
          <article
            v-for="entry in logs"
            :key="`${entry.taskId}-${entry.ts}-${entry.level}-${entry.message}`"
            class="task-log-item"
          >
            <div class="task-log-item-head">
              <span class="task-log-time">{{ entry.ts }}</span>
              <span class="task-log-level">{{ entry.level.toUpperCase() }}</span>
            </div>
            <p>{{ entry.message }}</p>
          </article>
        </div>
      </article>

      <article class="surface task-detail-panel">
        <header class="task-detail-panel-head">
          <h3>{{ text.diagnostics }}</h3>
        </header>
        <div class="task-diagnostics-stack">
          <details class="task-diagnostic-block">
            <summary>{{ text.rawInput }}</summary>
            <pre class="detail-pre">{{ formattedInput }}</pre>
          </details>
          <details class="task-diagnostic-block">
            <summary>{{ text.rawOutput }}</summary>
            <pre class="detail-pre">{{ formattedOutput }}</pre>
          </details>
        </div>
      </article>
    </div>
  </section>
</template>

<style scoped>
.task-detail-page {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: 12px;
  height: 100%;
  min-height: 0;
}

.task-detail-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 16px 18px;
  background: var(--bg-card);
}

.task-detail-header-copy {
  min-width: 0;
}

.task-detail-caption {
  margin: 0 0 6px;
  color: var(--text-muted);
  font-size: 12px;
}

.task-detail-title {
  margin: 0;
  font-size: 22px;
  line-height: 1.2;
  color: var(--text-main);
}

.task-detail-subtitle {
  margin: 8px 0 0;
  color: var(--text-muted);
  font-size: 13px;
  line-height: 1.5;
  word-break: break-all;
}

.task-status-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  min-width: 84px;
  padding: 7px 12px;
  border: 1px solid var(--stroke-soft);
  border-radius: 999px;
  background: var(--bg-input);
  color: var(--text-main);
  font-size: 12px;
  font-weight: 600;
}

.task-status-badge[data-tone='info'] {
  color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 38%, var(--stroke-soft));
  background: color-mix(in srgb, var(--bg-input) 82%, var(--accent) 18%);
}

.task-status-badge[data-tone='success'] {
  color: #2aa36b;
  border-color: color-mix(in srgb, #2aa36b 42%, var(--stroke-soft));
  background: color-mix(in srgb, var(--bg-input) 82%, #2aa36b 18%);
}

.task-status-badge[data-tone='danger'] {
  color: var(--danger);
  border-color: color-mix(in srgb, var(--danger) 44%, var(--stroke-soft));
  background: color-mix(in srgb, var(--bg-input) 84%, var(--danger) 16%);
}

.task-detail-scroll {
  display: grid;
  gap: 12px;
  overflow: auto;
  min-height: 0;
  padding-right: 2px;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.task-detail-scroll::-webkit-scrollbar {
  width: 0;
  height: 0;
}

.task-detail-panel {
  display: grid;
  gap: 12px;
  padding: 16px;
  background: var(--bg-card);
}

.task-detail-panel-head h3 {
  margin: 0;
  font-size: 15px;
  color: var(--text-main);
}

.task-overview-grid {
  display: grid;
  gap: 10px;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  margin: 0;
}

.task-overview-item,
.task-info-item {
  display: grid;
  gap: 6px;
  padding: 12px;
  border: 1px solid var(--divider-soft);
  border-radius: 10px;
  background: var(--bg-input);
}

.task-overview-item dt,
.task-info-item dt {
  color: var(--text-muted);
  font-size: 12px;
}

.task-overview-item dd,
.task-info-item dd {
  margin: 0;
  color: var(--text-main);
  font-size: 14px;
  line-height: 1.55;
}

.task-info-stack,
.task-diagnostics-stack {
  display: grid;
  gap: 10px;
  margin: 0;
}

.mono {
  font-family: 'SF Mono', 'Menlo', monospace;
  word-break: break-all;
}

.multiline {
  white-space: pre-wrap;
  word-break: break-word;
}

.task-log-list {
  display: grid;
  gap: 10px;
  max-height: 280px;
  overflow: auto;
  min-height: 0;
  padding-right: 2px;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.task-log-list::-webkit-scrollbar,
.detail-pre::-webkit-scrollbar {
  width: 0;
  height: 0;
}

.task-log-item {
  display: grid;
  gap: 8px;
  padding: 12px;
  border: 1px solid var(--divider-soft);
  border-radius: 10px;
  background: var(--bg-input);
}

.task-log-item-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.task-log-time {
  color: var(--text-muted);
  font-size: 12px;
}

.task-log-level {
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.05em;
}

.task-log-item p {
  margin: 0;
  color: var(--text-main);
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-word;
}

.task-diagnostic-block {
  border: 1px solid var(--divider-soft);
  border-radius: 10px;
  background: var(--bg-input);
  overflow: hidden;
}

.task-diagnostic-block summary {
  cursor: pointer;
  padding: 12px 14px;
  color: var(--text-main);
  font-size: 13px;
  font-weight: 600;
  user-select: none;
}

.task-diagnostic-block[open] summary {
  border-bottom: 1px solid var(--divider-soft);
}

.detail-pre {
  margin: 0;
  max-height: 260px;
  overflow: auto;
  padding: 14px;
  color: var(--text-main);
  white-space: pre-wrap;
  word-break: break-word;
  scrollbar-width: none;
  -ms-overflow-style: none;
  font-family: 'SF Mono', 'Menlo', monospace;
  font-size: 12px;
  line-height: 1.55;
}

@media (max-width: 760px) {
  .task-detail-header {
    flex-direction: column;
  }

  .task-overview-grid {
    grid-template-columns: 1fr;
  }

  .task-log-item-head {
    align-items: flex-start;
    flex-direction: column;
    gap: 4px;
  }
}
</style>
