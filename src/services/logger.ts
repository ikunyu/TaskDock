import { bridge } from './bridge';

export type FrontendErrorCode =
  | 'VUE_ERROR'
  | 'UNHANDLED_REJECTION'
  | 'WINDOW_ERROR'
  | 'SERVICE_ERROR';

export function reportError(code: FrontendErrorCode): void {
  // Only send a fixed code, never an exception message or task content.
  void bridge.reportError(code).catch(() => console.warn('[TaskDock] LOG_UNAVAILABLE'));
}
