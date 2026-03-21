<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { storeToRefs } from 'pinia';
import emptyIcon from '@/assets/icons/empty.svg';
import {
  listDownloadBatches,
  openDownloadBatchDetailWindow,
  retryDownloadBatch,
} from '@/modules/tasks/api/tasks.api';
import { useSettingsStore } from '@/modules/settings/stores/settings.store';
import { translate, useMessages } from '@/i18n';
import { toErrorMessage } from '@/lib/errors';
import type { DownloadBatchSummary } from '@/modules/tasks/types';

const settingsStore = useSettingsStore();
const { settings } = storeToRefs(settingsStore);

const items = ref<DownloadBatchSummary[]>([]);
const loading = ref(false);
const error = ref('');
const openError = ref('');
const retryingBatchId = ref('');
const page = ref(1);
const pageSize = 10;
const total = ref(0);
let refreshRequestId = 0;
const text = useMessages((messages) => messages.pages.history);
const totalPages = computed(() => Math.max(1, Math.ceil(total.value / pageSize)));
const hasPreviousPage = computed(() => page.value > 1);
const hasNextPage = computed(() => page.value < totalPages.value);
const showPagination = computed(() => !error.value && !openError.value && total.value > 0 && items.value.length > 0);
const pageInfoText = computed(() =>
  text.value.pageInfo.replace('{page}', String(page.value)).replace('{pages}', String(totalPages.value))
);
const totalInfoText = computed(() => text.value.totalInfo.replace('{total}', String(total.value)));

onMounted(async () => {
  await settingsStore.ensureLoaded();
  await refresh();
});

async function refresh(): Promise<void> {
  if (loading.value) {
    return;
  }

  const requestId = ++refreshRequestId;
  loading.value = true;
  error.value = '';
  try {
    const nextResult = await listDownloadBatches(page.value, pageSize);
    if (requestId !== refreshRequestId) {
      return;
    }
    items.value = nextResult.items;
    total.value = nextResult.total;
    const safeTotalPages = Math.max(1, Math.ceil(nextResult.total / pageSize));
    if (page.value > safeTotalPages) {
      page.value = safeTotalPages;
      loading.value = false;
      window.setTimeout(() => {
        void refresh();
      }, 0);
      return;
    }
  } catch (loadError) {
    if (requestId === refreshRequestId) {
      error.value = toErrorMessage(loadError, text.value.loadFailed);
    }
  } finally {
    if (requestId === refreshRequestId) {
      loading.value = false;
    }
  }
}

async function changePage(nextPage: number): Promise<void> {
  if (loading.value) {
    return;
  }

  const normalizedPage = Math.min(Math.max(1, nextPage), totalPages.value);
  if (normalizedPage === page.value) {
    return;
  }

  page.value = normalizedPage;
  await refresh();
}

async function openBatchDetail(item: DownloadBatchSummary): Promise<void> {
  openError.value = '';
  try {
    await openDownloadBatchDetailWindow(item.id, translate(settings.value.locale, 'routes.batchDetail'));
  } catch (openDetailError) {
    openError.value = toErrorMessage(openDetailError, text.value.openWindowFailed);
  }
}

async function retryBatch(item: DownloadBatchSummary): Promise<void> {
  if (item.status !== 'queued' || retryingBatchId.value) {
    return;
  }

  openError.value = '';
  retryingBatchId.value = item.id;
  try {
    items.value = items.value.map((entry) =>
      entry.id === item.id
        ? {
            ...entry,
            status: 'running',
            runningCount: Math.max(entry.runningCount, 1)
          }
        : entry
    );
    await retryDownloadBatch(item.id);
  } catch (retryError) {
    openError.value = toErrorMessage(retryError, text.value.retryFailed);
    await refresh();
  } finally {
    retryingBatchId.value = '';
  }

  window.setTimeout(() => {
    void refresh();
  }, 250);
}

function formatPlatform(value: string): string {
  if (value in text.value.platformLabels) {
    return text.value.platformLabels[value as keyof typeof text.value.platformLabels];
  }
  return value || '-';
}

function formatStatus(value: string): string {
  return text.value.statusMap[value as keyof typeof text.value.statusMap] ?? value;
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
    hour12: false
  }).format(date);
}
</script>

<template>
  <section class="task-records-page">
    <div class="task-records-actions">
      <button :disabled="loading || Boolean(retryingBatchId)" @click="refresh">{{ text.refresh }}</button>
    </div>

    <article class="surface task-records-card" :class="{ 'has-pagination': showPagination }">
      <p v-if="loading">{{ text.loading }}</p>
      <p v-else-if="error" class="danger-text">{{ error }}</p>
      <p v-else-if="openError" class="danger-text">{{ openError }}</p>
      <div v-else-if="items.length === 0" class="empty-state">
        <img :src="emptyIcon" alt="" class="empty-state-icon" />
        <p class="settings-hint empty-state-text">{{ text.empty }}</p>
      </div>
      <div v-else class="task-records-table-wrap">
        <table class="task-records-table">
          <colgroup>
            <col class="column-batch" />
            <col class="column-platform" />
            <col class="column-status" />
            <col class="column-count" />
            <col class="column-count" />
            <col class="column-count" />
            <col class="column-count" />
            <col class="column-updated" />
            <col class="column-action" />
          </colgroup>
          <thead>
          <tr>
            <th class="column-batch">{{ text.batchId }}</th>
            <th class="column-platform">{{ text.platform }}</th>
            <th class="column-status">{{ text.status }}</th>
            <th class="column-count">{{ text.total }}</th>
            <th class="column-count">{{ text.success }}</th>
            <th class="column-count">{{ text.failed }}</th>
            <th class="column-count">{{ text.running }}</th>
            <th class="column-updated">{{ text.updated }}</th>
            <th class="column-action">
              <div class="action-sticky action-sticky-head">{{ text.action }}</div>
            </th>
          </tr>
          </thead>
          <tbody>
            <tr v-for="item in items" :key="item.id">
              <td class="task-record-id column-batch table-ellipsis">{{ item.id }}</td>
              <td class="column-platform table-ellipsis">{{ formatPlatform(item.platform) }}</td>
              <td class="column-status">
                <span class="task-status-pill" :data-status="item.status">{{ formatStatus(item.status) }}</span>
              </td>
              <td class="column-count">{{ item.totalCount }}</td>
              <td class="column-count">{{ item.successCount }}</td>
              <td class="column-count">{{ item.failedCount }}</td>
              <td class="column-count">{{ item.runningCount }}</td>
              <td class="column-updated table-ellipsis">{{ formatDateTime(item.updatedAt) }}</td>
              <td class="column-action">
                <div class="task-record-actions-cell action-sticky action-sticky-body">
                  <button
                    v-if="item.status === 'queued'"
                    class="detail-link-btn"
                    :disabled="retryingBatchId === item.id"
                    @click="retryBatch(item)"
                  >
                    {{ retryingBatchId === item.id ? text.retrying : text.retry }}
                  </button>
                  <button class="detail-link-btn" @click="openBatchDetail(item)">{{ text.detail }}</button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <footer v-if="showPagination" class="task-records-pagination">
        <p class="task-records-pagination-meta">{{ totalInfoText }} · {{ pageInfoText }}</p>
        <div class="task-records-pagination-actions">
          <button :disabled="loading || !hasPreviousPage" @click="changePage(page - 1)">
            {{ text.previousPage }}
          </button>
          <button :disabled="loading || !hasNextPage" @click="changePage(page + 1)">
            {{ text.nextPage }}
          </button>
        </div>
      </footer>
    </article>
  </section>
</template>

<style scoped>
.task-records-page {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: 12px;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.task-records-card {
  position: relative;
  height: 100%;
  min-height: 0;
  padding: 14px;
  overflow: hidden;
}

.task-records-card.has-pagination {
  padding-bottom: 92px;
}

.task-records-actions {
  display: flex;
  justify-content: flex-end;
  min-height: 0;
}

.empty-state {
  display: grid;
  justify-items: center;
  gap: 12px;
  padding: 28px 16px 20px;
}

.empty-state-icon {
  width: 180px;
  max-width: 100%;
  height: auto;
  display: block;
}

.empty-state-text {
  margin: 0;
  text-align: center;
}

.task-records-table-wrap {
  overflow-x: auto;
  overflow-y: auto;
  height: 100%;
  min-height: 0;
  padding-right: 8px;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.task-records-table-wrap::-webkit-scrollbar {
  width: 0;
  height: 0;
}

.task-records-table {
  width: 100%;
  min-width: 1040px;
  border-collapse: separate;
  border-spacing: 0;
  table-layout: auto;
}

.task-records-table th,
.task-records-table td {
  padding: 16px 12px;
  text-align: left;
  border-bottom: 1px solid var(--divider-soft);
  vertical-align: middle;
}

.task-records-table th {
  padding-top: 14px;
  padding-bottom: 14px;
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 600;
}

.task-records-table th:first-child,
.task-records-table td:first-child {
  padding-left: 14px;
}

.task-records-table th:last-child,
.task-records-table td:last-child {
  padding-right: 14px;
}

.task-records-table tbody tr:last-child td {
  border-bottom: none;
}

.task-record-id {
  font-family: 'SF Mono', 'Menlo', monospace;
  font-size: 11px;
  line-height: 1.4;
  max-width: 220px;
}

.table-ellipsis {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.task-status-pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 28px;
  padding: 0 10px;
  border: 1px solid var(--stroke-soft);
  border-radius: 999px;
  background: var(--bg-input);
  color: var(--text-main);
  font-size: 11px;
  font-weight: 600;
}

.task-status-pill[data-status='running'] {
  color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 35%, var(--stroke-soft));
}

.task-status-pill[data-status='success'] {
  color: #2aa36b;
  border-color: color-mix(in srgb, #2aa36b 38%, var(--stroke-soft));
}

.task-status-pill[data-status='failed'] {
  color: var(--danger);
  border-color: color-mix(in srgb, var(--danger) 42%, var(--stroke-soft));
}

.task-status-pill[data-status='partial'] {
  color: #d28b12;
  border-color: color-mix(in srgb, #d28b12 42%, var(--stroke-soft));
}

.detail-link-btn {
  min-width: 76px;
  padding: 6px 12px;
  font-size: 12px;
}

.task-record-actions-cell {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  flex-wrap: nowrap;
  overflow: visible !important;
  white-space: nowrap;
}

.task-records-pagination {
  position: absolute;
  left: 14px;
  right: 14px;
  bottom: 14px;
  z-index: 2;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 14px;
  border: 1px solid color-mix(in srgb, var(--stroke-soft) 78%, transparent);
  border-radius: 16px;
  background: color-mix(in srgb, var(--bg-card) 92%, transparent);
  box-shadow: inset 0 1px 0 color-mix(in srgb, #ffffff 8%, transparent);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

:root[data-glass-style='transparent'] .task-records-pagination {
  background: color-mix(in srgb, var(--bg-card) 68%, transparent);
  backdrop-filter: blur(16px) saturate(130%);
  -webkit-backdrop-filter: blur(16px) saturate(130%);
}

:root[data-glass-style='tinted'] .task-records-pagination {
  background: var(--bg-card);
}

.task-records-pagination-meta {
  margin: 0;
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.5;
}

.task-records-pagination-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.task-records-pagination-actions button {
  min-width: 76px;
  border-radius: 999px;
  padding: 8px 14px;
}

.column-platform {
  width: 76px;
  text-align: center !important;
}

.column-status {
  width: 108px;
  text-align: center !important;
}

.column-count {
  width: 56px;
  text-align: center !important;
}

.column-updated {
  width: 148px;
  color: var(--text-muted);
  font-size: 12px;
  text-align: center !important;
}

.column-action {
  width: 160px;
  text-align: center !important;
  padding-left: 0 !important;
  padding-right: 0 !important;
}

.action-sticky {
  position: sticky;
  right: 0;
  z-index: 2;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 160px;
  min-height: 44px;
  margin-left: auto;
  background: var(--bg-card);
  box-shadow: -10px 0 18px color-mix(in srgb, var(--bg-card) 88%, transparent);
}

.action-sticky-head {
  z-index: 3;
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 600;
}

.action-sticky-body {
  border-left: 1px solid var(--divider-soft);
}

:root[data-glass-style='transparent'] .action-sticky {
  background: color-mix(in srgb, var(--bg-card) 88%, transparent);
  backdrop-filter: blur(12px) saturate(120%);
  -webkit-backdrop-filter: blur(12px) saturate(120%);
}

:root[data-glass-style='tinted'] .action-sticky {
  background: var(--bg-card);
}

.action-sticky .detail-link-btn {
  flex: 0 0 auto;
}

@media (max-width: 1200px) {
  .task-records-table {
    min-width: 1040px;
  }
}

@media (max-width: 720px) {
  .task-records-pagination {
    flex-direction: column;
    align-items: stretch;
  }

  .task-records-pagination-actions {
    justify-content: flex-end;
  }
}
</style>
