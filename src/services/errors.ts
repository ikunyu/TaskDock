import type { AppError } from '../types';

export function toAppError(value: unknown): AppError {
  if (
    typeof value === 'object' &&
    value !== null &&
    'code' in value &&
    'message' in value &&
    typeof value.code === 'string' &&
    typeof value.message === 'string'
  ) {
    return { code: value.code, message: value.message };
  }
  // Arbitrary JS/IPC errors can contain paths or task content; do not display them.
  return { code: 'UNEXPECTED', message: '操作未完成，请重试。详细信息已记录到本地日志。' };
}
