import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { blankDraft } from '../src/domain/tasks';
import { snapshot, task } from './fixtures';
import type { TaskSnapshot } from '../src/task-types';
const api = vi.hoisted(() => ({
  snapshot: vi.fn(),
  onChange: vi.fn(),
  create: vi.fn(),
  update: vi.fn(),
  status: vi.fn(),
  deleted: vi.fn(),
  saveCategory: vi.fn(),
  deleteCategory: vi.fn(),
}));
vi.mock('../src/services/tasks', () => ({ taskBridge: api }));
import { useTasksStore } from '../src/stores/tasks';
let stores: ReturnType<typeof useTasksStore>[] = [];
function store() {
  const result = useTasksStore(createPinia());
  stores.push(result);
  return result;
}
beforeEach(() => {
  vi.resetAllMocks();
  setActivePinia(createPinia());
  api.onChange.mockResolvedValue(vi.fn());
  api.snapshot.mockResolvedValue(snapshot());
});
afterEach(() => {
  stores.forEach((item) => item.dispose());
  stores = [];
  vi.useRealTimers();
});

describe('versioned task synchronization', () => {
  it('refreshes an approaching deadline without database polling or changing the task status', async () => {
    vi.useFakeTimers();
    const start = new Date(2026, 9, 28, 12).getTime();
    vi.setSystemTime(start);
    api.snapshot.mockResolvedValue(
      snapshot(1, [
        task({ plannedDate: '2026-10-30', dueAt: new Date(start + 5000).toISOString() }),
      ]),
    );
    const current = store();
    await current.initialize();
    expect(current.view('today')).toHaveLength(0);
    expect(current.view('plan')).toHaveLength(1);
    vi.advanceTimersByTime(5001);
    expect(current.view('today')).toHaveLength(1);
    expect(current.view('plan')).toHaveLength(0);
    expect(current.tasks[0]?.status).toBe('pending');
    expect(current.revision).toBe(1);
    expect(api.snapshot).toHaveBeenCalledTimes(1);
    expect(api.status).not.toHaveBeenCalled();
    current.dispose();
    expect(vi.getTimerCount()).toBe(0);
  });
  it('subscribes before snapshot and coalesces events arriving during a read', async () => {
    let notify!: (revision: number) => void;
    let resolve!: (value: TaskSnapshot) => void;
    api.onChange.mockImplementation(async (callback) => {
      notify = callback;
      return vi.fn();
    });
    api.snapshot
      .mockReturnValueOnce(
        new Promise((done) => {
          resolve = done;
        }),
      )
      .mockResolvedValueOnce(snapshot(4));
    const current = store();
    const initializing = current.initialize();
    await Promise.resolve();
    notify(2);
    notify(3);
    notify(4);
    resolve(snapshot(1));
    await initializing;
    expect(api.snapshot).toHaveBeenCalledTimes(2);
    expect(current.revision).toBe(4);
    expect(api.onChange.mock.invocationCallOrder[0]).toBeLessThan(
      api.snapshot.mock.invocationCallOrder[0]!,
    );
  });
  it('does not replace a newer event snapshot with an older write response', async () => {
    const current = store();
    current.apply(snapshot(3));
    api.create.mockImplementation(async () => {
      current.apply(snapshot(5, [task({ title: '最新内容', version: 5 })]));
      return { snapshot: snapshot(4), taskId: 'new', categoryId: null };
    });
    await current.create(blankDraft());
    expect(current.revision).toBe(5);
    expect(current.tasks[0]?.title).toBe('最新内容');
  });
  it('retains persisted data on write failure and serializes local write attempts', async () => {
    const current = store();
    current.apply(snapshot());
    let reject!: (reason: unknown) => void;
    api.create.mockReturnValue(
      new Promise((_, fail) => {
        reject = fail;
      }),
    );
    const saving = current.create(blankDraft());
    await expect(current.create(blankDraft())).rejects.toMatchObject({ code: 'BUSY' });
    reject({ code: 'SERVICE_UNAVAILABLE', message: '保存失败' });
    await expect(saving).rejects.toMatchObject({ code: 'SERVICE_UNAVAILABLE' });
    expect(current.tasks[0]?.title).toBe('准备周计划');
    expect(current.revision).toBe(1);
    expect(current.busy).toBe(false);
  });
  it('disposes listeners and discards an in-flight read after unmount', async () => {
    let resolve!: (value: TaskSnapshot) => void;
    const stop = vi.fn();
    api.onChange.mockResolvedValue(stop);
    api.snapshot.mockReturnValue(
      new Promise((done) => {
        resolve = done;
      }),
    );
    const current = store();
    const initializing = current.initialize();
    await Promise.resolve();
    current.dispose();
    resolve(snapshot(7));
    await initializing;
    expect(stop).toHaveBeenCalledOnce();
    expect(current.revision).toBe(-1);
  });
  it('synchronizes completion and undo between two window stores through committed revisions', async () => {
    let server = snapshot();
    const listeners = new Set<(revision: number) => void>();
    api.onChange.mockImplementation(async (callback) => {
      listeners.add(callback);
      return () => listeners.delete(callback);
    });
    api.snapshot.mockImplementation(async () => structuredClone(server));
    api.status.mockImplementation(async (id, _version, status) => {
      server = snapshot(server.revision + 1, [
        task({
          id,
          status,
          version: server.revision + 1,
          completedAt: status === 'completed' ? '2026-10-28T01:00:00Z' : null,
        }),
      ]);
      listeners.forEach((callback) => callback(server.revision));
      return { snapshot: structuredClone(server), taskId: id, categoryId: null };
    });
    const main = store(),
      widget = store();
    await Promise.all([main.initialize(), widget.initialize()]);
    await main.status(main.tasks[0]!, 'completed');
    await widget.refresh();
    expect(widget.tasks[0]?.status).toBe('completed');
    await widget.status(widget.tasks[0]!, 'pending');
    await main.refresh();
    expect(main.tasks[0]?.status).toBe('pending');
    expect(main.revision).toBe(widget.revision);
  });
  it('updates the local date across midnight without changing stored planned dates', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 9, 28, 23, 59, 50));
    const current = store();
    await current.initialize();
    expect(current.today).toBe('2026-10-28');
    vi.advanceTimersByTime(30_000);
    expect(current.today).toBe('2026-10-29');
    expect(current.tasks[0]?.plannedDate).toBe('2026-10-28');
  });
  it('does not erase a failed-write message when a delayed snapshot succeeds', async () => {
    const current = store();
    current.apply(snapshot());
    api.status.mockRejectedValue({ code: 'CONFLICT', message: '其他窗口已修改' });
    await expect(current.status(current.tasks[0]!, 'completed')).rejects.toMatchObject({
      code: 'CONFLICT',
    });
    api.snapshot.mockResolvedValue(snapshot(2));
    await current.refresh();
    expect(current.error?.code).toBe('CONFLICT');
    expect(current.tasks[0]?.status).toBe('pending');
  });
});
