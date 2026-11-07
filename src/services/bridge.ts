import {
  defaultDesktopSettings,
  type DesktopSettings,
  type DesktopRuntimeStatus,
} from '../desktop-types';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  SETTINGS_EVENT,
  WINDOW_LABELS,
  type BootstrapSnapshot,
  type SettingsSnapshot,
  type ThemePreference,
  type WindowLabel,
} from '../types';

export const nativeMode = isTauri();
let preview: BootstrapSnapshot = {
  appVersion: '1.2.1',
  schemaVersion: 0,
  desktop: defaultDesktopSettings(),
  theme: 'system',
  revision: 0,
};
const previewListeners = new Set<(settings: SettingsSnapshot) => void>();

export function windowLabel(): WindowLabel {
  const label = nativeMode
    ? getCurrentWindow().label
    : new URLSearchParams(location.search).get('window');
  return WINDOW_LABELS.includes(label ?? '') ? (label as WindowLabel) : 'main';
}

export const bridge = {
  desktopStatus: (): Promise<DesktopRuntimeStatus> =>
    nativeMode
      ? invoke('desktop_status')
      : Promise.resolve({
          ready: false,
          effectiveShortcut: null,
          autostartEnabled: null,
          warnings: ['浏览器仅预览设置布局。'],
        }),
  saveDesktop: (settings: DesktopSettings, expectedRevision: number): Promise<SettingsSnapshot> =>
    nativeMode
      ? invoke('save_desktop_settings', { settings, expectedRevision })
      : Promise.reject({ code: 'DESKTOP_REQUIRED', message: '此操作需要 Windows 桌面版。' }),
  resetPositions: (): Promise<void> =>
    nativeMode
      ? invoke('reset_window_positions')
      : Promise.reject({ code: 'DESKTOP_REQUIRED', message: '此操作需要 Windows 桌面版。' }),
  exitApp: (): Promise<void> =>
    nativeMode
      ? invoke('exit_app')
      : Promise.reject({ code: 'DESKTOP_REQUIRED', message: '此操作需要 Windows 桌面版。' }),
  widgetAction: (action: 'drag' | 'resize' | 'hide' | 'lock' | 'unlock'): Promise<void> =>
    nativeMode
      ? action === 'hide'
        ? invoke('hide_current_window')
        : action === 'lock'
          ? invoke('lock_widget')
          : action === 'unlock'
            ? invoke('unlock_widget')
            : invoke('widget_window_action', { action })
      : Promise.reject({ code: 'DESKTOP_REQUIRED', message: '此操作需要 Windows 桌面版。' }),
  widgetLayout: (expanded: boolean, width: number, height: number): Promise<void> =>
    nativeMode ? invoke('set_widget_layout', { expanded, width, height }) : Promise.resolve(),
  widgetLockBounds: (bounds: {
    x: number;
    y: number;
    width: number;
    height: number;
  }): Promise<void> =>
    nativeMode ? invoke('set_widget_lock_bounds', { bounds }) : Promise.resolve(),
  onOpenSettings: async (callback: () => void): Promise<() => void> =>
    nativeMode ? listen('taskdock://open-settings', callback) : () => {},
  onQuickFocus: async (callback: () => void): Promise<() => void> =>
    nativeMode ? listen('taskdock://quick-focus', callback) : () => {},
  bootstrap: (): Promise<BootstrapSnapshot> =>
    nativeMode ? invoke('bootstrap') : Promise.resolve({ ...preview }),
  async setTheme(theme: ThemePreference): Promise<SettingsSnapshot> {
    if (nativeMode) return invoke('set_theme', { theme });
    preview = { ...preview, theme, revision: preview.revision + 1 };
    previewListeners.forEach((listener) => listener(preview));
    return { ...preview };
  },
  async onSettings(callback: (settings: SettingsSnapshot) => void): Promise<() => void> {
    if (nativeMode)
      return listen<SettingsSnapshot>(SETTINGS_EVENT, (event) => callback(event.payload));
    previewListeners.add(callback);
    return () => {
      previewListeners.delete(callback);
    };
  },
  async showWindow(label: 'widget' | 'quick-entry'): Promise<void> {
    if (nativeMode) await invoke('show_window', { label });
    else window.open(`/?window=${label}`, '_blank', 'noopener,noreferrer');
  },
  async hideCurrent(): Promise<void> {
    if (nativeMode) await invoke('hide_current_window');
    else window.close();
  },
  async reportError(code: string): Promise<void> {
    if (nativeMode) await invoke('report_frontend_error', { code });
    else console.warn(`[TaskDock] ${code}`);
  },
};
