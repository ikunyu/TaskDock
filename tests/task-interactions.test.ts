// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, flushPromises, type VueWrapper } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { defineComponent, h, type Component } from 'vue';
import DetailDrawer from '../src/components/DetailDrawer.vue';
import TaskList from '../src/components/TaskList.vue';
import TaskRow from '../src/components/TaskRow.vue';
import QuickEntryWindow from '../src/windows/QuickEntryWindow.vue';
import MainWindow from '../src/windows/MainWindow.vue';
import CategoryManager from '../src/components/CategoryManager.vue';
import DialogFrame from '../src/components/DialogFrame.vue';
import App from '../src/App.vue';
import { task, snapshot } from './fixtures';
import { localDate } from '../src/domain/tasks';
const api = vi.hoisted(() => ({
  snapshot: vi.fn(),
  onChange: vi.fn(),
  create: vi.fn(),
  update: vi.fn(),
  status: vi.fn(),
  purge: vi.fn(),
  saveCategory: vi.fn(),
  deleteCategory: vi.fn(),
}));
vi.mock('../src/services/tasks', () => ({ taskBridge: api }));
import { useTasksStore } from '../src/stores/tasks';
let pinia: ReturnType<typeof createPinia>, current: ReturnType<typeof useTasksStore>;
const wrappers: VueWrapper[] = [];
function mountUi(component: Component, props: Record<string, unknown> = {}) {
  const wrapper = mount(component, {
    props,
    global: { plugins: [pinia], stubs: { Teleport: true } },
    attachTo: document.body,
  });
  wrappers.push(wrapper);
  return wrapper;
}
function button(wrapper: VueWrapper, text: string) {
  const found = wrapper.findAll('button').find((item) => item.text() === text);
  if (!found) throw new Error(`Missing visible action ${text}`);
  return found;
}
beforeEach(() => {
  vi.resetAllMocks();
  pinia = createPinia();
  setActivePinia(pinia);
  current = useTasksStore();
  current.apply(snapshot());
  current.ready = true;
});
afterEach(() => {
  wrappers.splice(0).forEach((item) => item.unmount());
  current.dispose();
  document.body.innerHTML = '';
  vi.unstubAllGlobals();
});

describe('application error boundaries', () => {
  it.each(['main', 'quick-entry'])(
    'preserves the %s draft when a write error reaches the root banner',
    async (label) => {
      vi.stubGlobal('matchMedia', () => ({
        matches: false,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      }));
      vi.stubGlobal('location', { search: `?window=${label}` });
      api.onChange.mockResolvedValue(vi.fn());
      api.snapshot.mockResolvedValue(snapshot());
      api.create.mockRejectedValue({ code: 'SERVICE_UNAVAILABLE', message: '保存失败，请重试' });
      const wrapper = mountUi(App);
      await flushPromises();
      if (label === 'main') await button(wrapper, '新建任务').trigger('click');
      const field = label === 'main' ? 'input[placeholder="想完成什么？"]' : '#quick-title';
      await wrapper.get(field).setValue('错误后必须保留的草稿');
      await wrapper.get('form').trigger('submit');
      await flushPromises();
      expect(wrapper.get('.task-error').text()).toContain('保存失败');
      expect(wrapper.get(field).element).toHaveProperty('value', '错误后必须保留的草稿');
      expect(wrapper.text()).not.toContain('任务已保存。');
      expect(wrapper.text()).not.toContain('已添加到收集箱');
    },
  );
});

describe('task editing interactions', () => {
  it('limits creation dates to today onward and preserves an invalid draft', async () => {
    const previous = new Date();
    previous.setDate(previous.getDate() - 1);
    const wrapper = mountUi(DetailDrawer, { task: null });
    expect(wrapper.get('input[type="date"]').attributes('min')).toBe(current.today);
    await wrapper.get('input[placeholder="想完成什么？"]').setValue('过去日期草稿');
    await wrapper.get('input[type="date"]').setValue(localDate(previous));
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(api.create).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('今天或之后');
    expect(wrapper.get('input[placeholder="想完成什么？"]').element).toHaveProperty(
      'value',
      '过去日期草稿',
    );
  });
  it('keeps Escape scoped to the top dialog even if focus returns to the document', async () => {
    const underlying = mountUi(DialogFrame, { title: '下层' });
    const top = mountUi(DialogFrame, { title: '上层' });
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(top.emitted('close')).toHaveLength(1);
    expect(underlying.emitted('close')).toBeUndefined();
  });
  it('returns an escaped Tab focus to the active dialog', async () => {
    const wrapper = mountUi(DialogFrame, { title: '焦点测试' });
    document.body.setAttribute('tabindex', '0');
    document.body.focus();
    window.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }),
    );
    expect(wrapper.get('[data-taskdock-dialog]').element.contains(document.activeElement)).toBe(
      true,
    );
    document.body.removeAttribute('tabindex');
  });
  it('retains a failed edit, does not claim saved, and allows retry', async () => {
    api.update
      .mockRejectedValueOnce({ code: 'SERVICE_UNAVAILABLE', message: '保存失败，请重试' })
      .mockResolvedValueOnce({
        snapshot: snapshot(2, [task({ title: '编辑后的标题', version: 2 })]),
        taskId: task().id,
        categoryId: null,
      });
    const wrapper = mountUi(DetailDrawer, { task: task() });
    await wrapper.get('input[placeholder="想完成什么？"]').setValue('编辑后的标题');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(wrapper.get('input[placeholder="想完成什么？"]').element).toHaveProperty(
      'value',
      '编辑后的标题',
    );
    expect(wrapper.text()).toContain('保存失败');
    expect(wrapper.text()).toContain('有未保存的修改');
    expect(wrapper.emitted('saved')).toBeUndefined();
    expect(current.tasks[0]?.title).toBe('准备周计划');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(wrapper.emitted('saved')?.[0]).toEqual([task().id]);
    expect(current.tasks[0]?.title).toBe('编辑后的标题');
  });
  it('validates a blank title before contacting the service', async () => {
    const wrapper = mountUi(DetailDrawer, { task: null });
    await wrapper.get('form').trigger('submit');
    expect(api.create).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('标题不能为空');
  });
  it('edits, completes, reorders and removes subtask drafts in the saved aggregate', async () => {
    api.update.mockResolvedValue({ snapshot: snapshot(2), taskId: task().id, categoryId: null });
    const wrapper = mountUi(DetailDrawer, { task: task() });
    await button(wrapper, '添加子任务').trigger('click');
    await wrapper.get('input[aria-label="子任务标题：1"]').setValue('第一步');
    await button(wrapper, '添加子任务').trigger('click');
    await wrapper.get('input[aria-label="子任务标题：2"]').setValue('第二步');
    await wrapper.get('input[aria-label="子任务完成：2"]').setValue(true);
    await wrapper.get('button[aria-label="上移子任务：2"]').trigger('click');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(api.update.mock.calls[0]?.[2].subtasks).toEqual([
      { id: null, title: '第二步', completed: true },
      { id: null, title: '第一步', completed: false },
    ]);
    await wrapper.get('button[aria-label="删除子任务：2"]').trigger('click');
    expect(wrapper.findAll('.subtask-edit-row')).toHaveLength(1);
  });
  it('preserves the current draft when a newer version arrives and requires explicit reload', async () => {
    const wrapper = mountUi(DetailDrawer, { task: task() });
    await wrapper.get('input[placeholder="想完成什么？"]').setValue('我的未保存草稿');
    const latest = task({ title: '来自另一个窗口', version: 2 });
    current.apply(snapshot(2, [latest]));
    await wrapper.setProps({ task: latest });
    expect(wrapper.text()).toContain('其他窗口已修改');
    expect(button(wrapper, '保存任务').attributes('disabled')).toBeDefined();
    expect(wrapper.get('input[placeholder="想完成什么？"]').element).toHaveProperty(
      'value',
      '我的未保存草稿',
    );
    api.snapshot.mockResolvedValue(snapshot(2, [latest]));
    await button(wrapper, '放弃草稿并重新载入').trigger('click');
    await flushPromises();
    expect(wrapper.get('input[placeholder="想完成什么？"]').element).toHaveProperty(
      'value',
      '来自另一个窗口',
    );
  });
});

describe('list, quick entry and categories', () => {
  it('keeps a visible timeout label in the collapsed widget row without showing actions', () => {
    const due = Date.parse('2026-10-28T06:00:00Z');
    const wrapper = mountUi(TaskRow, {
      task: task({ plannedDate: '2026-10-30', dueAt: new Date(due).toISOString() }),
      today: '2026-10-28',
      now: due,
      categories: [],
      compact: true,
      minimal: true,
    });
    expect(wrapper.classes()).toContain('task-timed-out');
    expect(wrapper.get('.task-title').text()).toContain('超时 ·');
    expect(wrapper.find('.task-actions').exists()).toBe(false);
    expect(wrapper.find('.task-metadata').exists()).toBe(false);
  });
  it('requires confirmation before completing a parent with unfinished subtasks', async () => {
    const initial = task({
      progress: { total: 1, completed: 0, percent: 0 },
      subtasks: [
        {
          id: 'sub',
          title: '未完成',
          completedAt: null,
          sortOrder: 0,
          createdAt: '2026-10-28T00:00:00Z',
          updatedAt: '2026-10-28T00:00:00Z',
        },
      ],
    });
    current.apply(snapshot(1, [initial]));
    api.status.mockResolvedValue({
      snapshot: snapshot(2, [
        { ...initial, status: 'completed', completedAt: '2026-10-28T02:00:00Z', version: 2 },
      ]),
      taskId: initial.id,
      categoryId: null,
    });
    const wrapper = mountUi(
      defineComponent({ setup: () => () => h(TaskList, { tasks: current.tasks }) }),
    );
    await wrapper.get('button[role="checkbox"]').trigger('click');
    expect(api.status).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('仍要完成父任务吗');
    await button(wrapper, '仍然完成').trigger('click');
    await flushPromises();
    expect(api.status).toHaveBeenCalledWith(initial.id, 1, 'completed', true);
    expect(wrapper.get('button[role="checkbox"]').attributes('aria-checked')).toBe('true');
    expect(wrapper.find('input[type=checkbox]').exists()).toBe(false);
    expect(current.tasks[0]?.progress.completed).toBe(0);
  });
  it('keeps the trailing completion box unchecked when a completion write fails', async () => {
    api.status.mockRejectedValue({ code: 'CONFLICT', message: '其他窗口已修改' });
    const wrapper = mountUi(
      defineComponent({ setup: () => () => h(TaskList, { tasks: current.tasks }) }),
    );
    await wrapper.get('button[role="checkbox"]').trigger('click');
    await flushPromises();
    expect(wrapper.get('button[role="checkbox"]').attributes('aria-checked')).toBe('false');
    expect(wrapper.find('input[type=checkbox]').exists()).toBe(false);
    expect(current.error?.code).toBe('CONFLICT');
  });
  it('requires explicit confirmation and preserves a failed permanent deletion for retry', async () => {
    const initial = task();
    api.purge.mockRejectedValueOnce({ code: 'CONFLICT', message: '其他窗口已修改' });
    const wrapper = mountUi(TaskList, { tasks: [initial] });
    await wrapper.get(`button[aria-label="删除任务：${initial.title}"]`).trigger('click');
    expect(api.purge).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain('无法撤销或恢复');
    await button(wrapper, '取消').trigger('click');
    expect(api.purge).not.toHaveBeenCalled();
    await wrapper.get(`button[aria-label="删除任务：${initial.title}"]`).trigger('click');
    await button(wrapper, '永久删除').trigger('click');
    await flushPromises();
    expect(current.tasks[0]?.id).toBe(initial.id);
    expect(wrapper.text()).toContain('其他窗口已修改');
    expect(button(wrapper, '永久删除').exists()).toBe(true);
    api.purge.mockResolvedValueOnce({
      snapshot: snapshot(2, []),
      taskId: initial.id,
      categoryId: null,
    });
    await button(wrapper, '永久删除').trigger('click');
    await flushPromises();
    expect(api.purge).toHaveBeenCalledWith(initial.id, initial.version, true);
    expect(current.tasks).toEqual([]);
    expect(wrapper.text()).not.toContain('永久删除任务');
  });
  it('preserves an unsaved drawer draft when permanent deletion fails', async () => {
    const initial = task();
    api.purge.mockRejectedValue({ code: 'CONFLICT', message: '其他窗口已修改' });
    const wrapper = mountUi(DetailDrawer, { task: initial });
    await wrapper.get('input[placeholder="想完成什么？"]').setValue('我的未保存草稿');
    await button(wrapper, '删除任务').trigger('click');
    expect(wrapper.text()).toContain('未保存的修改也会丢弃');
    expect(api.purge).not.toHaveBeenCalled();
    await button(wrapper, '永久删除').trigger('click');
    await flushPromises();
    expect(wrapper.emitted('deleted')).toBeUndefined();
    expect(wrapper.text()).toContain('其他窗口已修改');
    await wrapper
      .findAll('button')
      .filter((item) => item.text() === '取消')
      .at(-1)!
      .trigger('click');
    expect(wrapper.get('input[placeholder="想完成什么？"]').element).toHaveProperty(
      'value',
      '我的未保存草稿',
    );
  });
  it('does not clear the quick-entry draft on failure and clears it only after committed success', async () => {
    api.create
      .mockRejectedValueOnce({ code: 'SERVICE_UNAVAILABLE', message: '保存未完成' })
      .mockResolvedValueOnce({ snapshot: snapshot(2), taskId: 'new', categoryId: null });
    const wrapper = mountUi(QuickEntryWindow);
    await wrapper.get('#quick-title').setValue('快速记录');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(wrapper.get('#quick-title').element).toHaveProperty('value', '快速记录');
    expect(wrapper.text()).toContain('保存未完成');
    expect(wrapper.text()).not.toContain('已添加到今天');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(wrapper.get('#quick-title').element).toHaveProperty('value', '');
    expect(wrapper.text()).toContain('已添加到今天');
    expect(api.create.mock.calls[1]?.[0].plannedDate).toBe(current.today);
  });
  it('switches between task date tabs and the independent wishlist', async () => {
    current.today = '2026-10-28';
    current.apply(
      snapshot(2, [
        task({ id: 'future', title: '明天的任务', plannedDate: '2026-10-29' }),
        task({ id: 'inbox', title: '收集的想法', plannedDate: null }),
      ]),
    );
    const wrapper = mountUi(MainWindow);
    await wrapper.get('.task-view-tabs').findAll('button')[1]!.trigger('click');
    expect(wrapper.text()).toContain('明天的任务');
    expect(wrapper.text()).not.toContain('收集的想法');
    await wrapper.get('nav').findAll('button')[1]!.trigger('click');
    expect(wrapper.text()).toContain('收集的想法');
    expect(wrapper.text()).not.toContain('明天的任务');
  });
  it('keeps a category name draft when saving fails', async () => {
    api.saveCategory.mockRejectedValue({ code: 'DUPLICATE_CATEGORY', message: '已存在同名分类' });
    const wrapper = mountUi(CategoryManager);
    await wrapper.get('input:not([type=color])').setValue('工作');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(wrapper.text()).toContain('已存在同名分类');
    expect(wrapper.get('input:not([type=color])').element).toHaveProperty('value', '工作');
    expect(current.categories).toEqual([]);
  });
  it('removes the previous save notice before a new edit can fail', async () => {
    const savedTask = task({ title: '第一次保存' });
    api.create.mockResolvedValue({
      snapshot: snapshot(2, [savedTask]),
      taskId: savedTask.id,
      categoryId: null,
    });
    api.update.mockRejectedValue({ code: 'CONFLICT', message: '其他窗口已修改' });
    const wrapper = mountUi(MainWindow);
    await button(wrapper, '新建任务').trigger('click');
    await wrapper.get('input[placeholder="想完成什么？"]').setValue('第一次保存');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(wrapper.text()).toContain('任务已保存。');
    await button(wrapper, '第一次保存').trigger('click');
    await wrapper.get('input[placeholder="想完成什么？"]').setValue('第二次编辑');
    await wrapper.get('form').trigger('submit');
    await flushPromises();
    expect(wrapper.text()).not.toContain('任务已保存。');
    expect(wrapper.text()).toContain('其他窗口已修改');
    expect(wrapper.get('input[placeholder="想完成什么？"]').element).toHaveProperty(
      'value',
      '第二次编辑',
    );
  });
});
