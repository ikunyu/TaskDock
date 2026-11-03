<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted, onBeforeUnmount } from 'vue';
import AppIcon from '../components/AppIcon.vue';
import AppButton from '../components/AppButton.vue';
import TaskList from '../components/TaskList.vue';
import { bridge } from '../services/bridge';
import { useAppStore } from '../stores/app';
import { useTasksStore } from '../stores/tasks';
import { widgetTasks, isTimedOut } from '../domain/tasks';
import { groupTaskWeeks, mondayOf, shiftDate, type TaskWeek } from '../domain/weeks';
const app = useAppStore(),
  store = useTasksStore();
const selectedView = ref<'today' | 'plan'>('today');
const expanded = ref(false);
const locking = ref(false);
const fontStyle = computed(() => {
  const fonts = app.desktop;
  return {
    '--widget-task-font': `${fonts.widgetFontSize}px`,
    '--widget-compact-font': `${fonts.widgetCompactFontSize}px`,
    '--widget-date-font': `${fonts.widgetDateFontSize}px`,
    '--widget-week-font': `${fonts.widgetWeekFontSize}px`,
    '--widget-week-date-font': `${fonts.widgetWeekDateFontSize}px`,
  };
});
const activeView = computed(() => selectedView.value);
const tabs = [
  { id: 'today', label: '今日' },
  { id: 'plan', label: '未来' },
] as const;
const tasks = computed(() => widgetTasks(store.tasks, store.today, store.now, activeView.value));
const futureWeeks = computed(() =>
  activeView.value === 'plan' ? groupTaskWeeks(tasks.value) : [],
);
function weekPrefix(week: TaskWeek) {
  const current = mondayOf(store.today);
  return week.start === current ? '本周' : week.start === shiftDate(current, 7) ? '下周' : '';
}
function weekRange(week: TaskWeek) {
  const showYear =
    week.start.slice(0, 4) !== store.today.slice(0, 4) ||
    week.end.slice(0, 4) !== week.start.slice(0, 4);
  const format = (date: string) => (showYear ? date : date.slice(5)).replaceAll('-', '.');
  return `${format(week.start)}–${format(week.end)}`;
}
function weekLabel(week: TaskWeek) {
  return `${weekPrefix(week)} ${weekRange(week)}`.trim();
}
const surface = ref<HTMLElement>(),
  header = ref<HTMLElement>(),
  content = ref<HTMLElement>(),
  textProbe = ref<HTMLElement>();
let layoutQueue = Promise.resolve(),
  layoutRequest = 0,
  nativeExpanded = true,
  disposed = false;
let requestedKey = '',
  lastWidth = 160,
  lastHeight = 54;
let measureTimer: ReturnType<typeof setTimeout> | undefined;
let observer: ResizeObserver | undefined;
function scheduleLayout() {
  if (disposed) return;
  clearTimeout(measureTimer);
  measureTimer = setTimeout(() => {
    void syncLayout();
  }, 0);
}
async function syncLayout() {
  await nextTick();
  if (
    disposed ||
    !app.ready ||
    !surface.value ||
    !header.value ||
    !content.value ||
    !textProbe.value
  )
    return;
  const mode = expanded.value;
  const root = surface.value,
    top = header.value,
    body = content.value,
    probe = textProbe.value;
  const panel = body.parentElement!;
  const number = (value: string) => Number.parseFloat(value) || 0;
  const outerHeight = (element: Element) => {
    const css = getComputedStyle(element);
    return (
      element.getBoundingClientRect().height + number(css.marginTop) + number(css.marginBottom)
    );
  };
  const css = getComputedStyle(root),
    panelCss = getComputedStyle(panel);
  if (!mode) {
    const textWidth = (text: string, element: Element) => {
      probe.style.font = getComputedStyle(element).font;
      probe.textContent = text;
      return probe.getBoundingClientRect().width;
    };
    const titleElement = root.querySelector('.task-title') ?? body;
    let titleWidth = 0;
    for (const task of tasks.value) {
      titleWidth = Math.max(
        titleWidth,
        textWidth(
          `${isTimedOut(task, store.today, store.now) ? '超时 · ' : ''}${task.title.replace(/\s+/g, ' ')}`,
          titleElement,
        ),
      );
    }
    let weekWidth = 0;
    const period = root.querySelector('.widget-week-period'),
      dates = root.querySelector('.widget-week-dates');
    if (dates) {
      for (const week of futureWeeks.value) {
        const prefix = weekPrefix(week);
        weekWidth = Math.max(
          weekWidth,
          textWidth(weekRange(week), dates) +
            (prefix && period ? textWidth(prefix, period) + 4 : 0),
        );
      }
    }
    const tab = root.querySelector('.widget-tabs button');
    const tabWidth = tab ? Math.max(...tabs.map((item) => textWidth(item.label, tab))) * 2 + 32 : 0;
    probe.textContent = '';
    const topStyle = getComputedStyle(top);
    const headerWidth =
      Array.from(top.children).reduce(
        (sum, child) => sum + child.getBoundingClientRect().width,
        0,
      ) +
      number(topStyle.columnGap) * 2;
    const gutter = panel.offsetWidth - panel.clientWidth;
    const groupInset = futureWeeks.value.length ? 10 : 0;
    lastWidth = Math.min(
      180,
      Math.max(
        120,
        Math.ceil(
          number(css.paddingLeft) +
            number(css.paddingRight) +
            Math.max(headerWidth, tabWidth, Math.max(titleWidth, weekWidth) + groupInset + gutter) +
            2,
        ),
      ),
    );
  }
  // Both modes include every visible control. Tabs remain visible in compact mode.
  const banners = Array.from(root.parentElement?.children ?? [])
    .filter((element) => element !== root)
    .reduce((sum, element) => sum + outerHeight(element), 0);
  const controls = Array.from(root.children)
    .filter((element) => {
      if (element === panel) return false;
      const position = getComputedStyle(element).position;
      return position !== 'absolute' && position !== 'fixed';
    })
    .reduce((sum, element) => sum + outerHeight(element), 0);
  const panelSpacing =
    number(panelCss.marginTop) +
    number(panelCss.marginBottom) +
    number(panelCss.paddingTop) +
    number(panelCss.paddingBottom) +
    number(panelCss.borderTopWidth) +
    number(panelCss.borderBottomWidth);
  const fixedHeight =
    banners + controls + panelSpacing + number(css.paddingTop) + number(css.paddingBottom);
  const limit = Math.min(600, Math.max(mode ? 320 : 280, Math.ceil(fixedHeight + 24)));
  lastHeight = Math.min(
    limit,
    Math.max(mode ? 120 : 72, Math.ceil(fixedHeight + body.scrollHeight + 2)),
  );
  const key = mode ? `expanded:${lastHeight}` : `${lastWidth}:${lastHeight}`;
  if (key === requestedKey) return;
  requestedKey = key;
  const request = ++layoutRequest,
    width = lastWidth,
    height = lastHeight;
  layoutQueue = layoutQueue.then(async () => {
    if (disposed || request !== layoutRequest) return;
    try {
      await bridge.widgetLayout(mode, width, height);
      nativeExpanded = mode;
    } catch (error) {
      if (!disposed && request === layoutRequest) {
        requestedKey = '';
        expanded.value = nativeExpanded;
        app.fail(error);
      }
    }
  });
}
watch(
  [
    expanded,
    tasks,
    fontStyle,
    activeView,
    () => store.ready,
    () => app.ready,
    () => app.error,
    () => store.error,
  ],
  scheduleLayout,
  { flush: 'post' },
);
onMounted(() => {
  if (typeof ResizeObserver !== 'undefined') {
    observer = new ResizeObserver(scheduleLayout);
    if (content.value) observer.observe(content.value);
    if (header.value) observer.observe(header.value);
    if (surface.value) observer.observe(surface.value);
  }
  void document.fonts?.ready.then(scheduleLayout);
  scheduleLayout();
});
onBeforeUnmount(() => {
  disposed = true;
  layoutRequest++;
  observer?.disconnect();
  clearTimeout(measureTimer);
});
function switchTab(event: KeyboardEvent) {
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key) || store.busy) return;
  event.preventDefault();
  selectedView.value =
    event.key === 'Home'
      ? 'today'
      : event.key === 'End'
        ? 'plan'
        : selectedView.value === 'today'
          ? 'plan'
          : 'today';
  (event.currentTarget as HTMLElement)
    .querySelector<HTMLButtonElement>(`#widget-tab-${selectedView.value}`)
    ?.focus();
}
const completed = computed(() => tasks.value.filter((task) => task.status === 'completed').length);
const date = computed(() =>
  new Date(`${store.today}T12:00:00`).toLocaleDateString('zh-CN', {
    month: 'numeric',
    day: 'numeric',
    weekday: 'short',
  }),
);
async function openQuick() {
  try {
    await bridge.showWindow('quick-entry');
  } catch (error) {
    app.fail(error);
  }
}
async function windowAction(action: 'drag' | 'resize' | 'hide' | 'lock') {
  if (action === 'lock' && locking.value) return;
  if (action === 'lock') locking.value = true;
  try {
    await bridge.widgetAction(action);
  } catch (error) {
    app.fail(error);
  } finally {
    if (action === 'lock') locking.value = false;
  }
}
function drag(event: MouseEvent) {
  if (
    event.button === 0 &&
    !(event.target as HTMLElement).closest('button,input,select,textarea,a')
  ) {
    event.preventDefault();
    void windowAction('drag');
  }
}
function resize(event: MouseEvent) {
  if (event.button === 0) void windowAction('resize');
}
</script>
<template>
  <main
    ref="surface"
    class="widget-layout"
    :class="{ 'widget-collapsed': !expanded }"
    :style="fontStyle"
  >
    <header ref="header" class="widget-header" @mousedown="drag">
      <span class="mini-brand"><span v-if="expanded" class="brand-mark tiny">✓</span>任务</span>
      <div v-if="expanded" class="widget-header-actions">
        <span v-if="expanded" class="muted small widget-date">{{ date }}</span
        ><AppButton
          v-if="expanded"
          variant="ghost"
          :disabled="store.refreshing"
          :aria-label="activeView === 'today' ? '刷新今日任务' : '刷新未来事项'"
          @click="store.refresh()"
          >↻</AppButton
        >
        <AppButton
          v-if="expanded"
          variant="ghost"
          aria-label="隐藏悬浮窗"
          @click="windowAction('hide')"
          >×</AppButton
        >
      </div>
      <div class="widget-header-controls">
        <AppButton
          variant="ghost"
          class="widget-lock"
          aria-label="锁定悬浮窗"
          title="锁定后鼠标穿透，右键系统托盘可解锁"
          :aria-pressed="app.desktop.widgetLocked"
          :disabled="locking || store.busy || app.busy"
          @click="windowAction('lock')"
          ><AppIcon name="lock" :size="13"
        /></AppButton>
        <AppButton
          variant="ghost"
          class="widget-expand"
          :aria-expanded="expanded"
          aria-controls="widget-task-panel"
          :disabled="store.busy"
          @click="expanded = !expanded"
          >{{ expanded ? '收起' : '展开' }}</AppButton
        >
      </div>
    </header>
    <div class="widget-tabs" role="tablist" aria-label="悬浮任务范围" @keydown="switchTab">
      <button
        v-for="tab in tabs"
        :id="`widget-tab-${tab.id}`"
        :key="tab.id"
        type="button"
        role="tab"
        :aria-selected="selectedView === tab.id"
        aria-controls="widget-task-panel"
        :tabindex="selectedView === tab.id ? 0 : -1"
        :disabled="store.busy"
        @click="selectedView = tab.id"
      >
        {{ tab.label }}
      </button>
    </div>
    <div v-if="expanded && tasks.length" class="widget-summary">
      <span>{{
        selectedView === 'today'
          ? `今日完成 ${completed}/${tasks.length}`
          : `未来 ${tasks.length} 项`
      }}</span
      ><span v-if="selectedView === 'today'" class="muted"
        >{{ tasks.length ? Math.round((completed / tasks.length) * 100) : 0 }}%</span
      >
    </div>
    <progress
      v-if="expanded && activeView === 'today' && tasks.length"
      class="widget-progress"
      :value="completed"
      :max="tasks.length || 1"
      aria-label="今日完成进度"
    />
    <div
      id="widget-task-panel"
      class="widget-scroll"
      :class="{ 'has-tasks': tasks.length }"
      role="tabpanel"
      :aria-labelledby="`widget-tab-${activeView}`"
      tabindex="0"
    >
      <div ref="content" class="widget-content">
        <p v-if="!store.ready" class="muted small">正在读取任务…</p>
        <template v-else-if="activeView === 'plan'">
          <section
            v-for="week in futureWeeks"
            :key="week.start"
            class="widget-week-group"
            :aria-label="weekLabel(week)"
          >
            <h2>
              <span v-if="weekPrefix(week)" class="widget-week-period">{{ weekPrefix(week) }}</span
              ><span class="widget-week-dates">{{ weekRange(week) }}</span>
            </h2>
            <TaskList :tasks="week.tasks" compact :minimal="!expanded" :inline-date="expanded" />
          </section>
        </template>
        <TaskList
          v-else-if="tasks.length"
          :key="activeView"
          :tasks="tasks"
          compact
          :minimal="!expanded"
          :inline-date="expanded"
        />
      </div>
    </div>
    <AppButton v-if="expanded" class="widget-add" @click="openQuick"
      ><AppIcon name="plus" :size="17" />快速添加任务</AppButton
    >
    <button
      v-if="expanded"
      class="widget-resize"
      aria-label="调整悬浮窗大小"
      @mousedown.prevent.stop="resize"
    />
    <span ref="textProbe" class="widget-text-probe" aria-hidden="true" />
  </main>
</template>
