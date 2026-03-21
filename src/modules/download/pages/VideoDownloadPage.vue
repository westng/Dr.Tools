<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { submitVideoDownload } from '@/modules/download/api/download.api';
import { useSettingsStore } from '@/modules/settings/stores/settings.store';
import { getTaskBatchDetails } from '@/modules/tasks/api/tasks.api';
import { useMessages } from '@/i18n';
import { toErrorMessage } from '@/lib/errors';
import type { DownloadPlatform } from '@/modules/download/types';
import type { TaskDetail, TaskLogEntry } from '@/modules/tasks/types';

interface PlatformOption {
  value: DownloadPlatform;
  label: string;
  available: boolean;
}

const settingsStore = useSettingsStore();
const { settings } = storeToRefs(settingsStore);

const platform = ref<DownloadPlatform>('douyin');
const rawInput = ref('');
const downloadCover = ref(false);
const downloadMusic = ref(false);
const downloadDescription = ref(false);
const downloadLyric = ref(false);
const submitting = ref(false);
const validationError = ref('');
const submitError = ref('');
const trackedTaskIds = ref<string[]>([]);
const trackedTasks = ref<TaskDetail[]>([]);
const processingLogEntries = ref<TaskLogEntry[]>([]);
const logError = ref('');
let pollingTimer: number | null = null;

const supportedSubmitPlatforms: DownloadPlatform[] = ['douyin', 'tiktok'];

function extractFirstUrl(input: string): string | null {
  const match = input.match(/https?:\/\/\S+/i);
  return match ? match[0] : null;
}

const normalizedUrls = computed(() => {
  const seen = new Set<string>();
  const items: string[] = [];

  for (const rawLine of rawInput.value.split('\n')) {
    const value = extractFirstUrl(rawLine);
    if (!value || seen.has(value)) {
      continue;
    }
    seen.add(value);
    items.push(value);
  }

  return items;
});

const text = useMessages((messages) => messages.pages.videoDownload);
const platformOptions = computed<PlatformOption[]>(() => [
  { value: 'douyin', label: text.value.platformLabels.douyin, available: true },
  { value: 'tiktok', label: text.value.platformLabels.tiktok, available: true },
  { value: 'twitter', label: text.value.platformLabels.twitter, available: false },
  { value: 'weibo', label: text.value.platformLabels.weibo, available: false }
]);
const supportsLyric = computed(() => platform.value === 'douyin');
const activeTaskCount = computed(
  () => trackedTasks.value.filter((task) => task.status !== 'success' && task.status !== 'failed').length
);
const displayedLogEntries = computed(() =>
  [...processingLogEntries.value].sort((left, right) => right.ts.localeCompare(left.ts))
);

watch(supportsLyric, (value) => {
  if (!value) {
    downloadLyric.value = false;
  }
});

const tokenWarning = computed(() => {
  if (!supportedSubmitPlatforms.includes(platform.value)) {
    return {
      type: 'warning',
      message: text.value.platformUnavailableInApp
    };
  }

  const cookie = platform.value === 'douyin' ? settings.value.douyinCookie : settings.value.tiktokCookie;
  const status = platform.value === 'douyin'
    ? settings.value.douyinLastCheckStatus
    : settings.value.tiktokLastCheckStatus;

  if (!cookie) {
    return {
      type: 'warning',
      message: text.value.tokenMissing
    };
  }

  if (status === 'invalid' || status === 'expired') {
    return {
      type: 'danger',
      message: text.value.tokenInvalid
    };
  }

  if (status === 'unchecked') {
    return {
      type: 'warning',
      message: text.value.tokenUnchecked
    };
  }

  return null;
});

function stopPolling(): void {
  if (pollingTimer !== null) {
    window.clearInterval(pollingTimer);
    pollingTimer = null;
  }
}

async function refreshProcessingLogs(): Promise<void> {
  if (trackedTaskIds.value.length === 0) {
    processingLogEntries.value = [];
    trackedTasks.value = [];
    stopPolling();
    return;
  }

  try {
    const details = await getTaskBatchDetails(trackedTaskIds.value);
    trackedTasks.value = details;
    logError.value = '';

    const existing = new Set(
      processingLogEntries.value.map((entry) => `${entry.taskId}|${entry.ts}|${entry.level}|${entry.message}`)
    );
    const merged = [...processingLogEntries.value];

    for (const task of details) {
      for (const log of task.logs) {
        const key = `${log.taskId}|${log.ts}|${log.level}|${log.message}`;
        if (!existing.has(key)) {
          existing.add(key);
          merged.push(log);
        }
      }
    }

    merged.sort((left, right) => left.ts.localeCompare(right.ts));
    processingLogEntries.value = merged;

    const allFinished =
      details.length > 0 && details.every((task) => task.status === 'success' || task.status === 'failed');
    if (allFinished) {
      stopPolling();
    }
  } catch (error) {
    logError.value = toErrorMessage(error, text.value.logPollingError);
  }
}

function startPolling(taskIds: string[]): void {
  stopPolling();
  trackedTaskIds.value = taskIds;
  trackedTasks.value = [];
  processingLogEntries.value = [];
  logError.value = '';

  void refreshProcessingLogs();
  pollingTimer = window.setInterval(() => {
    void refreshProcessingLogs();
  }, 2000);
}

async function submit(): Promise<void> {
  validationError.value = '';
  submitError.value = '';

  if (!supportedSubmitPlatforms.includes(platform.value)) {
    validationError.value = text.value.unavailablePlatform;
    return;
  }

  if (normalizedUrls.value.length === 0) {
    validationError.value = text.value.provideLink;
    return;
  }

  submitting.value = true;
  try {
    const result = await submitVideoDownload({
      platform: platform.value,
      urls: normalizedUrls.value,
      downloadCover: downloadCover.value,
      downloadMusic: downloadMusic.value,
      downloadDescription: downloadDescription.value,
      downloadLyric: supportsLyric.value ? downloadLyric.value : false
    });
    startPolling(result.createdTaskIds);
    rawInput.value = '';
  } catch (error) {
    submitError.value = toErrorMessage(error, text.value.submitFailed);
  } finally {
    submitting.value = false;
  }
}

onUnmounted(() => {
  stopPolling();
});
</script>

<template>
  <section class="download-page">
    <div class="download-stack">
      <section class="input-panel">
        <div class="card-head card-head-row card-head-top">
          <div class="card-head-copy">
            <h3 class="panel-title">{{ text.inputTitle }}</h3>
            <p class="settings-hint">{{ text.workspaceHint }}</p>
          </div>
          <span class="link-count-chip">{{ text.linkCount }} {{ normalizedUrls.length }}</span>
        </div>
        <textarea
          v-model="rawInput"
          class="textarea link-input"
          :placeholder="text.inputPlaceholder"
        ></textarea>
      </section>

      <article class="surface download-card controls-card">
        <div class="controls-grid">
          <section class="control-block">
            <div class="card-head">
              <h3 class="panel-title">{{ text.platform }}</h3>
            </div>
            <div class="platform-grid">
              <button
                v-for="item in platformOptions"
                :key="item.value"
                class="platform-card"
                :class="{ active: platform === item.value, pending: !item.available }"
                type="button"
                @click="platform = item.value"
              >
                <span class="platform-card-name">{{ item.label }}</span>
                <span v-if="!item.available" class="platform-card-badge">{{ text.comingSoon }}</span>
              </button>
            </div>
          </section>

          <section class="control-block">
            <div class="card-head">
              <h3 class="panel-title">{{ text.extraContent }}</h3>
            </div>
            <div class="option-grid">
              <label class="option-tile">
                <input v-model="downloadCover" type="checkbox" />
                <span>{{ text.cover }}</span>
              </label>
              <label class="option-tile">
                <input v-model="downloadMusic" type="checkbox" />
                <span>{{ text.music }}</span>
              </label>
              <label class="option-tile">
                <input v-model="downloadDescription" type="checkbox" />
                <span>{{ text.description }}</span>
              </label>
              <label class="option-tile" :class="{ disabled: !supportsLyric }">
                <input v-model="downloadLyric" type="checkbox" :disabled="!supportsLyric" />
                <span>{{ text.lyric }}</span>
              </label>
            </div>
            <p v-if="!supportsLyric" class="settings-hint option-hint">{{ text.lyricUnsupported }}</p>
          </section>
        </div>

        <p v-if="tokenWarning" class="token-warning" :class="`token-warning-${tokenWarning.type}`">
          {{ tokenWarning.message }}
        </p>
      </article>

      <article class="surface download-card submit-card">
        <div class="submit-card-copy">
          <h3 class="panel-title">{{ text.start }}</h3>
          <p class="settings-hint">{{ text.queueHint }}</p>
        </div>
        <div class="submit-card-actions">
          <button class="primary-btn submit-btn" :disabled="submitting || normalizedUrls.length === 0" @click="submit">
            {{ submitting ? text.starting : text.start }}
          </button>
        </div>
        <div v-if="validationError || submitError" class="submit-feedback">
          <p v-if="validationError" class="danger-text">{{ validationError }}</p>
          <p v-if="submitError" class="danger-text">{{ submitError }}</p>
        </div>
      </article>

      <div class="feedback-grid">
        <article class="surface download-card log-card">
          <div class="card-head card-head-row card-head-top">
            <div class="card-head-copy">
              <h3 class="panel-title">{{ text.logTitle }}</h3>
              <p class="settings-hint">{{ text.logHint }}</p>
            </div>
            <span v-if="activeTaskCount > 0" class="log-running-pill">{{ text.processing }} {{ activeTaskCount }}</span>
          </div>
          <p v-if="logError" class="danger-text">{{ logError }}</p>
          <div v-else-if="displayedLogEntries.length === 0" class="log-empty">
            {{ text.logEmpty }}
          </div>
          <div v-else class="log-list">
            <div
              v-for="entry in displayedLogEntries"
              :key="`${entry.taskId}-${entry.ts}-${entry.level}-${entry.message}`"
              class="log-entry"
            >
              <div class="log-entry-top">
                <span class="log-time">{{ entry.ts }}</span>
                <span class="log-level" :class="`log-level-${entry.level}`">{{ entry.level }}</span>
              </div>
              <div class="log-entry-meta">
                <span class="log-task">{{ text.task }} {{ entry.taskId.slice(0, 8) }}</span>
              </div>
              <span class="log-message">{{ entry.message }}</span>
            </div>
          </div>
        </article>
      </div>
    </div>
  </section>
</template>

<style scoped>
.download-page {
  display: block;
}

.download-stack {
  display: grid;
  gap: 14px;
}

.input-panel {
  display: grid;
  gap: 12px;
}

.download-card {
  display: grid;
  gap: 14px;
  padding: 18px;
  background: var(--bg-card);
}

.input-card {
  gap: 16px;
}

.card-head {
  display: grid;
  gap: 4px;
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

.card-head-copy {
  display: grid;
  gap: 6px;
  min-width: 0;
}

.panel-title {
  margin: 0;
  font-size: 18px;
  line-height: 1.2;
}

.settings-hint,
.submit-card-copy p,
.download-page .danger-text {
  margin: 0;
}

.link-input {
  min-height: 240px;
  resize: none;
  padding: 14px 16px;
  border-radius: 14px;
}

.link-count-chip,
.log-running-pill {
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

.controls-card {
  gap: 16px;
}

.controls-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 16px;
}

.control-block {
  display: grid;
  gap: 12px;
  min-width: 0;
}

.platform-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
}

.platform-card {
  appearance: none;
  -webkit-appearance: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
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

.platform-card:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 1px;
}

.platform-card.active {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 14%, var(--bg-input));
  color: var(--text-main);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 20%, transparent);
}

.platform-card.pending {
  color: var(--text-muted);
}

.platform-card-name {
  font-weight: 600;
}

.platform-card-badge {
  display: inline-flex;
  align-items: center;
  min-height: 20px;
  padding: 0 7px;
  border-radius: 999px;
  border: 1px solid var(--stroke-soft);
  background: color-mix(in srgb, var(--bg-main) 70%, transparent);
  color: var(--text-muted);
  font-size: 11px;
  line-height: 1;
}

.option-section {
  display: grid;
  gap: 10px;
}

.option-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
}

.option-tile {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 52px;
  padding: 12px 14px;
  border: 1px solid var(--stroke-soft);
  border-radius: 14px;
  background: var(--bg-input);
  color: var(--text-main);
  line-height: 1.35;
}

.option-tile input {
  flex: 0 0 auto;
  margin: 0;
}

.option-tile span {
  flex: 1;
  min-width: 0;
}

.option-tile.disabled {
  opacity: 0.56;
}

.option-hint {
  color: var(--text-muted);
}

.feedback-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 14px;
  align-items: start;
}

.token-warning {
  padding: 12px 14px;
  border-radius: 12px;
  font-size: 12px;
  line-height: 1.5;
}

.token-warning-warning {
  color: var(--text-main);
  border: 1px solid color-mix(in srgb, #d2a54a 40%, var(--stroke-soft));
  background: color-mix(in srgb, #d2a54a 10%, transparent);
}

.token-warning-danger {
  color: var(--danger);
  border: 1px solid color-mix(in srgb, var(--danger) 45%, var(--stroke-soft));
  background: color-mix(in srgb, var(--danger) 10%, transparent);
}

.submit-card {
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
}

.submit-card-copy {
  display: grid;
  gap: 8px;
}

.submit-card-actions {
  display: grid;
  gap: 8px;
  justify-items: end;
}

.submit-feedback {
  grid-column: 1 / -1;
  display: grid;
  gap: 6px;
}

.submit-btn {
  min-width: 148px;
  min-height: 44px;
  border-radius: 10px;
}

.log-card {
  align-content: start;
}

.log-list {
  display: grid;
  gap: 8px;
}

.log-empty {
  padding: 14px;
  border: 1px dashed var(--stroke-soft);
  border-radius: 14px;
  color: var(--text-muted);
  background: color-mix(in srgb, var(--bg-input) 90%, transparent);
}

.log-entry {
  display: grid;
  gap: 6px;
  padding: 12px 14px;
  border: 1px solid var(--stroke-soft);
  border-radius: 14px;
  background: var(--bg-input);
  font-size: 12px;
}

.log-entry-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.log-time,
.log-task {
  color: var(--text-muted);
}

.log-entry-meta {
  display: flex;
  align-items: center;
  gap: 8px;
}

.log-level {
  font-weight: 700;
  text-transform: uppercase;
}

.log-level-info {
  color: var(--text-main);
}

.log-level-error {
  color: var(--danger);
}

.log-message {
  color: var(--text-main);
  word-break: break-word;
}

@media (max-width: 980px) {
  .controls-grid,
  .submit-card {
    grid-template-columns: 1fr;
  }

  .submit-card-actions {
    justify-items: stretch;
  }
}

@media (max-width: 760px) {
  .platform-grid,
  .option-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .submit-card-actions,
  .card-head-row,
  .log-entry-top {
    justify-items: stretch;
    align-items: stretch;
    flex-direction: column;
  }

  .card-head-top {
    align-items: stretch;
  }

  .submit-btn {
    width: 100%;
  }
}

@media (max-width: 560px) {
  .platform-grid,
  .option-grid {
    grid-template-columns: 1fr;
  }
}
</style>
