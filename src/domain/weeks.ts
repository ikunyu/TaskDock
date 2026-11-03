import type { Task } from '../task-types';
import { compareTaskOrder, localDate, validDate } from './tasks';

export type WeekStatus = 'all' | 'pending' | 'completed';
export interface WeekDay {
  date: string;
  label: string;
  tasks: Task[];
}
export interface TaskWeek {
  start: string;
  end: string;
  tasks: Task[];
}
export function groupTaskWeeks(tasks: Task[]): TaskWeek[] {
  const weeks = new Map<string, TaskWeek>();
  for (const task of tasks) {
    if (task.deletedAt) continue;
    const date = taskCalendarDate(task);
    if (!date) continue;
    const start = mondayOf(date);
    let week = weeks.get(start);
    if (!week) {
      week = { start, end: shiftDate(start, 6), tasks: [] };
      weeks.set(start, week);
    }
    week.tasks.push(task);
  }
  return [...weeks.values()].sort((a, b) => a.start.localeCompare(b.start));
}
export function shiftDate(value: string, days: number): string {
  if (!validDate(value)) throw new Error('日期无效。');
  const [year, month, day] = value.split('-').map(Number) as [number, number, number];
  const date = new Date(0);
  date.setFullYear(year, month - 1, day);
  date.setHours(12, 0, 0, 0);
  date.setDate(date.getDate() + days);
  return localDate(date);
}
export function mondayOf(value: string): string {
  const normalized = shiftDate(value, 0);
  const [year, month, day] = normalized.split('-').map(Number) as [number, number, number];
  const date = new Date(0);
  date.setFullYear(year, month - 1, day);
  date.setHours(12, 0, 0, 0);
  return shiftDate(value, -((date.getDay() + 6) % 7));
}
export function taskCalendarDate(task: Task): string | null {
  const date =
    task.plannedDate ?? task.dueDate ?? (task.dueAt ? localDate(new Date(task.dueAt)) : null);
  return date && validDate(date) ? date : null;
}
export function groupWeek(
  tasks: Task[],
  anchor: string,
  _today: string,
  _now: number,
  categoryId: string | null = null,
  status: WeekStatus = 'all',
): WeekDay[] {
  const start = mondayOf(anchor);
  const labels = ['周一', '周二', '周三', '周四', '周五', '周六', '周日'];
  const days = labels.map((label, index) => ({
    date: shiftDate(start, index),
    label,
    tasks: [] as Task[],
  }));
  const byDate = new Map(days.map((day) => [day.date, day]));
  for (const task of tasks) {
    if (task.deletedAt || (status !== 'all' && task.status !== status)) continue;
    if (
      categoryId &&
      (categoryId === 'uncategorized' ? task.categoryId !== null : task.categoryId !== categoryId)
    )
      continue;
    const date = taskCalendarDate(task);
    if (date) byDate.get(date)?.tasks.push(task);
  }
  for (const day of days) {
    day.tasks.sort(compareTaskOrder);
  }
  return days;
}
