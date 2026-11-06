<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted, onBeforeUnmount } from 'vue';
import AppIcon from '../components/AppIcon.vue';
import AppButton from '../components/AppButton.vue';
import TaskList from '../components/TaskList.vue';
import { bridge } from '../services/bridge';
import { useAppStore } from '../stores/app';
import { useTasksStore } from '../stores/tasks';
import { widgetTasks, isTimedOut } from '../domain/tasks';
const app = useAppStore(),
  store = useTasksStore();
const expanded = ref(false);
const locking = ref(false);
const fontStyle = computed(() => {
  const fonts = app.desktop;
  return {
    '--widget-task-font': `${fonts.widgetFontSize}px`,
    '--widget-compact-font': `${fonts.widgetCompactFontSize}px`,
    '--widget-date-font': `${fonts.widgetDateFontSize}px`,
    '--widget-week-font': `${fonts.widgetWeekFontSize}px`,
  };
});
const todayTasks = computed(() => widgetTasks(store.tasks, store.today, store.now, 'today'));
const futureTasks = computed(() => widgetTasks(store.tasks, store.today, store.now, 'plan'));
const visibleTasks = computed(() => [...todayTasks.value, ...futureTasks.value]);
const surface = ref<HTMLElement>(),
  header = ref<HTMLElement>(),
  content = ref<HTMLElement>(),
  textProbe = ref<HTMLElement>();
let lockBoundsKey = '';
async function syncLockBounds() {
  const button = surface.value?.querySelector('.widget-lock');
  if (!button || !app.ready || disposed) return;
  const rect = button.getBoundingClientRect();
  if (!rect.width || !rect.height) return;
  const bounds = { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
  const key = JSON.stringify(bounds);
  if (key === lockBoundsKey) return true;
  try {
    await bridge.widgetLockBounds(bounds);
    lockBoundsKey = key;
    return true;
  } catch (error) {
    lockBoundsKey = '';
    if (!disposed) app.fail(error);
    return false;
  }
}
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
  await syncLockBounds();
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
    for (const task of visibleTasks.value) {
      titleWidth = Math.max(
        titleWidth,
        textWidth(
          `${isTimedOut(task, store.today, store.now) ? '超时 · ' : ''}${task.title.replace(/\s+/g, ' ')}`,
          titleElement,
        ),
      );
    }
    probe.textContent = '';
    const topStyle = getComputedStyle(top);
    const headerWidth =
      Array.from(top.children).reduce(
        (sum, child) => sum + child.getBoundingClientRect().width,
        0,
      ) +
      number(topStyle.columnGap) * 2;
    const gutter = panel.offsetWidth - panel.clientWidth;
    lastWidth = Math.min(
      180,
      Math.max(
        120,
        Math.ceil(
          number(css.paddingLeft) +
            number(css.paddingRight) +
            Math.max(headerWidth, titleWidth + gutter) +
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
  // Both modes size to the real content. The ceiling only stops a long list from
  // growing past the screen work area; it is not a fixed window height.
  const workArea = Math.round(((window.screen?.availHeight || 900) * 7) / 10);
  const limit = Math.min(mode ? 800 : 600, Math.max(workArea, Math.ceil(fixedHeight + 24)));
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
    visibleTasks,
    fontStyle,
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
const completed = computed(
  () => todayTasks.value.filter((task) => task.status === 'completed').length,
);
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
async function windowAction(action: 'drag' | 'resize' | 'hide' | 'lock' | 'unlock') {
  const changesLock = action === 'lock' || action === 'unlock';
  if (changesLock && locking.value) return;
  if (changesLock) locking.value = true;
  try {
    if (changesLock && (await syncLockBounds()) === false) return;
    await bridge.widgetAction(action);
  } catch (error) {
    app.fail(error);
  } finally {
    if (changesLock) locking.value = false;
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
          aria-label="刷新任务"
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
          :aria-label="app.desktop.widgetLocked ? '解锁悬浮窗' : '锁定悬浮窗'"
          :title="app.desktop.widgetLocked ? '点击解锁悬浮窗' : '锁定后鼠标穿透，再点锁形按钮解锁'"
          :aria-pressed="app.desktop.widgetLocked"
          :disabled="locking || store.busy || app.busy"
          @click="windowAction(app.desktop.widgetLocked ? 'unlock' : 'lock')"
          ><AppIcon :name="app.desktop.widgetLocked ? 'lock' : 'unlock'" :size="14"
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
    <div v-if="expanded && todayTasks.length" class="widget-summary">
      <span>今日完成 {{ completed }}/{{ todayTasks.length }}</span
      ><span class="muted">{{ Math.round((completed / todayTasks.length) * 100) }}%</span>
    </div>
    <progress
      v-if="expanded && todayTasks.length"
      class="widget-progress"
      :value="completed"
      :max="todayTasks.length"
      aria-label="今日完成进度"
    />
    <div
      id="widget-task-panel"
      class="widget-scroll"
      :class="{ 'has-tasks': visibleTasks.length }"
      role="region"
      aria-label="悬浮任务"
      tabindex="0"
    >
      <div ref="content" class="widget-content">
        <p v-if="!store.ready" class="muted small">正在读取任务…</p>
        <template v-else>
          <section v-if="todayTasks.length" class="widget-group" aria-label="今日">
            <h2 class="widget-group-heading">
              <span class="widget-group-title">今日</span>
            </h2>
            <TaskList :tasks="todayTasks" compact :minimal="!expanded" :inline-date="expanded" />
          </section>
          <section v-if="futureTasks.length" class="widget-group" aria-label="未来">
            <h2 class="widget-group-heading">
              <span class="widget-group-title">未来</span>
            </h2>
            <TaskList :tasks="futureTasks" compact :minimal="!expanded" :inline-date="expanded" />
          </section>
        </template>
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
