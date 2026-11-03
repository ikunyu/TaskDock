// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import WidgetWindow from '../src/windows/WidgetWindow.vue';
import { useAppStore } from '../src/stores/app';
const api = vi.hoisted(() => ({
  widgetAction: vi.fn(),
  showWindow: vi.fn(),
  reportError: vi.fn(),
  widgetLayout: vi.fn(),
  widgetLockBounds: vi.fn(),
}));
vi.mock('../src/services/bridge', () => ({ bridge: api, nativeMode: true }));
let wrapper: VueWrapper;
beforeEach(() => {
  vi.resetAllMocks();
  api.widgetAction.mockResolvedValue(undefined);
  api.widgetLayout.mockResolvedValue(undefined);
  api.widgetLockBounds.mockResolvedValue(undefined);
  setActivePinia(createPinia());
  wrapper = mount(WidgetWindow);
});
afterEach(() => wrapper.unmount());
describe('widget native interaction boundaries', () => {
  it('routes the locked state button to the bounded unlock command', async () => {
    const app = useAppStore();
    app.desktop.widgetLocked = true;
    await wrapper.vm.$nextTick();
    await wrapper.get('button[aria-label="解锁悬浮窗"]').trigger('click');
    expect(api.widgetAction).toHaveBeenCalledWith('unlock');
  });
  it('locks through its explicit button without starting a drag', async () => {
    const lock = wrapper.get('button[aria-label="锁定悬浮窗"]');
    lock.element.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }));
    expect(api.widgetAction).not.toHaveBeenCalled();
    await lock.trigger('click');
    expect(api.widgetAction).toHaveBeenCalledWith('lock');
  });
  it('starts a drag only from its header surface', async () => {
    wrapper
      .get('.mini-brand')
      .element.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }));
    expect(api.widgetAction).toHaveBeenCalledWith('drag');
  });
  it('does not drag while using header buttons or non-left buttons', async () => {
    await wrapper.get('.widget-expand').trigger('click');
    wrapper
      .get('button[aria-label="刷新今日任务"]')
      .element.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }));
    wrapper
      .get('.mini-brand')
      .element.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 2 }));
    expect(api.widgetAction).not.toHaveBeenCalled();
  });
  it('bounds native hide and resize actions to the explicit handles', async () => {
    await wrapper.get('.widget-expand').trigger('click');
    wrapper
      .get('button[aria-label="调整悬浮窗大小"]')
      .element.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }));
    expect(api.widgetAction).toHaveBeenLastCalledWith('resize');
    await wrapper.get('button[aria-label="隐藏悬浮窗"]').trigger('click');
    expect(api.widgetAction).toHaveBeenLastCalledWith('hide');
  });
});
