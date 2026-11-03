import type { Task, TaskSnapshot } from '../src/task-types';
export function task(overrides: Partial<Task> = {}): Task {
  return {
    id: '4d7ade3b-c421-4dce-a390-a424d45248cb',
    title: '准备周计划',
    description: '',
    priority: 'none',
    categoryId: null,
    plannedDate: '2026-10-28',
    dueDate: null,
    dueAt: null,
    sortOrder: 0,
    status: 'pending',
    version: 1,
    createdAt: '2026-10-28T00:00:00.000Z',
    updatedAt: '2026-10-28T00:00:00.000Z',
    completedAt: null,
    deletedAt: null,
    subtasks: [],
    progress: { total: 0, completed: 0, percent: 0 },
    ...overrides,
  };
}
export function snapshot(revision = 1, tasks: Task[] = [task()]): TaskSnapshot {
  return { revision, tasks, categories: [] };
}
