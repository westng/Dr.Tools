<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { storeToRefs } from 'pinia';
import { useRoute, useRouter } from 'vue-router';
import SidebarNav from '@/layouts/SidebarNav.vue';
import { useSettingsStore } from '@/modules/settings/stores/settings.store';
import { translate } from '@/i18n';

const settingsStore = useSettingsStore();
const { settings } = storeToRefs(settingsStore);
const route = useRoute();
const router = useRouter();

const routeHistory = ref<string[]>([]);
const routeHistoryIndex = ref(-1);

const pageTitle = computed(() => {
  const titleKey = String(route.meta.titleKey ?? 'routes.workbench');
  return translate(settings.value.locale, titleKey);
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
        </header>
        <section class="content-body">
          <router-view />
        </section>
      </section>
    </main>
  </div>
</template>
