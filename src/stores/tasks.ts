import type { Reminder } from '../schedule-types';
import { computed, ref, watch } from 'vue';
import { defineStore } from 'pinia';
import { taskBridge } from '../services/tasks';
import { toAppError } from '../services/errors';
import { localDate, tasksForView } from '../domain/tasks';
import type { AppError } from '../types';
import type {
  Category,
  CategoryDraft,
  MutationResult,
  Task,
  TaskDraft,
  TaskSnapshot,
  TaskStatus,
  TaskView,
} from '../task-types';

export const useTasksStore = defineStore('tasks', () => {
  const tasks = ref<Task[]>([]),
    categories = ref<Category[]>([]),
    revision = ref(-1);
  const reminders = ref<Reminder[]>([]);
  const ready = ref(false),
    refreshing = ref(false),
    busy = ref(false),
    error = ref<AppError | null>(null);
  const today = ref(localDate()),
    now = ref(Date.now());
  const todayTasks = computed(() => tasksForView(tasks.value, 'today', today.value, now.value));
  let stop: (() => void) | undefined,
    generation = 0,
    wanted = -1;
  let flight: Promise<void> | undefined, clock: ReturnType<typeof setInterval> | undefined;
  let deadlineClock: ReturnType<typeof setTimeout> | undefined;
  let errorKind: 'read' | 'write' | null = null;

  // One timer for the next deadline/midnight, plus the existing periodic/focus
  // tick for sleep or system-clock changes. No database writes or extra polling.
  function scheduleDeadline() {
    clearTimeout(deadlineClock);
    if (!clock) return;
    const midnight = new Date(now.value);
    midnight.setHours(24, 0, 0, 0);
    let next = midnight.getTime();
    for (const task of tasks.value) {
      if (task.deletedAt || task.status !== 'pending' || !task.dueAt) continue;
      const due = Date.parse(task.dueAt);
      if (due > now.value) next = Math.min(next, due);
    }
    deadlineClock = setTimeout(tick, Math.max(1, next - Date.now() + 1));
  }
  watch([tasks, now], scheduleDeadline, { flush: 'sync' });

  function apply(snapshot: TaskSnapshot) {
    if (!Number.isSafeInteger(snapshot.revision) || snapshot.revision < revision.value) return;
    tasks.value = snapshot.tasks;
    categories.value = snapshot.categories;
    reminders.value = snapshot.reminders ?? [];
    revision.value = snapshot.revision;
  }
  function view(view: TaskView) {
    return tasksForView(tasks.value, view, today.value, now.value);
  }
  function tick() {
    today.value = localDate();
    now.value = Date.now();
  }
  async function refresh(minimum = -1) {
    if (Number.isSafeInteger(minimum)) wanted = Math.max(wanted, minimum);
    if (flight) return flight;
    const current = generation;
    refreshing.value = true;
    const pending = (async () => {
      try {
        do {
          const snapshot = await taskBridge.snapshot();
          if (generation !== current) return;
          apply(snapshot);
          ready.value = true;
          if (errorKind !== 'write') {
            error.value = null;
            errorKind = null;
          }
        } while (revision.value < wanted);
      } catch (value) {
        if (generation === current) {
          error.value = toAppError(value);
          errorKind = 'read';
        }
      } finally {
        if (generation === current) {
          refreshing.value = false;
          flight = undefined;
        }
      }
    })();
    flight = pending;
    return pending;
  }
  async function initialize() {
    dispose();
    const current = generation;
    error.value = null;
    errorKind = null;
    try {
      const unsubscribe = await taskBridge.onChange((value) => {
        if (generation === current && Number.isSafeInteger(value) && value >= 0)
          void refresh(value);
      });
      if (current !== generation) {
        unsubscribe();
        return;
      }
      stop = unsubscribe;
      clock = setInterval(tick, 30_000);
      tick();
      scheduleDeadline();
      await refresh();
    } catch (value) {
      if (generation === current) error.value = toAppError(value);
    }
  }
  function dispose() {
    generation++;
    stop?.();
    stop = undefined;
    if (clock) clearInterval(clock);
    clock = undefined;
    clearTimeout(deadlineClock);
    deadlineClock = undefined;
    flight = undefined;
    refreshing.value = false;
    wanted = -1;
  }
  async function mutate(operation: () => Promise<MutationResult>): Promise<MutationResult> {
    if (busy.value) throw { code: 'BUSY', message: '正在保存，请稍候。' };
    busy.value = true;
    error.value = null;
    errorKind = null;
    try {
      const result = await operation();
      apply(result.snapshot);
      return result;
    } catch (value) {
      const safe = toAppError(value);
      error.value = safe;
      errorKind = 'write';
      throw safe;
    } finally {
      busy.value = false;
    }
  }
  const create = (draft: TaskDraft) => mutate(() => taskBridge.create(draft));
  const update = (id: string, version: number, draft: TaskDraft) =>
    mutate(() => taskBridge.update(id, version, draft));
  const status = (task: Task, status: TaskStatus, confirm = false) =>
    mutate(() => taskBridge.status(task.id, task.version, status, confirm));
  const purge = (task: Task) => mutate(() => taskBridge.purge(task.id, task.version, true));
  const saveCategory = (draft: CategoryDraft) => mutate(() => taskBridge.saveCategory(draft));
  const deleteCategory = (category: Category) =>
    mutate(() => taskBridge.deleteCategory(category.id, category.version));
  return {
    tasks,
    reminders,
    reminderAction: (id: string, minutes: number | null) =>
      mutate(() => taskBridge.reminderAction(id, minutes)),
    categories,
    revision,
    ready,
    refreshing,
    busy,
    error,
    today,
    now,
    todayTasks,
    view,
    apply,
    refresh,
    initialize,
    dispose,
    tick,
    create,
    update,
    status,
    purge,
    saveCategory,
    deleteCategory,
  };
});
