// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import WidgetWindow from '../src/windows/WidgetWindow.vue';
const api = vi.hoisted(() => ({
  widgetAction: vi.fn(),
  showWindow: vi.fn(),
  reportError: vi.fn(),
  widgetLayout: vi.fn(),
}));
vi.mock('../src/services/bridge', () => ({ bridge: api, nativeMode: true }));
let wrapper: VueWrapper;
beforeEach(() => {
  vi.resetAllMocks();
  api.widgetAction.mockResolvedValue(undefined);
  api.widgetLayout.mockResolvedValue(undefined);
  setActivePinia(createPinia());
  wrapper = mount(WidgetWindow);
});
afterEach(() => wrapper.unmount());
describe('widget native interaction boundaries', () => {
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
