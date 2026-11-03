import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import type { BootstrapSnapshot, SettingsSnapshot } from '../src/types';

const mock = vi.hoisted(() => ({
  bootstrap: vi.fn(),
  setTheme: vi.fn(),
  onSettings: vi.fn(),
  reportError: vi.fn().mockResolvedValue(undefined),
}));
vi.mock('../src/services/bridge', () => ({ bridge: mock }));
import { useAppStore } from '../src/stores/app';

const snapshot = (
  theme: 'system' | 'light' | 'dark' = 'system',
  revision = 0,
): BootstrapSnapshot => ({ theme, revision, appVersion: '0.1.0', schemaVersion: 1 });

describe('settings synchronization', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mock.reportError.mockResolvedValue(undefined);
    setActivePinia(createPinia());
  });

  it('subscribes before reading and ignores an older bootstrap snapshot', async () => {
    let callback!: (settings: SettingsSnapshot) => void;
    mock.onSettings.mockImplementation(async (handler) => {
      callback = handler;
      return vi.fn();
    });
    mock.bootstrap.mockImplementation(async () => {
      callback({ theme: 'dark', revision: 2 });
      return snapshot('light', 1);
    });
    const store = useAppStore();
    await store.initialize();
    expect(mock.onSettings.mock.invocationCallOrder[0]).toBeLessThan(
      mock.bootstrap.mock.invocationCallOrder[0]!,
    );
    expect(store.theme).toBe('dark');
    expect(store.revision).toBe(2);
    expect(store.ready).toBe(true);
  });

  it('does not let an out-of-order write response roll back an event', async () => {
    const store = useAppStore();
    mock.setTheme.mockImplementation(async () => {
      store.applySettings({ theme: 'light', revision: 5 });
      return { theme: 'dark', revision: 4 };
    });
    await store.changeTheme('dark');
    expect(store.theme).toBe('light');
    expect(store.revision).toBe(5);
  });

  it('disposes listeners and discards an in-flight bootstrap after unmount', async () => {
    const stop = vi.fn();
    let finish!: (value: BootstrapSnapshot) => void;
    mock.onSettings.mockResolvedValue(stop);
    mock.bootstrap.mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    const store = useAppStore();
    const initializing = store.initialize();
    await Promise.resolve();
    store.dispose();
    finish(snapshot('dark', 1));
    await initializing;
    expect(stop).toHaveBeenCalledOnce();
    expect(store.ready).toBe(false);
    expect(store.revision).toBe(-1);
  });

  it('cleans up a late listener subscription after unmount', async () => {
    const stop = vi.fn();
    let finish!: (value: () => void) => void;
    mock.onSettings.mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    const store = useAppStore();
    const initializing = store.initialize();
    store.dispose();
    finish(stop);
    await initializing;
    expect(stop).toHaveBeenCalledOnce();
    expect(mock.bootstrap).not.toHaveBeenCalled();
  });

  it('reports a read failure and can reconnect without leaking listeners', async () => {
    const stop = vi.fn();
    mock.onSettings.mockResolvedValue(stop);
    mock.bootstrap
      .mockRejectedValueOnce({ code: 'SERVICE_UNAVAILABLE', message: '本地服务暂不可用' })
      .mockResolvedValueOnce(snapshot());
    const store = useAppStore();
    await store.initialize();
    expect(store.error?.code).toBe('SERVICE_UNAVAILABLE');
    expect(stop).toHaveBeenCalledOnce();
    await store.initialize();
    expect(store.error).toBeNull();
    expect(store.ready).toBe(true);
    store.dispose();
    expect(stop).toHaveBeenCalledTimes(2);
  });

  it('preserves the persisted theme when a theme write fails', async () => {
    const store = useAppStore();
    store.applySettings({ theme: 'light', revision: 1 });
    mock.setTheme.mockRejectedValue(new Error('private local path'));
    await store.changeTheme('dark');
    expect(store.theme).toBe('light');
    expect(store.busy).toBe(false);
    expect(store.error?.message).not.toContain('private local path');
  });

  it('resolves system theme and rejects malformed settings events', () => {
    const store = useAppStore();
    store.systemDark = true;
    expect(store.resolvedTheme).toBe('dark');
    store.applySettings({ theme: 'light', revision: NaN });
    expect(store.theme).toBe('system');
    store.applySettings({ theme: 'light', revision: 3 });
    store.applySettings({ theme: 'dark', revision: 2 });
    expect(store.resolvedTheme).toBe('light');
  });
});
