import type { Task } from './task-types';
export interface BackupSettings {
  autoBackup: boolean;
  intervalHours: number;
  retention: number;
  version: number;
}
export interface BackupEntry {
  id: string;
  filename: string;
  kind: 'auto' | 'manual' | 'safety';
  createdAt: string;
  sha256: string;
  bytes: number;
  schemaVersion: number;
}
export interface DataStatus {
  dataDirectory: string;
  settings: BackupSettings;
  backups: BackupEntry[];
  lastSuccess: string | null;
  lastAttempt: string | null;
  lastError: string | null;
}
export interface SearchFilter {
  keyword: string;
  status: string | null;
  priority: string | null;
  categoryId: string | null;
  plannedFrom: string | null;
  plannedTo: string | null;
  dueFrom: string | null;
  dueTo: string | null;
  completed: boolean | null;
  overdue: boolean | null;
  calendarDate: string | null;
  offset: number;
}
export const emptySearch = (): SearchFilter => ({
  keyword: '',
  status: null,
  priority: null,
  categoryId: null,
  plannedFrom: null,
  plannedTo: null,
  dueFrom: null,
  dueTo: null,
  completed: null,
  overdue: null,
  calendarDate: null,
  offset: 0,
});
export interface SearchPage {
  tasks: Task[];
  hasMore: boolean;
  offset: number;
}
export interface CalendarDay {
  date: string;
  total: number;
  completed: number;
  high: number;
  overdue: number;
}
