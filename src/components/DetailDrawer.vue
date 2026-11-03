<script setup lang="ts">
import { computed, reactive, ref } from 'vue';
import ScheduleEditor from './ScheduleEditor.vue';
import { emptySchedule } from '../schedule-types';
import type { Task, TaskDraft } from '../task-types';
import {
  blankDraft,
  taskDraft,
  validateDraft,
  validateCreationDates,
  toLocalDateTime,
  toUtcDateTime,
  formatInstant,
} from '../domain/tasks';
import { useTasksStore } from '../stores/tasks';
import { toAppError } from '../services/errors';
import DialogFrame from './DialogFrame.vue';
import ConfirmDialog from './ConfirmDialog.vue';
import DeleteTaskDialog from './DeleteTaskDialog.vue';
import AppButton from './AppButton.vue';
import DateField from './DateField.vue';
const props = defineProps<{ task: Task | null; plannedDate?: string | null; wishlist?: boolean }>();
const emit = defineEmits<{ close: []; saved: [id: string]; deleted: [] }>();
const store = useTasksStore();
const activeReminders = computed(() =>
  store.reminders.filter((r) => r.taskId === props.task?.id && r.state === 'notified'),
);
async function snooze(id: string, minutes: number) {
  try {
    await store.reminderAction(id, minutes);
    error.value = '';
  } catch (e) {
    error.value = toAppError(e).message;
  }
}
const draft = reactive<TaskDraft>(
  props.task
    ? taskDraft(props.task)
    : blankDraft(
        props.wishlist
          ? null
          : props.plannedDate && props.plannedDate >= store.today
            ? props.plannedDate
            : store.today,
      ),
);
const minimumDate = computed(() => (props.task ? '0001-01-01' : store.today));
const wishlist = ref(props.task ? props.task.plannedDate === null : !!props.wishlist);
const legacyScheduling = computed(
  () =>
    !!props.task &&
    (!!props.task.dueDate ||
      !!props.task.dueAt ||
      !!props.task.schedule?.config.at ||
      !!props.task.schedule?.config.atDeadline ||
      !!props.task.schedule?.config.reminderRepeat ||
      !!props.task.schedule?.config.beforeMinutes.length ||
      !!props.task.schedule?.config.recurrence),
);
const baseVersion = ref(props.task?.version ?? 0);
function deadlineFields(value: TaskDraft, creating = false) {
  const local = toLocalDateTime(value.dueAt);
  return {
    date: local ? local.slice(0, 10) : (value.dueDate ?? value.plannedDate ?? ''),
    time: local,
    kind: (value.dueAt
      ? 'time'
      : value.dueDate ||
          (value.plannedDate &&
            (creating || value.schedule?.atDeadline || value.schedule?.beforeMinutes.length))
        ? 'date'
        : 'none') as 'none' | 'date' | 'time',
  };
}
let savedDeadline = deadlineFields(draft, !props.task);
const deadlineDate = ref(savedDeadline.date),
  dueTime = ref(savedDeadline.time),
  dueKind = ref(savedDeadline.kind);
function setDeadlineDate(value: string | null) {
  deadlineDate.value = value ?? '';
  if (dueTime.value) dueTime.value = value ? `${value}T${dueTime.value.slice(11)}` : '';
}
function setDueKind(event: Event) {
  dueKind.value = (event.target as HTMLSelectElement).value as typeof dueKind.value;
  if (dueKind.value === 'time' && !dueTime.value && deadlineDate.value)
    dueTime.value = `${deadlineDate.value}T23:59`;
  if (dueKind.value === 'none' && draft.schedule) {
    draft.schedule = { ...draft.schedule, atDeadline: false, beforeMinutes: [] };
  }
}
function syncExactDate() {
  if (dueTime.value) deadlineDate.value = dueTime.value.slice(0, 10);
}
const error = ref(''),
  deleteError = ref(''),
  discard = ref(false),
  removing = ref(false);
function formState() {
  return JSON.stringify({
    draft,
    date: deadlineDate.value,
    time: dueTime.value,
    kind: dueKind.value,
  });
}
let baseline = formState();
const dirty = computed(() => formState() !== baseline);
const stale = computed(() => !!props.task && props.task.version !== baseVersion.value);
const previewProgress = computed(
  () => `${draft.subtasks.filter((item) => item.completed).length}/${draft.subtasks.length}`,
);
function close() {
  if (dirty.value) discard.value = true;
  else emit('close');
}
function move(index: number, direction: number) {
  const other = index + direction;
  if (other < 0 || other >= draft.subtasks.length) return;
  const item = draft.subtasks.splice(index, 1)[0];
  if (item) draft.subtasks.splice(other, 0, item);
}
async function reload() {
  await store.refresh();
  const latest = store.tasks.find((item) => item.id === props.task?.id);
  if (!latest || latest.deletedAt) {
    error.value = '此任务已被删除，当前草稿保留，请关闭抽屉返回任务列表。';
    return;
  }
  Object.assign(draft, taskDraft(latest));
  wishlist.value = latest.plannedDate === null;
  baseVersion.value = latest.version;
  savedDeadline = deadlineFields(draft);
  deadlineDate.value = savedDeadline.date;
  dueTime.value = savedDeadline.time;
  dueKind.value = savedDeadline.kind;
  baseline = formState();
  error.value = '';
}
async function save() {
  error.value = '';
  store.tick();
  try {
    if ((!wishlist.value || dueKind.value !== 'none') && !deadlineDate.value) {
      error.value = '请选择截止日期。';
      return;
    }
    if (dueKind.value === 'time' && !dueTime.value) {
      error.value = '请选择截止时间。';
      return;
    }
    const unchanged =
      !!props.task &&
      deadlineDate.value === savedDeadline.date &&
      dueTime.value === savedDeadline.time &&
      dueKind.value === savedDeadline.kind;
    const exactTime =
      dueKind.value === 'time'
        ? draft.dueAt && dueTime.value === toLocalDateTime(draft.dueAt)
          ? draft.dueAt
          : toUtcDateTime(dueTime.value)
        : null;
    const value: TaskDraft = {
      ...draft,
      // Preserve legacy independent date fields unless the user edits the date,
      // deadline mode or exact time. Date/time inputs share the same local day.
      plannedDate: wishlist.value
        ? null
        : unchanged
          ? draft.plannedDate
          : deadlineDate.value || null,
      categoryId: draft.categoryId || null,
      dueDate: unchanged
        ? draft.dueDate
        : dueKind.value === 'date'
          ? deadlineDate.value || null
          : null,
      dueAt: unchanged ? draft.dueAt : exactTime,
      subtasks: draft.subtasks.map((item) => ({ ...item })),
    };
    const invalid = validateDraft(value) || (!props.task ? validateCreationDates(value) : null);
    if (invalid) {
      error.value = invalid;
      return;
    }
    const result = props.task
      ? await store.update(props.task.id, baseVersion.value, value)
      : await store.create(value);
    if (result.taskId) emit('saved', result.taskId);
  } catch (value) {
    error.value = value instanceof Error ? value.message : toAppError(value).message;
  }
}
async function remove() {
  if (!props.task || store.busy) return;
  deleteError.value = '';
  try {
    await store.purge({ ...props.task, version: baseVersion.value });
    emit('deleted');
  } catch (value) {
    deleteError.value = toAppError(value).message;
  }
}
</script>
<template>
  <DialogFrame
    :title="wishlist ? (task ? '心愿详情' : '添加心愿') : task ? '任务详情' : '新建任务'"
    drawer
    :busy="store.busy"
    @close="close"
  >
    <form class="task-form" @submit.prevent="save">
      <p v-if="task" class="muted small">
        {{
          task.status === 'completed'
            ? `${wishlist ? '已划掉' : '已完成'} · ${formatInstant(task.completedAt)}`
            : wishlist
              ? '等待实现'
              : '进行中'
        }}
        · 版本 {{ baseVersion }}
      </p>
      <div v-if="stale" class="note-card">
        其他窗口已修改此任务，当前草稿已保留。<AppButton
          variant="ghost"
          :disabled="store.busy"
          @click="reload"
          >放弃草稿并重新载入</AppButton
        >
      </div>
      <label
        >{{ wishlist ? '心愿' : '任务标题'
        }}<input
          v-model="draft.title"
          required
          maxlength="500"
          :disabled="store.busy"
          :placeholder="wishlist ? '比如：买一台电脑' : '想完成什么？'"
      /></label>
      <label
        >任务描述<textarea
          v-model="draft.description"
          rows="3"
          maxlength="20000"
          :disabled="store.busy"
          placeholder="补充说明、步骤或想法…"
        />
      </label>
      <div class="form-grid">
        <DateField
          v-if="!wishlist"
          :model-value="deadlineDate"
          @update:model-value="setDeadlineDate"
          label="截止日期"
          :min="minimumDate"
          required
          :disabled="store.busy"
        />
        <label
          >优先级<select v-model="draft.priority" :disabled="store.busy">
            <option value="none">无优先级</option>
            <option value="low">低</option>
            <option value="medium">中</option>
            <option value="high">高</option>
          </select></label
        >
      </div>
      <template v-if="!wishlist || legacyScheduling">
        <DateField
          v-if="wishlist && (dueKind !== 'none' || draft.dueAt || draft.dueDate)"
          :model-value="deadlineDate"
          @update:model-value="setDeadlineDate"
          label="截止日期（旧记录）"
          :min="minimumDate"
          :disabled="store.busy"
        />
        <label
          >截止方式（最晚完成，可不设）<select
            :value="dueKind"
            :disabled="store.busy"
            @change="setDueKind"
          >
            <option value="none">无截止</option>
            <option value="date">仅日期（当天结束前）</option>
            <option value="time">精确时间（本地时间）</option>
          </select></label
        >
        <label v-if="dueKind === 'time'"
          >截止时间<input
            v-model="dueTime"
            type="datetime-local"
            required
            :min="`${minimumDate}T00:00`"
            max="9999-12-31T23:59"
            :disabled="store.busy"
            @input="syncExactDate"
        /></label>
      </template>
      <label
        >分类<select v-model="draft.categoryId" :disabled="store.busy">
          <option :value="null">无分类</option>
          <option v-for="category in store.categories" :key="category.id" :value="category.id">
            {{ category.name }}
          </option>
        </select></label
      >
      <template v-if="!wishlist || legacyScheduling">
        <div v-for="reminder in activeReminders" :key="reminder.id" class="note-card">
          <p>到期提醒 · {{ formatInstant(reminder.scheduledAt) }}</p>
          <div class="schedule-options">
            <AppButton
              v-for="[m, label] in [
                [10, '10分钟'],
                [30, '30分钟'],
                [60, '1小时'],
              ] as const"
              :key="m"
              :disabled="store.busy"
              @click="snooze(reminder.id, m)"
              >稍后{{ label }}</AppButton
            >
          </div>
        </div>
        <ScheduleEditor
          :minimum-date="minimumDate"
          :model-value="draft.schedule ?? emptySchedule()"
          :has-due="dueKind !== 'none' && !!deadlineDate"
          :busy="store.busy"
          @update:model-value="draft.schedule = $event"
        />
        <label v-if="task?.schedule?.seriesId"
          >修改范围<select v-model="draft.repeatScope" :disabled="store.busy">
            <option value="once">仅此一次（后续规则不变）</option>
            <option value="following">此次及此后所有实例</option>
          </select></label
        >
        <p v-if="task?.schedule?.seriesId" class="muted small">
          修改后续规则会保留完成历史，重新安排此后未完成实例。选择“不重复”并保存此次及此后，可结束重复。
        </p>
      </template>
      <div class="section-heading">
        <h3>
          子任务 <small class="muted">{{ previewProgress }}</small>
        </h3>
        <AppButton
          :disabled="store.busy || draft.subtasks.length >= 200"
          @click="draft.subtasks.push({ id: null, title: '', completed: false })"
          >添加子任务</AppButton
        >
      </div>
      <p class="muted small">全部子任务完成后，父任务仍由你手动完成。</p>
      <div class="subtask-editor">
        <div
          v-for="(item, index) in draft.subtasks"
          :key="item.id ?? `draft-${index}`"
          class="subtask-edit-row"
        >
          <input
            v-model="item.completed"
            type="checkbox"
            :disabled="store.busy"
            :aria-label="`子任务完成：${index + 1}`"
          /><input
            v-model="item.title"
            maxlength="500"
            required
            :disabled="store.busy"
            :aria-label="`子任务标题：${index + 1}`"
            placeholder="输入子任务…"
          /><button
            type="button"
            :disabled="store.busy || index === 0"
            :aria-label="`上移子任务：${index + 1}`"
            @click="move(index, -1)"
          >
            ↑</button
          ><button
            type="button"
            :disabled="store.busy || index === draft.subtasks.length - 1"
            :aria-label="`下移子任务：${index + 1}`"
            @click="move(index, 1)"
          >
            ↓</button
          ><button
            type="button"
            :disabled="store.busy"
            :aria-label="`删除子任务：${index + 1}`"
            @click="draft.subtasks.splice(index, 1)"
          >
            ×
          </button>
        </div>
      </div>
      <p v-if="error" class="form-error" role="alert">
        {{ error
        }}<AppButton v-if="task && !stale" variant="ghost" :disabled="store.busy" @click="reload"
          >重新载入任务</AppButton
        >
      </p>
      <div class="dialog-actions">
        <AppButton
          v-if="task"
          variant="danger"
          :disabled="store.busy"
          @click="
            deleteError = '';
            removing = true;
          "
          >删除任务</AppButton
        ><AppButton :disabled="store.busy" @click="close">取消</AppButton
        ><AppButton type="submit" variant="primary" :disabled="store.busy || stale">{{
          store.busy ? '正在保存…' : wishlist ? '保存心愿' : '保存任务'
        }}</AppButton>
      </div>
      <p v-if="dirty" class="muted small">有未保存的修改</p>
    </form>
  </DialogFrame>
  <ConfirmDialog
    v-if="discard"
    title="放弃修改"
    message="关闭抽屉将放弃未保存的草稿，已保存内容不会改变。"
    confirm-label="放弃修改"
    @cancel="discard = false"
    @confirm="emit('close')"
  />
  <DeleteTaskDialog
    v-if="removing && task"
    :task="task"
    :unsaved="dirty"
    :error="deleteError"
    :busy="store.busy"
    @cancel="removing = false"
    @confirm="remove"
  />
</template>
