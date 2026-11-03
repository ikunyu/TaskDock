import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { nativeMode } from './bridge';
import {
  TASKS_EVENT,
  type CategoryDraft,
  type MutationResult,
  type TaskDraft,
  type TaskSnapshot,
  type TaskStatus,
} from '../task-types';

const desktopRequired = {
  code: 'DESKTOP_REQUIRED',
  message: '请在 TaskDock 桌面版中保存任务。浏览器仅用于布局预览。',
};
function write(command: string, args: Record<string, unknown>): Promise<MutationResult> {
  return nativeMode ? invoke(command, args) : Promise.reject(desktopRequired);
}
export const taskBridge = {
  snapshot: (): Promise<TaskSnapshot> =>
    nativeMode
      ? invoke('tasks_snapshot')
      : Promise.resolve({ revision: 0, tasks: [], categories: [] }),
  onChange: async (callback: (revision: number) => void): Promise<() => void> =>
    nativeMode
      ? listen<{ revision: number }>(TASKS_EVENT, (event) => callback(event.payload.revision))
      : () => {},
  create: (draft: TaskDraft) => write('create_task', { draft }),
  update: (id: string, expectedVersion: number, draft: TaskDraft) =>
    write('update_task', { id, expectedVersion, draft }),
  status: (id: string, expectedVersion: number, status: TaskStatus, confirmIncomplete: boolean) =>
    write('set_task_status', { id, expectedVersion, status, confirmIncomplete }),
  purge: (id: string, expectedVersion: number, confirmed: boolean) =>
    write('purge_task', { id, expectedVersion, confirmed }),
  reminderAction: (id: string, minutes: number | null) => write('reminder_action', { id, minutes }),
  saveCategory: (draft: CategoryDraft) => write('save_category', { draft }),
  deleteCategory: (id: string, expectedVersion: number) =>
    write('delete_category', { id, expectedVersion }),
};
