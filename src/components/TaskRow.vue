<script setup lang="ts">
import { computed } from 'vue';
import type { Task, Category } from '../task-types';
import { overdue, isTimedOut, priorityLabels, formatInstant, localDate } from '../domain/tasks';
import AppButton from './AppButton.vue';
import AppIcon from './AppIcon.vue';
const props = defineProps<{
  task: Task;
  categories: Category[];
  today: string;
  now: number;
  compact?: boolean;
  minimal?: boolean;
  dense?: boolean;
  inlineDate?: boolean;
  busy?: boolean;
}>();
const emit = defineEmits<{
  toggle: [task: Task];
  open: [task: Task];
  delete: [task: Task];
}>();
const category = computed(() => props.categories.find((item) => item.id === props.task.categoryId));
const late = computed(() => overdue(props.task, props.today, props.now));
const timedOut = computed(() => isTimedOut(props.task, props.today, props.now));
const wishlist = computed(() => props.task.plannedDate === null);
const inlineStamp = computed(() => {
  const task = props.task;
  const due = task.dueAt ? new Date(task.dueAt) : null;
  const dueDay = due && Number.isFinite(due.getTime()) ? localDate(due) : null;
  const date = task.plannedDate ?? task.dueDate ?? dueDay;
  if (!date) return null;
  const time =
    due && dueDay === date
      ? ` ${String(due.getHours()).padStart(2, '0')}:${String(due.getMinutes()).padStart(2, '0')}`
      : '';
  const title = [
    task.plannedDate ? `截止日期 ${task.plannedDate}` : '',
    task.dueDate && task.dueDate !== task.plannedDate ? `截止 ${task.dueDate}` : '',
    task.dueAt ? `截止 ${formatInstant(task.dueAt)}` : '',
  ]
    .filter(Boolean)
    .join('；');
  return { date, label: `${date.slice(5).replace('-', '.')}${time}`, title };
});
</script>
<template>
  <article
    class="task-row"
    :class="{
      'task-completed': task.status === 'completed',
      'task-compact': compact,
      'task-timed-out': timedOut,
      'task-dense': dense,
      'task-inline-date': inlineDate && !minimal,
    }"
  >
    <div class="task-row-content">
      <div v-if="inlineDate && !minimal" class="task-inline-heading">
        <span class="task-title" :title="task.title"
          >{{ timedOut ? '超时 · ' : '' }}{{ task.title }}</span
        >
        <time
          v-if="inlineStamp"
          class="task-inline-time"
          :datetime="inlineStamp.date"
          :title="inlineStamp.title"
          >{{ inlineStamp.label }}</time
        >
      </div>
      <span v-else-if="compact" class="task-title" :title="task.title"
        >{{ minimal && timedOut ? '超时 · ' : '' }}{{ task.title }}</span
      ><button
        v-else
        class="task-title task-title-button"
        :title="task.title"
        @click="emit('open', task)"
      >
        {{ task.title }}
      </button>
      <div v-if="!minimal && !dense && !inlineDate" class="task-metadata">
        <span v-if="task.schedule?.seriesId">↻ 重复</span>
        <span v-if="task.schedule?.hasNotified" class="overdue-badge">提醒待处理</span>
        <span v-else-if="task.schedule?.nextAt"
          >下次提醒 {{ formatInstant(task.schedule.nextAt) }}</span
        >
        <span v-if="late" class="overdue-badge">{{ late }}</span
        ><span v-if="task.plannedDate">{{ task.plannedDate }}</span
        ><span v-if="task.priority !== 'none'" :class="`priority-${task.priority}`">{{
          priorityLabels[task.priority]
        }}</span
        ><span v-if="category" class="category-tag"
          ><i :style="{ backgroundColor: category.color }" />{{ category.name }}</span
        ><span v-if="task.dueDate">截止 {{ task.dueDate }}</span
        ><span v-if="task.dueAt">截止 {{ formatInstant(task.dueAt) }}</span
        ><span v-if="task.status === 'completed' && !compact"
          >{{ wishlist ? '划掉于' : '完成于' }} {{ formatInstant(task.completedAt) }}</span
        >
      </div>
      <div v-if="!minimal && !dense && !inlineDate && task.progress.total" class="subtask-progress">
        <progress
          :value="task.progress.completed"
          :max="task.progress.total"
          :aria-label="`${task.title} 子任务进度`"
        /><span
          >{{ task.progress.completed }}/{{ task.progress.total }} ·
          {{ task.progress.percent }}%</span
        >
      </div>
    </div>
    <div v-if="!minimal" class="task-actions">
      <button
        type="button"
        role="checkbox"
        class="task-completion"
        :aria-checked="task.status === 'completed'"
        :disabled="busy"
        :aria-label="`${wishlist ? (task.status === 'completed' ? '撤销划掉心愿' : '划掉心愿') : task.status === 'completed' ? '撤销完成' : '完成任务'}：${task.title}`"
        :title="
          wishlist
            ? task.status === 'completed'
              ? '已划掉，点击撤销'
              : '点击划掉心愿'
            : task.status === 'completed'
              ? '已完成，点击撤销'
              : '点击标记完成'
        "
        @click="emit('toggle', task)"
      >
        <AppIcon v-if="task.status === 'completed'" name="check" :size="10" />
      </button>
      <AppButton
        v-if="!compact && !dense"
        class="task-delete"
        variant="ghost"
        :disabled="busy"
        :aria-label="`删除任务：${task.title}`"
        @click="emit('delete', task)"
        ><AppIcon name="close" :size="16"
      /></AppButton>
    </div>
  </article>
</template>
