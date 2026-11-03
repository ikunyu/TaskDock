import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { nativeMode } from './bridge';
import type {
  BackupEntry,
  BackupSettings,
  CalendarDay,
  DataStatus,
  SearchFilter,
  SearchPage,
} from '../data-types';
const call = <T>(command: string, args?: Record<string, unknown>): Promise<T> =>
  nativeMode
    ? invoke(command, args)
    : Promise.reject({ code: 'DESKTOP_REQUIRED', message: '数据功能需要在TaskDock桌面版使用。' });
export const dataBridge = {
  status: () => call<DataStatus>('data_status'),
  save: (settings: BackupSettings) => call<DataStatus>('save_backup_settings', { settings }),
  backup: () => call<BackupEntry>('backup_now'),
  restore: (id: string) =>
    call<{ safetyId: string; warnings: string[] }>('restore_backup', { id, confirmed: true }),
  export: (format: 'json' | 'csv', filter: SearchFilter | null = null) =>
    call<string | null>('export_data', { format, filter }),
  openDirectory: () => call<void>('open_backup_directory'),
  search: (filter: SearchFilter) => call<SearchPage>('search_tasks', { filter }),
  calendar: (month: string, categoryId: string | null) =>
    call<CalendarDay[]>('calendar_month', { month, categoryId }),
  onChange: (callback: () => void) =>
    nativeMode ? listen('taskdock://data-changed', callback) : Promise.resolve(() => {}),
};
