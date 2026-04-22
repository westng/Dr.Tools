<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { useRoute, useRouter } from 'vue-router';
import { openExternalUrl } from '@/api/system.api';
import SidebarNav from '@/layouts/SidebarNav.vue';
import { getEnvironmentStatus } from '@/modules/settings/api/settings.api';
import { useSettingsStore } from '@/modules/settings/stores/settings.store';
import { translate } from '@/i18n';

const settingsStore = useSettingsStore();
const { settings } = storeToRefs(settingsStore);
const route = useRoute();
const router = useRouter();

const routeHistory = ref<string[]>([]);
const routeHistoryIndex = ref(-1);
const pythonEnvironmentInstalled = ref(false);

const pageTitle = computed(() => {
  const titleKey = String(route.meta.titleKey ?? 'routes.workbench');
  return translate(settings.value.locale, titleKey);
});

const pythonEnvironmentLabel = computed(() => {
  return pythonEnvironmentInstalled.value
    ? translate(settings.value.locale, 'app.pythonEnvReady')
    : translate(settings.value.locale, 'app.pythonEnvMissing');
});

const isStandalonePage = computed(() => route.meta.standalone === true);
const canGoBack = computed(() => !isStandalonePage.value && routeHistoryIndex.value > 0);
const canGoForward = computed(
  () => !isStandalonePage.value && routeHistoryIndex.value >= 0 && routeHistoryIndex.value < routeHistory.value.length - 1
);

watch(
  () => ({ path: route.fullPath, standalone: isStandalonePage.value }),
  ({ path, standalone }) => {
    if (standalone) {
      return;
    }
    syncRouteHistory(path);
  },
  { immediate: true }
);

onMounted(async () => {
  await refreshPythonEnvironmentStatus();
});

function syncRouteHistory(path: string): void {
  if (!path) {
    return;
  }

  if (routeHistoryIndex.value === -1) {
    routeHistory.value = [path];
    routeHistoryIndex.value = 0;
    return;
  }

  if (routeHistory.value[routeHistoryIndex.value] === path) {
    return;
  }

  const previousPath = routeHistory.value[routeHistoryIndex.value - 1];
  if (previousPath === path) {
    routeHistoryIndex.value -= 1;
    return;
  }

  const nextPath = routeHistory.value[routeHistoryIndex.value + 1];
  if (nextPath === path) {
    routeHistoryIndex.value += 1;
    return;
  }

  routeHistory.value = routeHistory.value.slice(0, routeHistoryIndex.value + 1);
  routeHistory.value.push(path);
  routeHistoryIndex.value = routeHistory.value.length - 1;
}

async function goBack(): Promise<void> {
  if (!canGoBack.value) {
    return;
  }

  const target = routeHistory.value[routeHistoryIndex.value - 1];
  if (!target) {
    return;
  }

  await router.push(target);
}

async function goForward(): Promise<void> {
  if (!canGoForward.value) {
    return;
  }

  const target = routeHistory.value[routeHistoryIndex.value + 1];
  if (!target) {
    return;
  }

  await router.push(target);
}

async function openGithub(): Promise<void> {
  await openExternalUrl('https://github.com/westng');
}

async function refreshPythonEnvironmentStatus(): Promise<void> {
  try {
    const status = await getEnvironmentStatus();
    pythonEnvironmentInstalled.value = status.installed && status.status === 'ready';
  } catch {
    pythonEnvironmentInstalled.value = false;
  }
}
</script>

<template>
  <section v-if="isStandalonePage" class="standalone-shell">
    <router-view />
  </section>
  <div v-else class="app-shell">
    <aside class="sidebar">
      <SidebarNav />
    </aside>
    <main class="content">
      <section class="content-page">
        <header class="content-toolbar">
          <div class="toolbar-nav">
            <button
              class="toolbar-nav-btn"
              type="button"
              :disabled="!canGoBack"
              :aria-label="translate(settings.locale, 'app.back')"
              @click="goBack"
            >
              ‹
            </button>
            <button
              class="toolbar-nav-btn"
              type="button"
              :disabled="!canGoForward"
              :aria-label="translate(settings.locale, 'app.forward')"
              @click="goForward"
            >
              ›
            </button>
          </div>
          <h2 class="toolbar-title">{{ pageTitle }}</h2>
          <div class="toolbar-spacer"></div>
          <div class="toolbar-actions">
            <button
              class="toolbar-env-status"
              type="button"
              :class="{ ready: pythonEnvironmentInstalled }"
              :aria-label="pythonEnvironmentLabel"
              :title="pythonEnvironmentLabel"
              @click="refreshPythonEnvironmentStatus"
            >
              <span class="toolbar-env-dot"></span>
              <span>{{ pythonEnvironmentLabel }}</span>
            </button>
            <button
              class="toolbar-action-btn"
              type="button"
              :aria-label="translate(settings.locale, 'app.github')"
              :title="translate(settings.locale, 'app.github')"
              @click="openGithub"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path
                  fill="currentColor"
                  d="M12 .5C5.649.5.5 5.653.5 12.01c0 5.086 3.292 9.383 7.86 10.906.575.106.785-.25.785-.556 0-.274-.01-1-.016-1.962-3.197.695-3.872-1.542-3.872-1.542-.523-1.33-1.278-1.684-1.278-1.684-1.045-.714.08-.7.08-.7 1.156.081 1.764 1.189 1.764 1.189 1.028 1.764 2.697 1.255 3.354.96.104-.746.402-1.255.731-1.544-2.552-.291-5.236-1.278-5.236-5.692 0-1.258.449-2.286 1.186-3.092-.12-.291-.514-1.464.112-3.052 0 0 .967-.31 3.17 1.181A11.04 11.04 0 0 1 12 6.32c.978.004 1.963.133 2.882.39 2.201-1.492 3.166-1.182 3.166-1.182.628 1.589.234 2.761.116 3.052.739.806 1.184 1.834 1.184 3.092 0 4.425-2.688 5.397-5.248 5.683.414.356.783 1.058.783 2.133 0 1.541-.013 2.784-.013 3.164 0 .309.207.668.792.555 4.563-1.525 7.85-5.821 7.85-10.904C23.5 5.653 18.351.5 12 .5Z"
                />
              </svg>
            </button>
          </div>
        </header>
        <section class="content-body">
          <router-view />
        </section>
      </section>
    </main>
  </div>
</template>
