import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { nativeMode } from '../services/bridge';
import { toAppError } from '../services/errors';

export interface UpdateStatus {
  phase:
    | 'idle'
    | 'checking'
    | 'available'
    | 'downloading'
    | 'ready'
    | 'installing'
    | 'up_to_date'
    | 'error';
  currentBuild: string;
  availableBuild: string | null;
  notes: string | null;
  downloadedBytes: number;
  totalBytes: number | null;
  message: string | null;
}
export const useUpdatesStore = defineStore('updates', () => {
  const state = ref<UpdateStatus>({
    phase: 'idle',
    currentBuild: '',
    availableBuild: null,
    notes: null,
    downloadedBytes: 0,
    totalBytes: null,
    message: null,
  });
  const error = ref('');
  const dismissedBuild = ref<string | null>(null);
  const busy = computed(() =>
    ['checking', 'downloading', 'installing'].includes(state.value.phase),
  );
  const percent = computed(() =>
    state.value.totalBytes
      ? Math.min(100, Math.floor((state.value.downloadedBytes / state.value.totalBytes) * 100))
      : null,
  );
  let initialized = false,
    disposed = false,
    stop: (() => void) | undefined;
  async function initialize(main: boolean) {
    if (initialized || !nativeMode) return;
    initialized = true;
    try {
      const unlisten = await listen<UpdateStatus>('taskdock://update-state', ({ payload }) => {
        state.value = payload;
        if (payload.phase !== 'error') error.value = '';
      });
      if (disposed) {
        unlisten();
        return;
      }
      stop = unlisten;
      if (main) state.value = await invoke<UpdateStatus>('update_status');
    } catch (e) {
      error.value = toAppError(e).message;
    }
  }
  async function act(command: 'check_for_updates' | 'download_update' | 'install_update') {
    if (!nativeMode || busy.value) return;
    error.value = '';
    try {
      const result = await invoke<UpdateStatus | void>(
        command,
        command === 'install_update' ? { confirmed: true } : {},
      );
      if (result) state.value = result;
    } catch (e) {
      error.value = toAppError(e).message;
    }
  }
  function dispose() {
    disposed = true;
    stop?.();
  }
  return { state, error, busy, percent, dismissedBuild, initialize, act, dispose };
});
