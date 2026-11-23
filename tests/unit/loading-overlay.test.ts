import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

// The loading overlay is a full-window cover with the maximum z-index. It is
// safe only while it is on screen legitimately; a version that re-attached
// itself after being dismissed covered the page again on every single-page-app
// re-render and cleared the page's selection, so Cmd+C copied nothing (a user's
// report, 2026-11-23: "copy works sometimes").
describe('loading overlay', () => {
  const source = fs.readFileSync(
    path.join(process.cwd(), 'src-tauri/src/inject/loading.js'),
    'utf8',
  );

  it('never re-attaches itself once dismissed', () => {
    const mountStart = source.indexOf('function mount(');
    const mountEnd = source.indexOf('\n  }', mountStart);
    const mount = source.slice(mountStart, mountEnd);
    expect(mountStart).toBeGreaterThan(-1);
    // The guard must come before any appendChild in mount().
    expect(mount.indexOf('if (settled) return;')).toBeGreaterThan(-1);
    expect(mount.indexOf('if (settled) return;')).toBeLessThan(
      mount.indexOf('appendChild'),
    );
  });

  it('disconnects its mutation observer when it goes away', () => {
    const removeStart = source.indexOf('function remove(');
    const removeEnd = source.indexOf('\n  }', removeStart);
    const remove = source.slice(removeStart, removeEnd);
    expect(remove).toContain('mountObserver.disconnect()');
    expect(remove).toContain('if (settled) return;');
  });

  it('stops polling once the overlay is gone', () => {
    expect(source).toContain('clearInterval(contentPoll)');
  });

  it('is injected into the main frame only', () => {
    const windowSource = fs.readFileSync(
      path.join(process.cwd(), 'src-tauri/src/app/window.rs'),
      'utf8',
    );
    // Injecting it into every iframe covered embedded content, where a
    // whole-window overlay has no business being.
    expect(windowSource).toContain(
      '.initialization_script(include_str!("../inject/loading.js"))',
    );
    expect(windowSource).not.toContain(
      '.initialization_script_for_all_frames(include_str!("../inject/loading.js"))',
    );
  });
});
