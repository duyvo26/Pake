import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

// macOS Cmd+C is routed to the webview's first responder and is silently
// dropped when something else holds it, so copy "worked sometimes" (a user's
// report, 2026-11-23). The page now asks the host to write the selection, but
// only when its own `copy` event never arrived: a copy the webview handled must
// never be overwritten, or rich text and file copies would lose their data.
describe('macOS copy fallback', () => {
  const source = fs.readFileSync(
    path.join(process.cwd(), 'src-tauri/src/inject/copy_bridge.js'),
    'utf8',
  );
  const block = source;

  it('never runs twice in one document', () => {
    // Injected into every frame, so a repeated install would double-wrap
    // clipboard.writeText and stack copy handlers.
    expect(block).toContain('__pakeCopyBridgeInstalled');
  });

  it('waits for the webview to handle the copy itself', () => {
    // A grace period is what separates "the webview copied" from "the
    // keystroke was dropped"; firing immediately would race the native path.
    expect(block).toMatch(/COPY_GRACE_MS\s*=\s*\d+/);
    expect(block).toContain('setTimeout(function () {');
  });

  it('cancels the fallback when a copy event arrives', () => {
    const copyHandler = block.slice(block.indexOf('"copy"'));
    expect(copyHandler).toContain('pending = null');
  });

  it('ignores key presses that are not a plain Cmd+C', () => {
    for (const guard of [
      'event.isTrusted !== true',
      '!event.metaKey',
      'event.ctrlKey',
      'event.altKey',
      'event.shiftKey',
    ]) {
      expect(block).toContain(guard);
    }
  });

  it('asks the host to write the selection', () => {
    expect(block).toContain('invoke("copy_selection"');
    expect(block).toContain('selection.toString()');
  });
});

describe('copy_selection command', () => {
  const rust = fs.readFileSync(
    path.join(process.cwd(), 'src-tauri/src/app/clipboard.rs'),
    'utf8',
  );

  it('refuses to clear the pasteboard for a blank selection', () => {
    expect(rust).toContain('if !should_copy(&text)');
    expect(rust).toContain('!text.trim().is_empty()');
  });

  it('writes plain text to the general pasteboard', () => {
    expect(rust).toContain('NSPasteboard::generalPasteboard()');
    expect(rust).toContain('setString_forType');
  });

  it('keeps a copy that already landed', () => {
    // A failed WebKit copy leaves the pasteboard empty; a non-empty one holds
    // what the page copied and may carry rich text, so it is left alone.
    expect(rust).toContain('pasteboard_has_text()');
    expect(rust).toContain('if pasteboard_has_text()');
  });
});

// WebKit's native Copy (Edit menu / right-click) can clear the pasteboard and
// write nothing, and a site's own menu calls navigator.clipboard.writeText which
// WebKit rejects when the document is not focused. Both paths are covered here.
describe('copy event filled by the page', () => {
  const source = fs.readFileSync(
    path.join(process.cwd(), 'src-tauri/src/inject/copy_bridge.js'),
    'utf8',
  );
  const block = source.slice(
    source.indexOf('// Path 1'),
    source.indexOf('// Cmd+C safety net'),
  );

  it('only fills clipboard data the webview left empty', () => {
    expect(block).toContain('event.clipboardData.getData("text/plain")');
    expect(block).toContain('if (event.clipboardData.getData("text/plain")) return;');
  });

  it('never touches a copy that already carried data', () => {
    // Rich text and file copies must keep their own payload; the handler only
    // writes when the pasteboard came through empty.
    expect(block).toContain('event.clipboardData.setData("text/plain", text)');
    expect(block).not.toContain('clearContents');
  });

  it('always writes through the host, because a silent no-op cannot be caught', () => {
    // WebKit can resolve writeText without writing when the document is not
    // focused; relying on a rejection left the pasteboard empty.
    expect(source).toContain('invoke("copy_selection"');
    expect(source).toContain('const viaHost = hostWrite(value);');
    expect(source).toContain('if (!value.trim()) return original(value);');
  });

  it('repairs an execCommand copy that wrote nothing', () => {
    // Zalo copies messages with document.execCommand("copy"), which WebKit can
    // report as successful while leaving the pasteboard empty.
    expect(source).toContain('originalExec(command, ...rest)');
    expect(source).toContain('requestWrite(text)');
    expect(source).toContain('String(command).toLowerCase() !== "copy"');
  });

  it('notes why it must run in every frame', () => {
    const windowSource = fs.readFileSync(
      path.join(process.cwd(), 'src-tauri/src/app/window.rs'),
      'utf8',
    );
    // A selection can live in an iframe, where the top document's
    // getSelection() is empty; a main-frame-only bridge cannot see it.
    expect(windowSource).toContain(
      '.initialization_script_for_all_frames(include_str!("../inject/copy_bridge.js"))',
    );
  });
});
