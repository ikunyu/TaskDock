import {
  defaultDesktopSettings,
  type DesktopSettings,
  type DesktopRuntimeStatus,
} from '../desktop-types';
import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { bridge } from '../services/bridge';
import { toAppError } from '../services/errors';
import { reportError } from '../services/logger';
import type { AppError, BootstrapSnapshot, SettingsSnapshot, ThemePreference } from '../types';

export const useAppStore = defineStore('app', () => {
  const desktop = ref(defaultDesktopSettings());
  const desktopStatus = ref<DesktopRuntimeStatus>({
    ready: false,
    effectiveShortcut: null,
    autostartEnabled: null,
    warnings: [],
  });
  const theme = ref<ThemePreference>('system');
  const revision = ref(-1);
  const schemaVersion = ref(0);
  const appVersion = ref('1.1.0');
  const ready = ref(false);
  const error = ref<AppError | null>(null);
  const busy = ref(false);
  const systemDark = ref(false);
  const resolvedTheme = computed(() =>
    theme.value === 'system' ? (systemDark.value ? 'dark' : 'light') : theme.value,
  );
  let unlisten: (() => void) | undefined;
  let generation = 0;

  function applySettings(settings: SettingsSnapshot) {
    if (
      !Number.isSafeInteger(settings.revision) ||
      settings.revision < revision.value ||
      !['system', 'light', 'dark'].includes(settings.theme)
    )
      return;
    theme.value = settings.theme;
    if (settings.desktop) desktop.value = { ...settings.desktop };
    revision.value = settings.revision;
  }
  function applyBootstrap(snapshot: BootstrapSnapshot) {
    applySettings(snapshot);
    schemaVersion.value = snapshot.schemaVersion;
    appVersion.value = snapshot.appVersion;
  }
  function fail(value: unknown) {
    error.value = toAppError(value);
    reportError('SERVICE_ERROR');
  }
  async function initialize() {
    const current = ++generation;
    unlisten?.();
    unlisten = undefined;
    ready.value = false;
    error.value = null;
    try {
      const stop = await bridge.onSettings(applySettings);
      if (current !== generation) {
        stop();
        return;
      }
      unlisten = stop;
      const snapshot = await bridge.bootstrap();
      if (current !== generation) return;
      applyBootstrap(snapshot);
      ready.value = true;
    } catch (value) {
      if (current === generation) {
        unlisten?.();
        unlisten = undefined;
        fail(value);
      }
    }
  }
  async function refresh() {
    try {
      applyBootstrap(await bridge.bootstrap());
    } catch (value) {
      fail(value);
    }
  }
  async function changeTheme(value: ThemePreference) {
    if (busy.value) return;
    busy.value = true;
    error.value = null;
    try {
      applySettings(await bridge.setTheme(value));
    } catch (value) {
      fail(value);
    } finally {
      busy.value = false;
    }
  }
  async function refreshDesktopStatus() {
    try {
      desktopStatus.value = await bridge.desktopStatus();
    } catch (value) {
      fail(value);
    }
  }
  async function saveDesktop(value: DesktopSettings, expectedRevision: number) {
    if (busy.value) throw { code: 'BUSY', message: '正在保存，请稍候。' };
    busy.value = true;
    error.value = null;
    try {
      const result = await bridge.saveDesktop(value, expectedRevision);
      applySettings(result);
      await refreshDesktopStatus();
      return result;
    } catch (value) {
      fail(value);
      throw toAppError(value);
    } finally {
      busy.value = false;
    }
  }
  function dispose() {
    ++generation;
    unlisten?.();
    unlisten = undefined;
  }
  return {
    desktop,
    desktopStatus,
    refreshDesktopStatus,
    saveDesktop,
    theme,
    revision,
    schemaVersion,
    appVersion,
    ready,
    error,
    busy,
    systemDark,
    resolvedTheme,
    initialize,
    refresh,
    changeTheme,
    applySettings,
    dispose,
    fail,
  };
});
