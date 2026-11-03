import { afterEach, describe, expect, it, vi } from 'vitest';
import { bridge, nativeMode, windowLabel } from '../src/services/bridge';

afterEach(() => vi.unstubAllGlobals());

describe('browser preview boundary', () => {
  it('does not claim a database connection outside Tauri', async () => {
    expect(nativeMode).toBe(false);
    expect((await bridge.bootstrap()).schemaVersion).toBe(0);
  });

  it('notifies preview listeners and releases subscriptions', async () => {
    const callback = vi.fn();
    const before = await bridge.bootstrap();
    const stop = await bridge.onSettings(callback);
    const result = await bridge.setTheme('dark');
    expect(result.revision).toBe(before.revision + 1);
    expect(callback).toHaveBeenCalledWith(expect.objectContaining({ theme: 'dark' }));
    stop();
    await bridge.setTheme('system');
    expect(callback).toHaveBeenCalledOnce();
  });

  it.each([
    ['?window=widget', 'widget'],
    ['?window=quick-entry', 'quick-entry'],
    ['?window=untrusted', 'main'],
    ['', 'main'],
  ])('accepts only predefined preview labels: %s', (search, expected) => {
    vi.stubGlobal('location', { search });
    expect(windowLabel()).toBe(expected);
  });
});
