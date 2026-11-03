import { describe, expect, it, vi, afterEach } from 'vitest';
import { task } from './fixtures';
import {
  blankDraft,
  inView,
  tasksForView,
  overdue,
  isTimedOut,
  validDate,
  widgetTasks,
  localDate,
  toUtcDateTime,
  toLocalDateTime,
  validateDraft,
  validateCreationDates,
} from '../src/domain/tasks';
afterEach(() => vi.useRealTimers());

describe('calendar dates and four task views', () => {
  it('rejects past dates only in the creation policy, preserving general historical validation', () => {
    const draft = blankDraft('2026-10-29');
    draft.title = '保留草稿';
    expect(validateCreationDates(draft, '2026-10-30')).toContain('今天或之后');
    expect(validateDraft(draft)).toBeNull();
    draft.plannedDate = '2026-10-30';
    expect(validateCreationDates(draft, '2026-10-30')).toBeNull();
    draft.plannedDate = '2026-10-31';
    draft.dueDate = '2026-10-29';
    expect(validateCreationDates(draft, '2026-10-30')).toContain('今天或之后');
    draft.dueDate = null;
    draft.dueAt = new Date(2026, 9, 29, 23, 59).toISOString();
    expect(validateCreationDates(draft, '2026-10-30')).toContain('今天或之后');
    draft.dueAt = new Date(2026, 9, 30, 0, 0).toISOString();
    expect(validateCreationDates(draft, '2026-10-30')).toBeNull();
    expect(validateCreationDates(blankDraft(), '2026-10-30')).toBeNull();
  });
  it('keeps crossed-off wishes in the wishlist and out of dated task views', () => {
    const wish = task({
      plannedDate: null,
      status: 'completed',
      completedAt: '2026-10-28T02:00:00Z',
    });
    expect(inView(wish, 'inbox', '2026-10-28')).toBe(true);
    for (const view of ['today', 'plan', 'done'] as const) {
      expect(inView(wish, view, '2026-10-28')).toBe(false);
    }
    expect(widgetTasks([wish], '2026-10-28')).toHaveLength(0);
  });
  it('filters inbox, overdue/today, future, completed and deleted without overlap', () => {
    const cases = [
      task({ id: 'inbox', plannedDate: null }),
      task({ id: 'old', plannedDate: '2026-10-27' }),
      task({ id: 'today' }),
      task({ id: 'future', plannedDate: '2026-10-29' }),
      task({ id: 'done', status: 'completed', completedAt: '2026-10-28T02:00:00Z' }),
      task({ id: 'deleted', deletedAt: '2026-10-28T03:00:00Z' }),
    ];
    expect(tasksForView(cases, 'inbox', '2026-10-28').map((item) => item.id)).toEqual(['inbox']);
    expect(tasksForView(cases, 'today', '2026-10-28').map((item) => item.id)).toEqual([
      'old',
      'today',
    ]);
    expect(tasksForView(cases, 'plan', '2026-10-28').map((item) => item.id)).toEqual(['future']);
    expect(tasksForView(cases, 'done', '2026-10-28').map((item) => item.id)).toEqual(['done']);
  });
  it('moves a future task to today and marks unfinished tasks overdue across midnight', () => {
    const item = task({ plannedDate: '2026-10-29' });
    expect(inView(item, 'plan', '2026-10-28')).toBe(true);
    expect(inView(item, 'today', '2026-10-29')).toBe(true);
    expect(overdue(item, '2026-10-29')).toBeNull();
    expect(overdue(item, '2026-10-30')).toBe('超时任务');
  });
  it('keeps deadline-only inbox tasks in inbox and treats date-only deadlines as whole days', () => {
    const item = task({ plannedDate: null, dueDate: '2026-10-28' });
    expect(inView(item, 'inbox', '2026-10-28')).toBe(true);
    expect(overdue(item, '2026-10-28')).toBeNull();
    expect(overdue(item, '2026-10-29')).toBe('超时任务');
    expect(
      overdue(
        task({ dueAt: '2026-10-28T00:00:00Z' }),
        '2026-10-28',
        Date.parse('2026-10-28T00:00:01Z'),
      ),
    ).toBe('超时任务');
  });
  it('moves a future-planned task into today exactly at its deadline in both task lists', () => {
    const due = Date.parse('2026-10-28T06:00:00Z');
    const item = task({ plannedDate: '2026-10-30', dueAt: new Date(due).toISOString() });
    expect(inView(item, 'plan', '2026-10-28', due - 1)).toBe(true);
    expect(widgetTasks([item], '2026-10-28', due - 1)).toEqual([]);
    expect(isTimedOut(item, '2026-10-28', due)).toBe(true);
    expect(inView(item, 'plan', '2026-10-28', due)).toBe(false);
    expect(tasksForView([item], 'today', '2026-10-28', due)).toEqual([item]);
    expect(widgetTasks([item], '2026-10-28', due)).toEqual([item]);
    expect(overdue(item, '2026-10-28', due)).toBe('超时任务');
  });
  it('honors an explicit later deadline and treats a date-only deadline as a full local day', () => {
    const item = task({ plannedDate: '2026-10-27', dueDate: '2026-10-28' });
    expect(isTimedOut(item, '2026-10-28')).toBe(false);
    expect(overdue(item, '2026-10-28')).toBe('计划逾期');
    expect(isTimedOut(item, '2026-10-29')).toBe(true);
    expect(isTimedOut(task({ plannedDate: null }), '2026-10-29')).toBe(false);
  });
  it('orders timed-out tasks first and removes their marker on completion or deadline extension', () => {
    const due = Date.parse('2026-10-28T06:00:00Z');
    const expired = task({
      id: 'expired',
      plannedDate: '2026-10-30',
      dueAt: new Date(due).toISOString(),
    });
    const normal = task({ id: 'today' });
    expect(
      tasksForView([normal, expired], 'today', '2026-10-28', due).map((item) => item.id),
    ).toEqual(['expired', 'today']);
    const completed = {
      ...expired,
      status: 'completed' as const,
      completedAt: new Date(due + 1000).toISOString(),
    };
    expect(overdue(completed, '2026-10-28', due)).toBeNull();
    expect(widgetTasks([completed], '2026-10-28', due)).toEqual([completed]);
    expect(
      widgetTasks([{ ...expired, deletedAt: new Date(due).toISOString() }], '2026-10-28', due),
    ).toEqual([]);
    expect(
      inView({ ...expired, dueAt: new Date(due + 1000).toISOString() }, 'plan', '2026-10-28', due),
    ).toBe(true);
  });
  it('retains completed tasks in the widget so completion can be undone', () => {
    const done = task({
      status: 'completed',
      completedAt: new Date(2026, 9, 28, 12).toISOString(),
    });
    expect(tasksForView([done], 'today', '2026-10-28')).toEqual([]);
    expect(widgetTasks([done], '2026-10-28')).toHaveLength(1);
    expect(widgetTasks([done], '2026-10-29')).toHaveLength(0);
  });
  it('keeps future completions in their original slots and out of the ordinary today list', () => {
    const now = new Date(2026, 10, 2, 12).getTime();
    const items = [
      task({ id: 'first', plannedDate: '2026-11-04', sortOrder: 0 }),
      task({ id: 'second', plannedDate: '2026-11-04', sortOrder: 1024 }),
      task({ id: 'third', plannedDate: '2026-11-09', sortOrder: 2048 }),
    ];
    const completed = items.map((item) =>
      item.id === 'second'
        ? { ...item, status: 'completed' as const, completedAt: new Date(now).toISOString() }
        : item,
    );
    expect(widgetTasks(items, '2026-11-02', now, 'plan').map((item) => item.id)).toEqual([
      'first',
      'second',
      'third',
    ]);
    expect(widgetTasks(completed, '2026-11-02', now, 'plan').map((item) => item.id)).toEqual([
      'first',
      'second',
      'third',
    ]);
    expect(widgetTasks(completed, '2026-11-02', now)).toEqual([]);
    expect(widgetTasks(items, '2026-11-02', now, 'plan').map((item) => item.id)).toEqual([
      'first',
      'second',
      'third',
    ]);
  });
  it('does not append today completions or overdue completions below other items', () => {
    const now = new Date(2026, 10, 2, 12).getTime();
    const items = [
      task({ id: 'first', plannedDate: '2026-11-02', sortOrder: 0 }),
      task({ id: 'second', plannedDate: '2026-11-02', sortOrder: 1024 }),
      task({ id: 'third', plannedDate: '2026-11-02', sortOrder: 2048 }),
    ];
    const completed = items.map((item) =>
      item.id === 'first'
        ? { ...item, status: 'completed' as const, completedAt: new Date(now).toISOString() }
        : item,
    );
    expect(widgetTasks(completed, '2026-11-02', now).map((item) => item.id)).toEqual([
      'first',
      'second',
      'third',
    ]);
    const overdueTask = task({
      id: 'late-future',
      plannedDate: '2026-11-05',
      dueAt: new Date(now - 1000).toISOString(),
    });
    const before = widgetTasks([...items, overdueTask], '2026-11-02', now).map((item) => item.id);
    const after = widgetTasks(
      [...items, { ...overdueTask, status: 'completed', completedAt: new Date(now).toISOString() }],
      '2026-11-02',
      now,
    );
    expect(after.map((item) => item.id)).toEqual(before);
    expect(widgetTasks(after, '2026-11-02', now, 'plan')).toEqual([]);
  });
  it.each(['2026-02-29', '2026-02-30', '2026-04-31', '2026-13-01', '2026-1-01', '0000-01-01'])(
    'rejects invalid plain date %s',
    (date) => expect(validDate(date)).toBe(false),
  );
  it.each(['2024-02-29', '2026-10-28', '0001-01-01'])('accepts strict calendar date %s', (date) =>
    expect(validDate(date)).toBe(true),
  );
  it('converts precise local times independently of planned calendar dates', () => {
    const local = '2026-10-28T18:35';
    const utc = toUtcDateTime(local);
    expect(toLocalDateTime(utc)).toBe(local);
    expect(utc).toMatch(/Z$/);
    const draft = blankDraft('2026-10-28');
    draft.title = '独立日期';
    draft.dueAt = utc;
    expect(draft.plannedDate).toBe('2026-10-28');
    expect(validateDraft(draft)).toBeNull();
    expect(() => toUtcDateTime('2026-02-30T18:35')).toThrow();
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 9, 29, 0, 1));
    expect(localDate()).toBe('2026-10-29');
  });
});
