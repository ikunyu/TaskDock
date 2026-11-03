import type { Task, TaskDraft, TaskView, Priority } from '../task-types';

export function localDate(date = new Date()): string {
  return `${date.getFullYear().toString().padStart(4, '0')}-${(date.getMonth() + 1).toString().padStart(2, '0')}-${date.getDate().toString().padStart(2, '0')}`;
}
export function validDate(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const [year, month, day] = value.split('-').map(Number);
  if (!year || !month || !day) return false;
  const date = new Date(0);
  date.setUTCFullYear(year, month - 1, day);
  date.setUTCHours(12, 0, 0, 0);
  return (
    year >= 1 &&
    year <= 9999 &&
    date.getUTCFullYear() === year &&
    date.getUTCMonth() === month - 1 &&
    date.getUTCDate() === day
  );
}
export function blankDraft(plannedDate: string | null = null): TaskDraft {
  return {
    title: '',
    description: '',
    priority: 'none',
    categoryId: null,
    plannedDate,
    dueDate: null,
    dueAt: null,
    sortOrder: 0,
    subtasks: [],
  };
}
export function taskDraft(task: Task): TaskDraft {
  return {
    title: task.title,
    description: task.description,
    priority: task.priority,
    categoryId: task.categoryId,
    plannedDate: task.plannedDate,
    dueDate: task.dueDate,
    dueAt: task.dueAt,
    sortOrder: task.sortOrder,
    schedule: task.schedule ? JSON.parse(JSON.stringify(task.schedule.config)) : null,
    repeatScope: 'once',
    subtasks: task.subtasks.map((item) => ({
      id: item.id,
      title: item.title,
      completed: !!item.completedAt,
    })),
  };
}
const priorityOrder: Record<Priority, number> = { high: 0, medium: 1, low: 2, none: 3 };
function taskTimeElapsed(task: Task, today: string, now: number): boolean {
  if (task.dueAt) return Date.parse(task.dueAt) <= now;
  if (task.dueDate) return task.dueDate < today;
  return task.plannedDate !== null && task.plannedDate < today;
}
export function isTimedOut(task: Task, today: string, now = Date.now()): boolean {
  return task.status === 'pending' && !task.deletedAt && taskTimeElapsed(task, today, now);
}
export function compareTaskOrder(a: Task, b: Task): number {
  return (
    (a.plannedDate ?? '').localeCompare(b.plannedDate ?? '') ||
    priorityOrder[a.priority] - priorityOrder[b.priority] ||
    a.sortOrder - b.sortOrder ||
    a.createdAt.localeCompare(b.createdAt) ||
    a.id.localeCompare(b.id)
  );
}
export function inView(task: Task, view: TaskView, today: string, now = Date.now()): boolean {
  if (task.deletedAt) return false;
  if (view === 'inbox') return task.plannedDate === null;
  if (view === 'done') return task.status === 'completed' && task.plannedDate !== null;
  if (task.status !== 'pending') return false;
  const timedOut = isTimedOut(task, today, now);
  if (view === 'today') return timedOut || (task.plannedDate !== null && task.plannedDate <= today);
  return !timedOut && task.plannedDate !== null && task.plannedDate > today;
}
export function tasksForView(
  tasks: Task[],
  view: TaskView,
  today: string,
  now = Date.now(),
): Task[] {
  return tasks
    .filter((task) => inView(task, view, today, now))
    .sort((a, b) => {
      if (view === 'inbox' && a.status !== b.status) return a.status === 'pending' ? -1 : 1;
      if (view === 'done')
        return (b.completedAt ?? '').localeCompare(a.completedAt ?? '') || a.id.localeCompare(b.id);
      if (view === 'today') {
        const timeoutOrder = Number(isTimedOut(b, today, now)) - Number(isTimedOut(a, today, now));
        if (timeoutOrder) return timeoutOrder;
      }
      return (
        (a.plannedDate ?? '').localeCompare(b.plannedDate ?? '') ||
        priorityOrder[a.priority] - priorityOrder[b.priority] ||
        a.sortOrder - b.sortOrder ||
        a.createdAt.localeCompare(b.createdAt) ||
        a.id.localeCompare(b.id)
      );
    });
}
export function overdue(task: Task, today: string, now = Date.now()): string | null {
  if (task.status === 'completed' || task.deletedAt) return null;
  if (isTimedOut(task, today, now)) return '超时任务';
  if (task.plannedDate && task.plannedDate < today) return '计划逾期';
  return null;
}
export function widgetTasks(
  tasks: Task[],
  today: string,
  now = Date.now(),
  view: 'today' | 'plan' = 'today',
): Task[] {
  return tasks
    .filter((task) => {
      if (task.deletedAt) return false;
      if (task.status === 'pending') return inView(task, view, today, now);
      if (!task.plannedDate && !task.dueDate && !task.dueAt) return false;
      const completedAt = task.completedAt ? Date.parse(task.completedAt) : NaN;
      const completedDay = Number.isFinite(completedAt) ? localDate(new Date(completedAt)) : null;
      // A future task completed on time stays in its original future week. A
      // deadline-overdue instance completed today stays in today's list instead.
      const completedLate =
        completedDay !== null && taskTimeElapsed(task, completedDay, completedAt);
      if (view === 'plan') return !!task.plannedDate && task.plannedDate > today && !completedLate;
      return (
        task.plannedDate === today ||
        (completedDay === today &&
          (task.plannedDate === null || task.plannedDate <= today || completedLate))
      );
    })
    .sort(
      (a, b) =>
        Number(taskTimeElapsed(b, today, now)) - Number(taskTimeElapsed(a, today, now)) ||
        compareTaskOrder(a, b),
    );
}
export function validateCreationDates(draft: TaskDraft, today = localDate()): string | null {
  const dates = [draft.plannedDate, draft.dueDate, draft.schedule?.recurrence?.until];
  for (const instant of [draft.dueAt, draft.schedule?.at]) {
    if (instant) dates.push(localDate(new Date(instant)));
  }
  for (const date of dates) {
    if (!date) continue;
    if (!validDate(date)) return '日期无效，请重新选择。';
    if (date < today) return '新建任务的日期只能选择今天或之后的日期。';
  }
  return null;
}
export function validateDraft(draft: TaskDraft): string | null {
  if (!draft.title.trim() || [...draft.title.trim()].length > 500)
    return '标题不能为空，且最多 500 字。';
  if ([...draft.description].length > 20000) return '描述最多 20000 字。';
  if (draft.plannedDate && !validDate(draft.plannedDate)) return '计划日期无效。';
  if (draft.dueDate && !validDate(draft.dueDate)) return '截止日期无效。';
  if (draft.dueDate && draft.dueAt) return '请选择一种截止方式。';
  if (
    draft.dueAt &&
    (!Number.isFinite(Date.parse(draft.dueAt)) || !/[zZ]|[+-]\d\d:\d\d$/.test(draft.dueAt))
  )
    return '截止时间无效。';
  const schedule = draft.schedule;
  if (schedule?.reminderRepeat) {
    const repeat = schedule.reminderRepeat;
    const maximum = { minutes: 5256000, hours: 87600, days: 3650 }[repeat.unit];
    if (
      !maximum ||
      !Number.isInteger(repeat.interval) ||
      repeat.interval < 1 ||
      repeat.interval > maximum
    )
      return '定期提醒间隔必须为正整数，且不能超过3650天。';
    if (!schedule.at || !Number.isFinite(Date.parse(schedule.at)))
      return '请设置有效的首次提醒时间。';
    const day = draft.dueDate ?? draft.plannedDate;
    const deadline = draft.dueAt ?? (day ? `${day}T23:59:00` : null);
    if (deadline && Date.parse(schedule.at) > Date.parse(deadline))
      return '首次提醒不能晚于任务截止时间。';
  }
  if (
    draft.subtasks.length > 200 ||
    draft.subtasks.some((item) => !item.title.trim() || [...item.title.trim()].length > 500)
  )
    return '子任务标题不能为空且最多 500 字，子任务最多 200 项。';
  return null;
}
export function toLocalDateTime(value: string | null): string {
  if (!value) return '';
  const date = new Date(value);
  return `${localDate(date)}T${date.getHours().toString().padStart(2, '0')}:${date.getMinutes().toString().padStart(2, '0')}`;
}
export function toUtcDateTime(value: string): string | null {
  if (!value) return null;
  const date = new Date(value);
  if (!Number.isFinite(date.getTime()) || toLocalDateTime(date.toISOString()) !== value)
    throw new Error('截止时间无效，或该本地时间不存在。');
  return date.toISOString();
}
export function formatInstant(value: string | null): string {
  return value ? new Date(value).toLocaleString('zh-CN', { hour12: false }) : '';
}
export const priorityLabels: Record<Priority, string> = {
  none: '无优先级',
  low: '低优先级',
  medium: '中优先级',
  high: '高优先级',
};
