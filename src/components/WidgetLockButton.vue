<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { bridge } from '../services/bridge';
import type { SettingsSnapshot } from '../types';
const busy = ref(false),
  error = ref('');
const media = window.matchMedia('(prefers-color-scheme: dark)');
const theme = ref('system'),
  dark = ref(media.matches);
let revision = -1,
  disposed = false,
  stop: (() => void) | undefined;
const system = () => {
  dark.value = media.matches;
};
function apply(settings: SettingsSnapshot) {
  if (disposed || settings.revision < revision) return;
  revision = settings.revision;
  theme.value = settings.theme;
}
watch(
  [theme, dark],
  () => {
    document.documentElement.dataset.theme =
      theme.value === 'system' ? (dark.value ? 'dark' : 'light') : theme.value;
  },
  { immediate: true },
);
onMounted(async () => {
  media.addEventListener('change', system);
  try {
    const unlisten = await bridge.onSettings(apply);
    if (disposed) {
      unlisten();
      return;
    }
    stop = unlisten;
    apply(await bridge.bootstrap());
  } catch {
    /* Unlock must remain available even if theme loading fails. */
  }
  if (!disposed) {
    try {
      await invoke('widget_lock_control_ready');
    } catch {
      error.value = '解锁按钮尚未就绪，也可从托盘解锁。';
    }
  }
});
onBeforeUnmount(() => {
  disposed = true;
  stop?.();
  media.removeEventListener('change', system);
});
async function unlock() {
  if (busy.value) return;
  busy.value = true;
  error.value = '';
  try {
    await bridge.widgetAction('unlock');
  } catch {
    error.value = '解锁失败，请重试或从托盘解锁。';
  } finally {
    busy.value = false;
  }
}
</script>
<template>
  <button
    type="button"
    class="unlock-control"
    aria-label="解锁悬浮窗"
    :aria-busy="busy"
    :title="error || '点击解锁悬浮窗'"
    @click="unlock"
  />
</template>
<style>
html,
body,
#app {
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  background: var(--surface);
}
.unlock-control {
  display: grid;
  place-items: center;
  width: 100%;
  height: 100%;
  margin: 0;
  padding: 0;
  border: 0;
  border-radius: 3px;
  background: transparent;
  color: var(--text);
}
.unlock-control:hover,
.unlock-control:active {
  background: transparent;
  color: var(--text);
}
</style>
