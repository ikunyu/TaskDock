import type { DesktopSettings } from './desktop-types';
export type ThemePreference = 'system' | 'light' | 'dark';
export type WindowLabel = 'main' | 'widget' | 'quick-entry';
export interface SettingsSnapshot {
  theme: ThemePreference;
  desktop?: DesktopSettings;
  revision: number;
}
export interface BootstrapSnapshot extends SettingsSnapshot {
  appVersion: string;
  schemaVersion: number;
}
export interface AppError {
  code: string;
  message: string;
}
export const SETTINGS_EVENT = 'taskdock://settings-changed';
export const WINDOW_LABELS: readonly string[] = ['main', 'widget', 'quick-entry'];
