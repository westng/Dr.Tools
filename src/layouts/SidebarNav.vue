<script setup lang="ts">
import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { storeToRefs } from 'pinia';
import { getPrimaryNavigation, resolvePrimaryNavKey } from '@/navigation/primary-navigation';
import { useSettingsStore } from '@/modules/settings/stores/settings.store';
import { useMessages } from '@/i18n';

const route = useRoute();
const settingsStore = useSettingsStore();
const { settings } = storeToRefs(settingsStore);

const navigation = computed(() => getPrimaryNavigation(settings.value.locale));
const activeKey = computed(() => resolvePrimaryNavKey(route.meta.primaryNav));
const appText = useMessages((messages) => messages.app);
</script>

<template>
  <div class="nav-wrap">
    <header class="nav-header">
      <h1>Dr.Tools</h1>
      <p>{{ appText.sidebarTagline }}</p>
    </header>

    <nav class="nav-list">
      <RouterLink v-for="item in navigation.main" :key="item.to" :to="item.to" class="nav-item"
        :class="{ active: activeKey === item.key }">
        {{ item.label }}
      </RouterLink>
    </nav>

    <nav class="nav-list nav-list-bottom">
      <RouterLink v-for="item in navigation.bottom" :key="item.to" :to="item.to" class="nav-item"
        :class="{ active: activeKey === item.key }">
        {{ item.label }}
      </RouterLink>
    </nav>
  </div>
</template>

<style scoped>
.nav-wrap {
  display: grid;
  grid-template-rows: auto 1fr auto;
  height: 100%;
}

.nav-header {
  padding: 16px;
  border-bottom: 1px solid var(--divider-soft);
}

.nav-header h1 {
  margin: 0;
  font-size: 20px;
}

.nav-header p {
  margin: 4px 0 0;
  color: var(--text-muted);
}

.nav-list {
  display: grid;
  gap: 8px;
  padding: 12px;
  align-content: start;
}

.nav-list-bottom {}

.nav-item {
  text-decoration: none;
  color: var(--text-main);
  border-radius: 10px;
  padding: 10px 12px;
  background: var(--bg-card);
}

.nav-item:hover {
  border-color: color-mix(in srgb, var(--accent) 60%, var(--sidebar-stroke));
  background: color-mix(in srgb, var(--accent) 10%, var(--bg-card));
}

.nav-item.active {
  border-color: var(--accent);
  background: var(--accent);
  color: var(--accent-contrast);
}
</style>
