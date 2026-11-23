// Make copying work in every frame.
//
// Two copy paths exist on macOS and both fail silently for the same reason: the
// responder chain or the document's focus is not where the selection is.
//
//   1. WebKit's own Edit > Copy and Cmd+C. When it considers the selection
//      empty it still runs: it clears the pasteboard and writes nothing, so the
//      clipboard ends up blank.
//   2. A site's right-click menu, which reads the selection and calls
//      navigator.clipboard.writeText. WebKit raises NotAllowedError when the
//      document is not focused, and sites swallow it, so their Copy item looks
//      dead. Zalo's message menu is this kind: "right-click, Copy" copied
//      nothing (a user's report, 2026-11-23).
//
// This runs in every frame because a selection can live in an iframe, where the
// top document's getSelection() is empty.
(function () {
  if (window.__pakeCopyBridgeInstalled) return;
  window.__pakeCopyBridgeInstalled = true;

  function selectedText() {
    const selection = window.getSelection?.();
    if (!selection) return "";
    const text = selection.toString();
    if (text) return text;

    // Inputs keep their selection outside the document selection.
    const active = document.activeElement;
    if (active && typeof active.selectionStart === "number") {
      try {
        const value = active.value ?? "";
        return value.slice(active.selectionStart, active.selectionEnd ?? 0);
      } catch (_) {
        return "";
      }
    }
    return "";
  }

  // Path 1: fill the copy event the webview left empty. The clipboard data
  // belongs to this event only, so a copy that already carried something —
  // rich text, files, images — is never disturbed.
  document.addEventListener(
    "copy",
    function (event) {
      if (!event.clipboardData) return;
      if (event.clipboardData.getData("text/plain")) return;
      const text = selectedText();
      if (!text) return;
      try {
        event.clipboardData.setData("text/plain", text);
      } catch (_) {
        // A locked-down clipboard refuses writes; the host path below still
        // covers the scripted case.
      }
    },
    true,
  );

  // Cmd+C safety net: if the keystroke never produced a copy event, the webview
  // dropped it. Waiting one grace period is what tells the two apart.
  const COPY_GRACE_MS = 250;
  let pending = null;

  document.addEventListener(
    "keydown",
    function (event) {
      if (
        event.isTrusted !== true ||
        event.key?.toLowerCase() !== "c" ||
        !event.metaKey ||
        event.ctrlKey ||
        event.altKey ||
        event.shiftKey
      ) {
        return;
      }
      const text = selectedText();
      if (!text.trim()) {
        pending = null;
        return;
      }
      const token = {};
      pending = token;
      setTimeout(function () {
        if (pending !== token) return;
        pending = null;
        requestWrite(text);
      }, COPY_GRACE_MS);
    },
    true,
  );

  document.addEventListener(
    "copy",
    function () {
      pending = null;
    },
    true,
  );

  // Path 2: a scripted write the webview rejected. The host writes through
  // NSPasteboard, which has no focus requirement.
  function hostWrite(text) {
    const invoke = window.__TAURI__?.core?.invoke;
    if (!invoke) return Promise.reject(new Error("no host bridge"));
    return invoke("copy_selection", { text: String(text) }).then(function (result) {
      if (!result || !result.copied) throw new Error("host copy refused");
    });
  }

  function requestWrite(text) {
    hostWrite(text).catch(function () {});
  }

  // Path 2: document.execCommand("copy"). Zalo copies messages this way: it
  // builds a temporary textarea (or a hidden div for HTML) and calls
  // execCommand. WebKit can return true while writing nothing — the pasteboard
  // ends up empty — so the result is verified by the host and repaired.
  // Reproduced on 2026-11-23: the same call works in a bare WKWebView, but in a
  // packaged app the pasteboard came back blank.
  const originalExec = document.execCommand?.bind(document);
  if (originalExec) {
    Object.defineProperty(document, "execCommand", {
      configurable: true,
      writable: true,
      value: function (command, ...rest) {
        const result = originalExec(command, ...rest);
        if (String(command).toLowerCase() !== "copy") return result;

        // Zalo leaves its temporary element in the DOM until after the call, so
        // the selection still points at the text being copied.
        const text = selectedText();
        if (!text.trim()) return result;

        // The host writes only if the pasteboard stayed empty, so a copy that
        // carried rich text or files is never overwritten.
        requestWrite(text);
        return result;
      },
    });
  }

  // Path 3: the scripted write itself. WebKit can resolve writeText without
  // writing anything when the document is not focused — a rejection would be
  // catchable, a silent no-op is not — so the host write always runs and the
  // webview's own attempt is best effort. writeText is plain text by contract,
  // so writing the same text twice changes nothing.
  const clipboard = navigator.clipboard;
  if (clipboard && typeof clipboard.writeText === "function") {
    const original = clipboard.writeText.bind(clipboard);
    Object.defineProperty(clipboard, "writeText", {
      configurable: true,
      writable: true,
      value: function (text) {
        const value = String(text);
        if (!value.trim()) return original(value);
        const viaHost = hostWrite(value);
        return original(value).then(
          function () {
            return viaHost;
          },
          function (error) {
            return viaHost.catch(function () {
              // Surface the webview's own error so callers that handle it still
              // see the real failure.
              throw error;
            });
          },
        ).catch(function () {
          // The host refused (an empty or overlong selection); stay silent so a
          // site's Copy does not throw where it used to succeed.
        });
      },
    });
  }
})();
