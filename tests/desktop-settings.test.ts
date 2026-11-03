// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import SettingsPanel from '../src/components/SettingsPanel.vue';
import { useAppStore } from '../src/stores/app';
import { defaultDesktopSettings } from '../src/desktop-types';
const api = vi.hoisted(() => ({
  desktopStatus: vi.fn(),
  saveDesktop: vi.fn(),
  resetPositions: vi.fn(),
  exitApp: vi.fn(),
  reportError: vi.fn(),
}));
vi.mock('../src/services/bridge', () => ({ nativeMode: true, bridge: api }));
let store: ReturnType<typeof useAppStore>, wrapper: VueWrapper;
beforeEach(() => {
  vi.resetAllMocks();
  api.reportError.mockResolvedValue(undefined);
  setActivePinia(createPinia());
  store = useAppStore();
  store.applySettings({ theme: 'light', revision: 1, desktop: defaultDesktopSettings() });
  api.desktopStatus.mockResolvedValue({
    ready: true,
    effectiveShortcut: 'Ctrl+Alt+T',
    autostartEnabled: false,
    warnings: [],
  });
  wrapper = mount(SettingsPanel, {
    global: { stubs: { Teleport: true } },
    attachTo: document.body,
  });
});
afterEach(() => {
  wrapper.unmount();
  store.dispose();
  document.body.innerHTML = '';
});
function button(text: string) {
  const found = wrapper.findAll('button').find((b) => b.text() === text);
  if (!found) throw Error(text);
  return found;
}
describe('desktop settings interactions', () => {
  it('retains conflicting shortcut input and the original effective settings on failure', async () => {
    await flushPromises();
    api.saveDesktop.mockRejectedValue({
      code: 'SHORTCUT_CONFLICT',
      message: '快捷键被占用，原快捷键保留',
    });
    await wrapper.get('input[placeholder="Ctrl+Alt+T"]').setValue('Ctrl+Alt+Y');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(wrapper.text()).toContain('快捷键被占用');
    expect(wrapper.get('input[placeholder="Ctrl+Alt+T"]').element).toHaveProperty(
      'value',
      'Ctrl+Alt+Y',
    );
    expect(store.desktop.shortcut).toBe('Ctrl+Alt+T');
    expect(wrapper.text()).not.toContain('桌面设置已保存');
  });
  it('only publishes committed settings and keeps autostart off until explicit save', async () => {
    await flushPromises();
    expect(api.saveDesktop).not.toHaveBeenCalled();
    expect(store.desktop.autostart).toBe(false);
    await wrapper.get('input[type=range]').setValue('65');
    api.saveDesktop.mockResolvedValue({
      theme: 'light',
      revision: 2,
      desktop: { ...defaultDesktopSettings(), widgetOpacity: 65 },
    });
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(api.saveDesktop).toHaveBeenCalledWith(
      expect.objectContaining({ widgetOpacity: 65, autostart: false }),
      1,
    );
    expect(store.desktop.widgetOpacity).toBe(65);
    expect(wrapper.text()).toContain('桌面设置已保存并应用');
    await wrapper.get('input[placeholder="Ctrl+Alt+T"]').setValue('Ctrl+Alt+U');
    expect(wrapper.text()).not.toContain('桌面设置已保存并应用');
  });
  it('preserves a settings draft when another window changes the revision', async () => {
    await flushPromises();
    await wrapper.get('input[placeholder="Ctrl+Alt+T"]').setValue('Ctrl+Alt+U');
    store.applySettings({
      theme: 'dark',
      revision: 2,
      desktop: { ...defaultDesktopSettings(), widgetVisible: true },
    });
    await flushPromises();
    expect(wrapper.get('input[placeholder="Ctrl+Alt+T"]').element).toHaveProperty(
      'value',
      'Ctrl+Alt+U',
    );
    expect(button('保存桌面设置').attributes('disabled')).toBeDefined();
    await button('重新载入设置').trigger('click');
    expect(wrapper.get('input[placeholder="Ctrl+Alt+T"]').element).toHaveProperty(
      'value',
      'Ctrl+Alt+T',
    );
  });
  it('rejects out-of-range opacity before invoking native code', async () => {
    await flushPromises();
    store.desktop.widgetOpacity = 10;
    wrapper.unmount();
    wrapper = mount(SettingsPanel, { global: { stubs: { Teleport: true } } });
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(api.saveDesktop).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('60–100');
  });
  it('routes reset and explicit exit through bounded commands', async () => {
    await flushPromises();
    api.resetPositions.mockResolvedValue(undefined);
    api.exitApp.mockResolvedValue(undefined);
    await button('恢复窗口默认位置').trigger('click');
    await flushPromises();
    expect(api.resetPositions).toHaveBeenCalledOnce();
    await button('退出 TaskDock').trigger('click');
    expect(api.exitApp).toHaveBeenCalledOnce();
  });
});
