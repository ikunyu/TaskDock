import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
const read = (path: string) =>
  readFileSync(new URL(`../src-tauri/${path}`, import.meta.url), 'utf8');

describe('IPC security configuration', () => {
  it('keeps quick entry limited to creation and the widget limited to status changes', () => {
    const quick = JSON.parse(read('capabilities/quick-entry.json')).permissions;
    const widget = JSON.parse(read('capabilities/widget.json')).permissions;
    expect(quick).toContain('tasks-create');
    expect(quick).not.toContain('tasks-edit');
    expect(quick).not.toContain('tasks-status');
    expect(widget).toContain('tasks-status');
    expect(widget).not.toContain('tasks-create');
    expect(widget).not.toContain('tasks-edit');
  });
  it('grants only local named-window capabilities without broad core or filesystem access', () => {
    for (const name of ['shared', 'main', 'widget', 'quick-entry', 'widget-lock']) {
      const capability = JSON.parse(read(`capabilities/${name}.json`));
      expect(capability.local).toBe(true);
      expect(capability.remote).toBeUndefined();
      expect(capability.windows).not.toContain('*');
      expect(capability.permissions).not.toContain('core:default');
      expect(
        capability.permissions.some((permission: string) =>
          /^(fs|shell|sql|http|opener):/.test(permission),
        ),
      ).toBe(false);
    }
  });
  it('restricts theme changes to main and does not permit frontend event spoofing', () => {
    const shared = JSON.parse(read('capabilities/shared.json'));
    expect(shared.permissions).not.toContain('core:event:allow-emit');
    expect(shared.permissions).not.toContain('core:event:allow-emit-to');
    expect(shared.permissions).not.toContain('foundation-theme');
    expect(JSON.parse(read('capabilities/main.json')).permissions).toContain('foundation-theme');
  });
  it('uses a production CSP without remote scripts or eval', () => {
    const config = JSON.parse(read('tauri.conf.json'));
    expect(config.app.security.csp).toContain("script-src 'self'");
    expect(config.app.security.csp).not.toContain('unsafe-eval');
    expect(config.app.security.csp).not.toContain('https:');
    expect(
      config.app.windows.find((window: { label: string }) => window.label === 'widget'),
    ).toMatchObject({ width: 180, height: 240, visible: false });
  });
});

describe('desktop capabilities', () => {
  it('limits the lock control to settings reads and unlock without task access', () => {
    const permissions = JSON.parse(read('capabilities/widget-lock.json')).permissions;
    expect(permissions).toContain('desktop-widget-unlock');
    expect(permissions).not.toContain('tasks-read');
    expect(permissions).not.toContain('desktop-settings');
    expect(permissions).not.toContain('desktop-widget');
  });
  it('keeps native plugins behind bounded Rust commands', () => {
    const main = JSON.parse(read('capabilities/main.json')).permissions;
    const widget = JSON.parse(read('capabilities/widget.json')).permissions;
    const quick = JSON.parse(read('capabilities/quick-entry.json')).permissions;
    expect(main).toContain('desktop-settings');
    expect(widget).toContain('desktop-widget');
    expect(quick).not.toContain('desktop-settings');
    expect(widget).not.toContain('desktop-settings');
    expect(main).not.toContain('desktop-widget');
    for (const name of ['main', 'widget', 'quick-entry', 'shared']) {
      const permissions = JSON.parse(read(`capabilities/${name}.json`)).permissions as string[];
      expect(
        permissions.some((p) =>
          /^(global-shortcut|autostart|single-instance|core:window):/.test(p),
        ),
      ).toBe(false);
    }
  });
});
