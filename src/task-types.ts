import type { ScheduleConfig, ScheduleInfo, Reminder } from './schedule-types';
export type TaskStatus = 'pending' | 'completed';
export type Priority = 'none' | 'low' | 'medium' | 'high';
export type TaskView = 'today' | 'plan' | 'inbox' | 'done';
export interface SubtaskDraft {
  id: string | null;
  title: string;
  completed: boolean;
}
export interface TaskDraft {
  title: string;
  description: string;
  priority: Priority;
  categoryId: string | null;
  plannedDate: string | null;
  dueDate: string | null;
  dueAt: string | null;
  sortOrder: number;
  subtasks: SubtaskDraft[];
  schedule?: ScheduleConfig | null;
  repeatScope?: 'once' | 'following';
}
export interface Subtask {
  id: string;
  title: string;
  completedAt: string | null;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}
export interface Task extends Omit<TaskDraft, 'subtasks' | 'schedule' | 'repeatScope'> {
  id: string;
  schedule?: ScheduleInfo | null;
  status: TaskStatus;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
  deletedAt: string | null;
  version: number;
  subtasks: Subtask[];
  progress: { total: number; completed: number; percent: number };
}
export interface Category {
  id: string;
  name: string;
  color: string;
  sortOrder: number;
  version: number;
  createdAt: string;
  updatedAt: string;
}
export interface CategoryDraft {
  id: string | null;
  expectedVersion: number | null;
  name: string;
  color: string;
  sortOrder: number;
}
export interface TaskSnapshot {
  revision: number;
  tasks: Task[];
  categories: Category[];
  reminders?: Reminder[];
}
export interface MutationResult {
  snapshot: TaskSnapshot;
  taskId: string | null;
  categoryId: string | null;
}
export const TASKS_EVENT = 'taskdock://tasks-changed';
