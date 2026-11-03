<script setup lang="ts">
import { computed, ref, watch, onMounted, onUnmounted } from 'vue';
import CalendarView from '../components/CalendarView.vue';
import WeekOverview from '../components/WeekOverview.vue';
import ReminderCenter from '../components/ReminderCenter.vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import SettingsPanel from '../components/SettingsPanel.vue';
import AppIcon from '../components/AppIcon.vue';
import AppButton from '../components/AppButton.vue';
import ThemeSelect from '../components/ThemeSelect.vue';
import DetailDrawer from '../components/DetailDrawer.vue';
import CategoryManager from '../components/CategoryManager.vue';
import TaskList from '../components/TaskList.vue';
import { bridge, nativeMode } from '../services/bridge';
import { useAppStore } from '../stores/app';
import { useTasksStore } from '../stores/tasks';
import type { Task, TaskView } from '../task-types';
const app = useAppStore(),
  store = useTasksStore();
const settingsOpen = ref(false);
const queryRefresh = ref(0);
const mode = ref<'tasks' | 'calendar' | 'week'>('tasks');
const createDate = ref<string | null>(null),
  selectedRecord = ref<Task | null>(null);
let stopSettings: (() => void) | undefined;
let settingsDisposed = false;
let stopNotification: (() => void) | undefined;
async function openNotification() {
  if (!nativeMode) return;
  try {
    const id = await invoke<string | null>('take_notification_task');
    await store.refresh();
    if (!id) return;
    const task = store.tasks.find((t) => t.id === id && !t.deletedAt);
    if (task) {
      if (drawer.value) {
        notice.value = '收到任务提醒。当前草稿已保留，可关闭详情后在提醒中心查看。';
        return;
      }
      settingsOpen.value = false;
      edit(task);
    }
  } catch (e) {
    app.fail(e);
  }
}
onMounted(async () => {
  if (nativeMode) {
    const unlisten = await listen('taskdock://notification-open', () => {
      void openNotification();
    });
    if (settingsDisposed) unlisten();
    else stopNotification = unlisten;
    window.addEventListener('focus', openNotification);
    void openNotification();
  }
  const stop = await bridge.onOpenSettings(() => {
    settingsOpen.value = true;
  });
  if (settingsDisposed) stop();
  else stopSettings = stop;
});
onUnmounted(() => {
  settingsDisposed = true;
  stopSettings?.();
  stopNotification?.();
  window.removeEventListener('focus', openNotification);
});
const selected = ref<TaskView>('today'),
  categoryFilter = ref('');
const drawer = ref(false),
  selectedId = ref<string | null>(null),
  categoriesOpen = ref(false),
  notice = ref('');
watch(
  () => store.revision,
  () => {
    notice.value = '';
  },
);
watch(
  () => store.error,
  (error) => {
    if (error) notice.value = '';
  },
);
const views: { id: TaskView; title: string; icon: string }[] = [
  {
    id: 'today',
    title: '今天',
    icon: 'today',
  },
  {
    id: 'plan',
    title: '未来事项',
    icon: 'plan',
  },
  {
    id: 'inbox',
    title: '心愿单',
    icon: 'spark',
  },
  {
    id: 'done',
    title: '已完成',
    icon: 'done',
  },
];
const current = computed(() => views.find((item) => item.id === selected.value)!);
const taskViews = computed(() => views.filter((view) => view.id === 'today' || view.id === 'plan'));
const isTaskWorkspace = computed(() => selected.value === 'today' || selected.value === 'plan');
const otherViews = computed(() =>
  views.filter((view) => view.id === 'inbox' || view.id === 'done'),
);
const visible = computed(() =>
  store
    .view(selected.value)
    .filter(
      (task) =>
        !categoryFilter.value ||
        (categoryFilter.value === 'uncategorized'
          ? !task.categoryId
          : task.categoryId === categoryFilter.value),
    ),
);
const selectedTask = computed(() => {
  const current = store.tasks.find((t) => t.id === selectedId.value);
  return current && (!selectedRecord.value || current.version >= selectedRecord.value.version)
    ? current
    : selectedRecord.value;
});
const date = computed(() =>
  new Date(`${store.today}T12:00:00`).toLocaleDateString('zh-CN', {
    month: 'long',
    day: 'numeric',
    weekday: 'long',
  }),
);
function navigate(view: TaskView) {
  mode.value = 'tasks';
  selected.value = view;
  notice.value = '';
}
function edit(task: Task) {
  selectedRecord.value = task;
  notice.value = '';
  selectedId.value = task.id;
  drawer.value = true;
}
function create(day?: string) {
  createDate.value =
    day ?? (mode.value === 'tasks' && selected.value === 'inbox' ? null : store.today);
  selectedRecord.value = null;
  notice.value = '';
  selectedId.value = null;
  drawer.value = true;
}
function saved(id: string) {
  const task = store.tasks.find((item) => item.id === id);
  if (!selectedId.value && task && mode.value !== 'week') {
    navigate(!task.plannedDate ? 'inbox' : task.plannedDate > store.today ? 'plan' : 'today');
    categoryFilter.value = '';
  }
  drawer.value = false;
  notice.value = task?.plannedDate === null ? '心愿已保存。' : '任务已保存。';
}
async function openWindow(label: 'widget' | 'quick-entry') {
  try {
    await bridge.showWindow(label);
  } catch (error) {
    app.fail(error);
  }
}
</script>
<template>
  <div class="main-layout">
    <aside class="sidebar">
      <div class="brand">
        <span class="brand-mark">✓</span>
        <div><strong>TaskDock</strong></div>
      </div>
      <div class="sidebar-navigation">
        <div class="workspace-label">我的工作台 <span>本地</span></div>
        <nav aria-label="任务视图">
          <button
            class="nav-item"
            :class="{ active: mode === 'tasks' && isTaskWorkspace }"
            :aria-current="mode === 'tasks' && isTaskWorkspace ? 'page' : undefined"
            @click="navigate(isTaskWorkspace ? selected : 'today')"
          >
            <AppIcon name="today" /><span>任务</span>
            <span class="nav-count">{{
              store.view('today').length + store.view('plan').length
            }}</span>
          </button>
          <button
            v-for="view in otherViews"
            :key="view.id"
            class="nav-item"
            :class="{ active: mode === 'tasks' && selected === view.id }"
            :aria-current="mode === 'tasks' && selected === view.id ? 'page' : undefined"
            @click="navigate(view.id)"
          >
            <AppIcon :name="view.icon" /><span>{{ view.title }}</span
            ><span class="nav-count">{{ store.view(view.id).length }}</span>
          </button>
        </nav>
        <div class="sidebar-divider" />
        <button
          class="nav-item nav-extra"
          :class="{ active: mode === 'calendar' }"
          @click="mode = 'calendar'"
        >
          <AppIcon name="plan" /><span>月历</span>
        </button>
        <button
          class="nav-item nav-extra"
          :class="{ active: mode === 'week' }"
          :aria-current="mode === 'week' ? 'page' : undefined"
          @click="mode = 'week'"
        >
          <AppIcon name="plan" /><span>周概览</span>
        </button>
        <button class="nav-item" @click="categoriesOpen = true">
          <AppIcon name="folder" /><span>分类管理</span>
        </button>
        <div class="sidebar-divider" />
        <div class="workspace-label">桌面空间</div>
        <button class="nav-item" @click="openWindow('widget')">
          <AppIcon name="widget" /><span>悬浮小组件</span><AppIcon name="arrow" :size="15" />
        </button>
        <button class="nav-item" @click="settingsOpen = true">
          <AppIcon name="shield" /><span>软件设置</span>
        </button>
      </div>
      <div class="sidebar-bottom">
        <div class="local-note">
          <AppIcon name="shield" :size="17" /><span>{{
            nativeMode ? '数据留在这台电脑' : '浏览器布局预览'
          }}</span>
        </div>
        <ThemeSelect />
        <div class="version">v{{ app.appVersion }} <span>本地工作台</span></div>
      </div>
    </aside>
    <main class="main-content">
      <header class="topbar">
        <div class="breadcrumb">
          我的工作台 <span>/</span
          >{{
            mode === 'week'
              ? '周概览'
              : mode === 'calendar'
                ? '月历'
                : isTaskWorkspace
                  ? '任务'
                  : current.title
          }}
        </div>
        <AppButton
          variant="ghost"
          :disabled="store.refreshing"
          @click="mode === 'tasks' || mode === 'week' ? store.refresh() : queryRefresh++"
          >{{ store.refreshing ? '刷新中…' : '刷新任务' }}</AppButton
        >
      </header>
      <section class="content-body" :class="{ 'week-content': mode === 'week' }">
        <p v-if="mode === 'week' && notice" class="success-note" role="status">{{ notice }}</p>
        <WeekOverview
          v-if="mode === 'week'"
          :tasks="store.tasks"
          :categories="store.categories"
          :today="store.today"
          :clock="store.now"
          :loading="!store.ready || store.refreshing"
          @open="edit"
          @create="create"
        />
        <CalendarView
          v-else-if="mode === 'calendar'"
          :categories="store.categories"
          :revision="store.revision + queryRefresh"
          :today="store.today"
          :clock="store.now"
          @open="edit"
          @create="create"
        />
        <template v-else>
          <div class="page-heading">
            <div>
              <p class="eyebrow">{{ date }}</p>
              <h1>
                {{ isTaskWorkspace ? '任务' : current.title }}<span class="title-dot">.</span>
              </h1>
            </div>
            <AppButton variant="primary" :disabled="!store.ready" @click="create()"
              ><AppIcon name="plus" :size="18" />{{
                selected === 'inbox' ? '添加心愿' : '新建任务'
              }}</AppButton
            >
          </div>
          <div v-if="isTaskWorkspace" class="task-view-tabs" role="group" aria-label="任务时间范围">
            <button
              v-for="view in taskViews"
              :key="view.id"
              :aria-pressed="selected === view.id"
              @click="navigate(view.id)"
            >
              {{ view.title }}<span>{{ store.view(view.id).length }}</span>
            </button>
          </div>
          <ReminderCenter
            v-if="selected !== 'inbox'"
            @open="
              (id) => {
                const task = store.tasks.find((t) => t.id === id);
                if (task) edit(task);
              }
            "
          />
          <p v-if="notice" class="success-note" role="status">{{ notice }}</p>
          <div v-if="visible.length || categoryFilter" class="section-heading list-heading">
            <div v-if="visible.length">
              <strong>{{ selected === 'inbox' ? '我的心愿' : '任务清单' }}</strong
              ><span class="muted small">{{ visible.length }} 项</span>
            </div>
            <label class="filter-label"
              >分类筛选<select v-model="categoryFilter">
                <option value="">全部分类</option>
                <option value="uncategorized">无分类</option>
                <option
                  v-for="category in store.categories"
                  :key="category.id"
                  :value="category.id"
                >
                  {{ category.name }}
                </option>
              </select></label
            >
          </div>
          <div v-if="!store.ready || visible.length" class="task-surface task-surface-live">
            <p v-if="!store.ready" class="loading-copy">
              {{ store.error ? '任务服务暂不可用，请刷新重试。' : '正在读取本地任务…' }}
            </p>
            <TaskList v-else-if="visible.length" :tasks="visible" @open="edit" />
          </div>
        </template>
      </section>
    </main>
    <DetailDrawer
      v-if="drawer"
      :key="selectedId ?? 'new'"
      :task="selectedTask"
      :planned-date="createDate"
      :wishlist="createDate === null"
      @close="drawer = false"
      @saved="saved"
      @deleted="
        drawer = false;
        notice = '任务已永久删除。';
      "
    />
    <SettingsPanel v-if="settingsOpen" @close="settingsOpen = false" />
    <CategoryManager v-if="categoriesOpen" @close="categoriesOpen = false" />
  </div>
</template>
