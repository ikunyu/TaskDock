import { describe, expect, it } from 'vitest';
import { toAppError } from '../src/services/errors';

describe('error boundary', () => {
  it('preserves structured service codes and safe messages', () => {
    expect(toAppError({ code: 'FORBIDDEN', message: '没有权限' })).toEqual({
      code: 'FORBIDDEN',
      message: '没有权限',
    });
  });
  it.each([
    new Error('secret task content'),
    'internal path',
    null,
    { code: 1, message: 'unsafe' },
  ])('does not expose arbitrary exceptions: %s', (error) => {
    expect(toAppError(error).code).toBe('UNEXPECTED');
    expect(toAppError(error).message).not.toContain('secret');
  });
});
