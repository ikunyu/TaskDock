export interface Recurrence {
  frequency: 'daily' | 'weekly' | 'monthly' | 'interval';
  interval: number;
  weekdays: number[];
  monthDay: number | null;
  until: string | null;
}
export interface ReminderRepeat {
  interval: number;
  unit: 'minutes' | 'hours' | 'days';
}
export interface ScheduleConfig {
  at: string | null;
  reminderRepeat?: ReminderRepeat | null;
  beforeMinutes: number[];
  atDeadline: boolean;
  recurrence: Recurrence | null;
}
export interface ScheduleInfo {
  config: ScheduleConfig;
  seriesId: string | null;
  occurrenceDate: string | null;
  nextAt: string | null;
  hasNotified: boolean;
}
export interface Reminder {
  id: string;
  taskId: string;
  title: string;
  scheduledAt: string;
  state: 'pending' | 'notified' | 'cancelled' | 'dismissed';
  channel: 'system' | 'in_app' | 'unknown' | null;
}
export function emptySchedule(): ScheduleConfig {
  return { at: null, reminderRepeat: null, beforeMinutes: [], atDeadline: false, recurrence: null };
}
