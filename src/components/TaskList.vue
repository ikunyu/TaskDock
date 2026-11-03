<script setup lang="ts">
import { ref } from 'vue';
import type { Task } from '../task-types';
import { useTasksStore } from '../stores/tasks';
import TaskRow from './TaskRow.vue';
import ConfirmDialog from './ConfirmDialog.vue';
import DeleteTaskDialog from './DeleteTaskDialog.vue';
import { toAppError } from '../services/errors';
defineProps<{
  tasks: Task[];
  compact?: boolean;
  minimal?: boolean;
  dense?: boolean;
  inlineDate?: boolean;
}>();
const emit = defineEmits<{ open: [task: Task] }>();
const store = useTasksStore();
const confirming = ref<Task | null>(null);
const purging = ref<Task | null>(null),
  purgeError = ref('');
function askPurge(task: Task) {
  purgeError.value = '';
  purging.value = task;
}
async function purge() {
  if (!purging.value || store.busy) return;
  purgeError.value = '';
  try {
    await store.purge(purging.value);
    purging.value = null;
  } catch (error) {
    purgeError.value = toAppError(error).message;
  }
}
async function complete(task: Task, confirmed = false) {
  if (task.status === 'pending' && task.progress.completed < task.progress.total && !confirmed) {
    confirming.value = task;
    return;
  }
  try {
    await store.status(task, task.status === 'completed' ? 'pending' : 'completed', confirmed);
    confirming.value = null;
  } catch {
    confirming.value = null;
    /* The store exposes the error; keep the persisted task status. */
  }
}
</script>
<template>
  <div class="task-list">
    <TaskRow
      v-for="task in tasks"
      :key="task.id"
      :task="task"
      :categories="store.categories"
      :today="store.today"
      :now="store.now"
      :compact="compact"
      :minimal="minimal"
      :dense="dense"
      :inline-date="inlineDate"
      :busy="store.busy"
      @open="emit('open', $event)"
      @toggle="complete($event)"
      @delete="askPurge"
    />
  </div>
  <DeleteTaskDialog
    v-if="purging"
    :task="purging"
    :error="purgeError"
    :busy="store.busy"
    @cancel="purging = null"
    @confirm="purge"
  />
  <ConfirmDialog
    v-if="confirming"
    title="完成父任务"
    :message="`“${confirming.title}”还有未完成子任务，仍要完成父任务吗？子任务将保持原状态。`"
    confirm-label="仍然完成"
    :busy="store.busy"
    @cancel="confirming = null"
    @confirm="complete(confirming!, true)"
  />
</template>
