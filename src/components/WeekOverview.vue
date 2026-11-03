<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { Category, Task } from '../task-types';
import { groupWeek, mondayOf, shiftDate, type WeekStatus } from '../domain/weeks';
import { isTimedOut, validDate } from '../domain/tasks';
import AppButton from './AppButton.vue';
import TaskList from './TaskList.vue';

const props = defineProps<{
  tasks: Task[];
  categories: Category[];
  today: string;
  clock: number;
  loading?: boolean;
}>();
const emit = defineEmits<{ open: [task: Task]; create: [date: string] }>();
const anchor = ref(props.today),
  followingCurrent = ref(true);
const category = ref<string | null>(null),
  status = ref<WeekStatus>('all');
const expandedDays = ref(new Set<string>());
const start = computed(() => mondayOf(anchor.value));
const end = computed(() => shiftDate(start.value, 6));
const isCurrent = computed(() => start.value === mondayOf(props.today));
const days = computed(() =>
  groupWeek(props.tasks, anchor.value, props.today, props.clock, category.value, status.value),
);
const summary = computed(() => {
  const tasks = days.value.flatMap((day) => day.tasks);
  const completed = tasks.filter((task) => task.status === 'completed').length;
  return {
    total: tasks.length,
    completed,
    pending: tasks.length - completed,
    timedOut: tasks.filter((task) => isTimedOut(task, props.today, props.clock)).length,
  };
});
function browse(delta: number) {
  const next = shiftDate(start.value, delta * 7);
  if (!validDate(next)) return;
  anchor.value = next;
  followingCurrent.value = false;
}
function returnCurrent() {
  anchor.value = props.today;
  followingCurrent.value = true;
}
function toggleDay(date: string) {
  const next = new Set(expandedDays.value);
  if (next.has(date)) next.delete(date);
  else next.add(date);
  expandedDays.value = next;
}
function canCreate(date: string) {
  return !props.loading && validDate(date) && date >= props.today;
}
watch(
  () => props.today,
  (today) => {
    if (followingCurrent.value) anchor.value = today;
  },
);
watch([start, category, status], () => {
  expandedDays.value = new Set();
});
watch(
  () => props.categories,
  (categories) => {
    if (
      category.value &&
      category.value !== 'uncategorized' &&
      !categories.some((item) => item.id === category.value)
    )
      category.value = null;
  },
);
</script>

<template>
  <section class="week-overview" aria-label="周一至周日任务概览">
    <header class="week-heading">
      <div>
        <h1>周概览</h1>
        <span class="week-range"
          >{{ start.replaceAll('-', '/') }} — {{ end.replaceAll('-', '/')
          }}<span v-if="isCurrent"> · 本周</span></span
        >
      </div>
      <div class="week-navigation">
        <AppButton :disabled="!validDate(shiftDate(start, -7))" @click="browse(-1)"
          >上一周</AppButton
        ><AppButton @click="returnCurrent">本周</AppButton
        ><AppButton :disabled="!validDate(shiftDate(start, 7))" @click="browse(1)"
          >下一周</AppButton
        >
      </div>
    </header>
    <div class="week-toolbar">
      <select v-model="category" aria-label="周任务分类">
        <option :value="null">全部分类</option>
        <option value="uncategorized">无分类</option>
        <option v-for="item in categories" :key="item.id" :value="item.id">{{ item.name }}</option>
      </select>
      <select v-model="status" aria-label="周任务状态">
        <option value="all">全部状态</option>
        <option value="pending">未完成</option>
        <option value="completed">已完成</option>
      </select>
      <span v-if="summary.total" class="week-totals"
        >{{ summary.total }}项 · 已完成{{ summary.completed
        }}<span v-if="summary.timedOut" class="week-timeout">
          · 超时{{ summary.timedOut }}</span
        ></span
      >
    </div>
    <p v-if="loading" class="muted small" role="status">读取中…</p>
    <div class="week-rows">
      <section
        v-for="day in days"
        :key="day.date"
        class="week-day-row"
        :class="{ 'week-day-today': day.date === today }"
        :aria-label="`${day.date} ${day.label}`"
      >
        <div class="week-date">
          <strong>{{ day.label }}</strong
          ><span>{{ day.date.split('-').slice(1).join('.') }}</span>
        </div>
        <div class="week-day-body">
          <TaskList
            v-if="day.tasks.length"
            :tasks="expandedDays.has(day.date) ? day.tasks : day.tasks.slice(0, 2)"
            dense
            @open="emit('open', $event)"
          />
        </div>
        <div class="week-day-tools">
          <button
            v-if="day.tasks.length > 2"
            type="button"
            :aria-expanded="expandedDays.has(day.date)"
            :aria-label="`${expandedDays.has(day.date) ? '收起' : '展开'}${day.label}其余任务`"
            @click="toggleDay(day.date)"
          >
            {{ expandedDays.has(day.date) ? '收起' : `+${day.tasks.length - 2}` }}
          </button>
          <button
            v-if="canCreate(day.date)"
            type="button"
            :aria-label="`在 ${day.date} 添加任务`"
            :title="`在 ${day.date} 添加任务`"
            @click="emit('create', day.date)"
          >
            ＋
          </button>
        </div>
      </section>
    </div>
  </section>
</template>

<style scoped>
.week-overview {
  display: grid;
  gap: 10px;
  min-width: 0;
}
.week-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.week-heading h1 {
  display: inline;
  font-size: 23px;
  letter-spacing: -0.5px;
  margin-right: 12px;
}
.week-range {
  display: inline-block;
  color: var(--muted);
  font-size: 11px;
}
.week-navigation {
  display: flex;
  gap: 5px;
}
.week-navigation .app-button {
  min-height: 29px;
  padding: 4px 8px;
  font-size: 11px;
}
.week-toolbar {
  display: flex;
  align-items: center;
  gap: 7px;
  flex-wrap: wrap;
}
.week-toolbar select {
  max-width: 160px;
  min-width: 0;
  padding: 5px 7px;
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  background: var(--surface);
  font: inherit;
  font-size: 11px;
}
.week-totals {
  margin-left: auto;
  font-size: 11px;
  color: var(--muted);
}
.week-timeout {
  color: var(--error-text);
}
.week-rows {
  border-top: 1px solid var(--border);
}
.week-day-row {
  display: grid;
  grid-template-columns: 62px minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  min-height: 44px;
  padding: 5px 0;
  border-bottom: 1px solid var(--border);
}
.week-day-today .week-date {
  color: var(--accent);
}
.week-date {
  display: grid;
  gap: 3px;
  font-size: 17px;
}
.week-date strong {
  font-weight: 650;
}
.week-date span {
  font-size: 11px;
  color: var(--muted);
}
.week-day-body {
  min-width: 0;
}
.week-day-tools {
  display: flex;
  align-items: center;
  gap: 4px;
}
.week-day-tools button {
  min-width: 24px;
  min-height: 24px;
  padding: 2px 4px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--muted);
  font: inherit;
  font-size: 11px;
}
.week-day-tools button:hover {
  color: var(--accent);
  background: var(--accent-soft);
}
@media (max-width: 450px) {
  .week-day-row {
    grid-template-columns: 46px minmax(0, 1fr) auto;
    gap: 5px;
  }
}
</style>
