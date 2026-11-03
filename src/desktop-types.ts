export interface DesktopSettings {
  widgetOpacity: number;
  widgetFontSize: number;
  widgetCompactFontSize: number;
  widgetDateFontSize: number;
  widgetWeekFontSize: number;
  widgetWeekDateFontSize: number;
  widgetTopmost: boolean;
  widgetVisible: boolean;
  shortcut: string;
  autostart: boolean;
  closeBehavior: 'hide' | 'exit';
}
export interface DesktopRuntimeStatus {
  ready: boolean;
  effectiveShortcut: string | null;
  autostartEnabled: boolean | null;
  warnings: string[];
}
export const defaultDesktopSettings = (): DesktopSettings => ({
  widgetOpacity: 100,
  widgetFontSize: 13,
  widgetCompactFontSize: 14,
  widgetDateFontSize: 10,
  widgetWeekFontSize: 15,
  widgetWeekDateFontSize: 10,
  widgetTopmost: false,
  widgetVisible: false,
  shortcut: 'Ctrl+Alt+T',
  autostart: false,
  closeBehavior: 'hide',
});

export const widgetFontOptions = [
  { key: 'widgetFontSize', label: '展开任务', min: 10, max: 20, defaultValue: 13 },
  { key: 'widgetCompactFontSize', label: '收起任务', min: 10, max: 24, defaultValue: 14 },
  { key: 'widgetDateFontSize', label: '任务日期 / 时间', min: 8, max: 18, defaultValue: 10 },
  { key: 'widgetWeekFontSize', label: '周标题（本周 / 下周）', min: 10, max: 26, defaultValue: 15 },
  { key: 'widgetWeekDateFontSize', label: '周日期范围', min: 8, max: 18, defaultValue: 10 },
] as const;
