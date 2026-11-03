<script setup lang="ts">
import { onMounted, onUnmounted, watch, ref } from 'vue';
import { listen } from '@tauri-apps/api/event';
import { useAppStore } from './stores/app';
import { nativeMode, windowLabel } from './services/bridge';
import MainWindow from './windows/MainWindow.vue';
import WidgetWindow from './windows/WidgetWindow.vue';
import QuickEntryWindow from './windows/QuickEntryWindow.vue';
import AppButton from './components/AppButton.vue';
import { useTasksStore } from './stores/tasks';
import { useUpdatesStore } from './stores/updates';
import UpdatePanel from './components/UpdatePanel.vue';

const store = useAppStore();
const tasks = useTasksStore();
const updates = useUpdatesStore();
watch(
  () => store.ready,
  (ready) => {
    if (ready) void tasks.initialize();
  },
  { immediate: true },
);
const label = windowLabel();
document.documentElement.dataset.window = label;
const restoredEpoch = ref(0),
  restoreNote = ref('');
let stopRestore: (() => void) | undefined,
  restoreDisposed = false;
if (nativeMode)
  void listen<{ safetyId: string; warnings: string[] }>('taskdock://database-restored', (event) => {
    restoreNote.value =
      '数据库已恢复，恢复前安全备份已保留。' + (event.payload.warnings?.join('；') || '');
    restoredEpoch.value++;
    tasks.dispose();
    void store.initialize();
  }).then((stop) => {
    if (restoreDisposed) stop();
    else stopRestore = stop;
  });
const media = window.matchMedia('(prefers-color-scheme: dark)');
const syncSystem = () => {
  store.systemDark = media.matches;
};
const refresh = () => {
  if (store.ready) void store.refresh();
  tasks.tick();
  if (tasks.ready) void tasks.refresh();
};
const uiError = () => store.fail(null);
watch(
  () => store.resolvedTheme,
  (theme) => {
    document.documentElement.dataset.theme = theme;
  },
  { immediate: true },
);
onMounted(() => {
  syncSystem();
  media.addEventListener('change', syncSystem);
  window.addEventListener('focus', refresh);
  window.addEventListener('taskdock-ui-error', uiError);
  void store.initialize();
  void updates.initialize(label === 'main');
});
onUnmounted(() => {
  restoreDisposed = true;
  stopRestore?.();
  updates.dispose();
  store.dispose();
  tasks.dispose();
  media.removeEventListener('change', syncSystem);
  window.removeEventListener('focus', refresh);
  window.removeEventListener('taskdock-ui-error', uiError);
});
</script>

<template>
  <div class="application" :class="`window-${label}`">
    <div v-if="!nativeMode" class="preview-banner">浏览器预览 · 设置仅保存在内存中</div>
    <div v-if="store.error" class="error-banner" role="alert">
      <span
        >{{ store.error.message }} <small>{{ store.error.code }}</small></span
      >
      <AppButton variant="ghost" @click="store.initialize">重新连接</AppButton>
    </div>
    <div v-if="!store.ready" class="loading-state" role="status">
      <p v-if="tasks.error" role="alert">{{ tasks.error.message }}</p>
      <span class="brand-mark">✓</span>
      <h1>TaskDock</h1>
      <p>{{ store.error ? '基础服务暂不可用，请尝试重新连接。' : '正在连接本地工作台…' }}</p>
    </div>
    <div v-if="store.ready && tasks.error" class="error-banner task-error" role="alert">
      <span
        >{{ tasks.error.message }} <small>{{ tasks.error.code }}</small></span
      ><AppButton
        variant="ghost"
        :disabled="tasks.refreshing"
        @click="tasks.ready ? tasks.refresh() : tasks.initialize()"
        >刷新任务</AppButton
      >
    </div>
    <p v-if="restoreNote" class="success-note" role="status">{{ restoreNote }}</p>
    <UpdatePanel v-if="label === 'main' && store.ready" compact />
    <div v-if="updates.state.phase === 'installing'" class="update-installing" role="alert">
      正在安装更新，请稍候…
    </div>
    <MainWindow v-if="label === 'main'" :key="restoredEpoch" />
    <WidgetWindow v-else-if="label === 'widget'" :key="restoredEpoch" />
    <QuickEntryWindow v-else :key="restoredEpoch" />
  </div>
</template>
