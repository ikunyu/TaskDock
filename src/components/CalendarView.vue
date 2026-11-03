<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue';
import type { Task, Category } from '../task-types';
import type { CalendarDay } from '../data-types';
import { emptySearch } from '../data-types';
import { dataBridge } from '../services/data';
import { localDate, overdue } from '../domain/tasks';
import { toAppError } from '../services/errors';
import AppButton from './AppButton.vue';
const props = defineProps<{
  categories: Category[];
  revision: number;
  today: string;
  clock: number;
}>();
const emit = defineEmits<{ open: [task: Task]; create: [date: string] }>();
const today = computed(() => props.today),
  month = ref(props.today.slice(0, 7)),
  selected = ref(props.today),
  category = ref<string | null>(null),
  summary = ref<CalendarDay[]>([]),
  tasks = ref<Task[]>([]),
  error = ref(''),
  busy = ref(false),
  offset = ref(0),
  hasMore = ref(false);
let generation = 0;
function local(y: number, m: number, d: number) {
  const x = new Date(0);
  x.setFullYear(y, m, d);
  x.setHours(12, 0, 0, 0);
  return x;
}
const cells = computed(() => {
  const [y, m] = month.value.split('-').map(Number) as [number, number];
  const count = local(y, m, 0).getDate(),
    leading = (local(y, m - 1, 1).getDay() + 6) % 7;
  return Array.from({ length: Math.ceil((leading + count) / 7) * 7 }, (_, i) => {
    const day = i - leading + 1;
    return day > 0 && day <= count
      ? { day, date: `${month.value}-${String(day).padStart(2, '0')}` }
      : null;
  });
});
const byDate = computed(() => new Map(summary.value.map((d) => [d.date, d])));
function shift(delta: number) {
  const [y, m] = month.value.split('-').map(Number) as [number, number];
  const date = local(y, m - 1 + delta, 1);
  if (date.getFullYear() < 1 || date.getFullYear() > 9999) return;
  month.value = localDate(date).slice(0, 7);
  selected.value = `${month.value}-01`;
}
async function load() {
  const own = ++generation;
  busy.value = true;
  error.value = '';
  try {
    const [days, result] = await Promise.all([
      dataBridge.calendar(month.value, category.value),
      dataBridge.search({
        ...emptySearch(),
        calendarDate: selected.value,
        categoryId: category.value,
        offset: offset.value,
      }),
    ]);
    if (own !== generation) return;
    summary.value = days;
    tasks.value = result.tasks;
    hasMore.value = result.hasMore;
  } catch (e) {
    if (own === generation) error.value = toAppError(e).message;
  } finally {
    if (own === generation) busy.value = false;
  }
}
watch(
  [month, selected, category, () => props.revision],
  () => {
    offset.value = 0;
    void load();
  },
  { immediate: true },
);
watch(
  () => props.clock,
  () => {
    void load();
  },
);
onUnmounted(() => {
  ++generation;
});
</script>
<template>
  <section class="data-view">
    <div class="section-heading">
      <h1>月历</h1>
      <AppButton
        @click="
          month = today.slice(0, 7);
          selected = today;
          offset = 0;
          load();
        "
        >返回今天</AppButton
      >
    </div>
    <div class="calendar-toolbar">
      <AppButton :disabled="month === '0001-01'" @click="shift(-1)">上一月</AppButton
      ><strong>{{ month }}</strong
      ><AppButton :disabled="month === '9999-12'" @click="shift(1)">下一月</AppButton
      ><label
        >分类<select v-model="category">
          <option :value="null">全部</option>
          <option value="uncategorized">无分类</option>
          <option v-for="c in categories" :key="c.id" :value="c.id">{{ c.name }}</option>
        </select></label
      >
    </div>
    <p v-if="error" class="form-error" role="alert">{{ error }}</p>
    <div class="calendar-grid">
      <span
        v-for="name in ['一', '二', '三', '四', '五', '六', '日']"
        :key="name"
        class="calendar-weekday"
        >{{ name }}</span
      ><template v-for="(cell, i) in cells" :key="i"
        ><button
          v-if="cell"
          class="calendar-cell"
          :class="{
            selected: selected === cell.date,
            today: today === cell.date,
            late: !!byDate.get(cell.date)?.overdue,
          }"
          :aria-label="`${cell.date}，${byDate.get(cell.date)?.total ?? 0}项任务`"
          :aria-pressed="selected === cell.date"
          @click="selected = cell.date"
        >
          <strong>{{ cell.day }}</strong
          ><template v-if="byDate.get(cell.date)"
            ><span>{{ byDate.get(cell.date)!.total }}项</span
            ><small v-if="byDate.get(cell.date)!.completed" class="calendar-done"
              >✓ {{ byDate.get(cell.date)!.completed }}</small
            ><small v-if="byDate.get(cell.date)!.high" class="priority-high"
              >高 {{ byDate.get(cell.date)!.high }}</small
            ><small v-if="byDate.get(cell.date)!.overdue" class="overdue-badge"
              >逾期 {{ byDate.get(cell.date)!.overdue }}</small
            ></template
          ></button
        ><span v-else class="calendar-blank"
      /></template>
    </div>
    <div class="section-heading">
      <h3>{{ selected }}</h3>
      <AppButton variant="primary" :disabled="selected < today" @click="emit('create', selected)"
        >在此日期添加任务</AppButton
      >
    </div>
    <p v-if="busy" role="status">读取中…</p>
    <div v-if="tasks.length" class="query-results">
      <article v-for="task in tasks" :key="task.id">
        <button class="task-title-button" @click="emit('open', task)">{{ task.title }}</button>
        <div class="task-metadata">
          <span :class="`priority-${task.priority}`">{{
            task.priority === 'high'
              ? '高优先级'
              : task.priority === 'medium'
                ? '中优先级'
                : task.priority === 'low'
                  ? '低优先级'
                  : '普通'
          }}</span
          ><span>{{ task.status === 'completed' ? '✓ 已完成' : '未完成' }}</span
          ><span v-if="task.schedule?.seriesId">↻ 重复</span
          ><span v-if="overdue(task, today, clock)" class="overdue-badge">{{
            overdue(task, today, clock)
          }}</span>
        </div>
      </article>
    </div>
    <div v-if="tasks.length || offset > 0" class="pagination">
      <AppButton
        :disabled="busy || offset === 0"
        @click="
          offset -= 50;
          load();
        "
        >上一页</AppButton
      ><span>第{{ Math.floor(offset / 50) + 1 }}页</span
      ><AppButton
        :disabled="busy || !hasMore"
        @click="
          offset += 50;
          load();
        "
        >下一页</AppButton
      >
    </div>
  </section>
</template>
