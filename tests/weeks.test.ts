import { describe, expect, it } from 'vitest';
import {
  groupWeek,
  groupTaskWeeks,
  mondayOf,
  shiftDate,
  taskCalendarDate,
} from '../src/domain/weeks';
import { task } from './fixtures';

describe('Monday to Sunday overview', () => {
  it('preserves the persisted row order when the first task is completed', () => {
    const now = new Date(2026, 10, 2, 12).getTime();
    const tasks = [
      task({ id: 'first', plannedDate: '2026-11-02', sortOrder: 0 }),
      task({ id: 'second', plannedDate: '2026-11-02', sortOrder: 1024 }),
    ];
    const before = groupWeek(tasks, '2026-11-02', '2026-11-02', now)[0]!.tasks.map(
      (item) => item.id,
    );
    const changed = tasks.map((item) =>
      item.id === 'first'
        ? { ...item, status: 'completed' as const, completedAt: new Date(now).toISOString() }
        : item,
    );
    expect(
      groupWeek(changed, '2026-11-02', '2026-11-02', now)[0]!.tasks.map((item) => item.id),
    ).toEqual(before);
  });
  it('groups future items into ordered Monday-Sunday ranges and omits empty groups', () => {
    const groups = groupTaskWeeks([
      task({ id: 'next', plannedDate: '2026-11-09' }),
      task({ id: 'this-sunday', plannedDate: '2026-11-08' }),
      task({ id: 'this-tuesday', plannedDate: '2026-11-03' }),
      task({ id: 'year-end', plannedDate: '2027-01-01' }),
      task({ id: 'wish', plannedDate: null }),
      task({ id: 'removed', plannedDate: '2026-11-16', deletedAt: '2026-11-01T00:00:00Z' }),
    ]);
    expect(groups.map((week) => [week.start, week.end])).toEqual([
      ['2026-11-02', '2026-11-08'],
      ['2026-11-09', '2026-11-15'],
      ['2026-12-28', '2027-01-03'],
    ]);
    expect(groups[0]?.tasks.map((item) => item.id)).toEqual(['this-sunday', 'this-tuesday']);
    expect(groupTaskWeeks([])).toEqual([]);
  });
  it('uses the same week for Monday through Sunday, including across a month boundary', () => {
    expect(mondayOf('2026-11-02')).toBe('2026-11-02');
    expect(mondayOf('2026-11-08')).toBe('2026-11-02');
    expect(mondayOf('2026-11-01')).toBe('2026-10-26');
    expect(shiftDate('2026-10-26', 6)).toBe('2026-11-01');
  });
  it('keeps calendar dates stable across a year boundary and leap day', () => {
    expect(mondayOf('2027-01-01')).toBe('2026-12-28');
    expect(shiftDate('2026-12-28', 6)).toBe('2027-01-03');
    expect(shiftDate('2024-02-28', 1)).toBe('2024-02-29');
    expect(shiftDate('2024-02-29', 1)).toBe('2024-03-01');
    expect(() => mondayOf('2026-02-30')).toThrow();
  });
  it('includes completed and pending tasks but excludes deleted, undated and out-of-week records', () => {
    const days = groupWeek(
      [
        task({ id: 'monday', plannedDate: '2026-11-02' }),
        task({ id: 'sunday', plannedDate: '2026-11-08', status: 'completed' }),
        task({ id: 'wish', plannedDate: null }),
        task({ id: 'deleted', plannedDate: '2026-11-03', deletedAt: '2026-11-03T00:00:00Z' }),
        task({ id: 'previous', plannedDate: '2026-11-01' }),
        task({ id: 'following', plannedDate: '2026-11-09' }),
      ],
      '2026-11-02',
      '2026-11-02',
      new Date(2026, 10, 2, 12).getTime(),
    );
    expect(days).toHaveLength(7);
    expect(days[0]?.label).toBe('周一');
    expect(days[6]?.label).toBe('周日');
    expect(days.flatMap((day) => day.tasks.map((item) => item.id))).toEqual(['monday', 'sunday']);
    expect(days[6]?.date).toBe('2026-11-08');
  });
  it('uses planned dates first and local deadline dates as the fallback', () => {
    expect(taskCalendarDate(task({ plannedDate: '2026-11-04', dueDate: '2026-11-06' }))).toBe(
      '2026-11-04',
    );
    expect(taskCalendarDate(task({ plannedDate: null, dueDate: '2026-11-06' }))).toBe('2026-11-06');
    const instant = new Date(2026, 10, 2, 0, 30).toISOString();
    expect(taskCalendarDate(task({ plannedDate: null, dueAt: instant }))).toBe('2026-11-02');
  });
  it('combines category and completion filters without moving overdue tasks out of their assigned day', () => {
    const tasks = [
      task({ id: 'done', plannedDate: '2026-11-02', categoryId: 'work', status: 'completed' }),
      task({ id: 'pending', plannedDate: '2026-11-02', categoryId: 'work' }),
      task({ id: 'other', plannedDate: '2026-11-03' }),
    ];
    const now = new Date(2026, 10, 9, 12).getTime();
    expect(
      groupWeek(tasks, '2026-11-04', '2026-11-09', now, 'work', 'pending')[0]?.tasks.map(
        (item) => item.id,
      ),
    ).toEqual(['pending']);
    expect(
      groupWeek(tasks, '2026-11-04', '2026-11-09', now, 'uncategorized').flatMap((day) =>
        day.tasks.map((item) => item.id),
      ),
    ).toEqual(['other']);
    expect(
      groupWeek(tasks, '2026-11-04', '2026-11-09', now)[0]?.tasks.map((item) => item.id),
    ).toEqual(['done', 'pending']);
  });
});
