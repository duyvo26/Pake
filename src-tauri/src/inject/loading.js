// A loading overlay drawn before the page has anything to show.
//
// Rebuilding a window (account switch) or a slow first paint leaves an empty
// white frame on screen; a user reported exactly that (2026-11-23). This script
// runs at document start, so the overlay is on screen before the first pixel of
// page content, and it fades out once the document is ready.
//
// Everything is created through the DOM and animated with the Web Animations
// API: no stylesheet, so a page's strict CSP cannot block it, and no inline
// markup, so a page's own styles cannot target it.
(function () {
  if (window.__pakeLoadingInstalled) return;
  window.__pakeLoadingInstalled = true;

  var OVERLAY_ID = "__pake_loading";
  // A page that loads in one tick should not flash a spinner for a few frames.
  var MIN_VISIBLE_MS = 350;
  var SLOW_HINT_MS = 12000;
  var startedAt = Date.now();
  var settled = false;
  var slowTimer;

  function prefersDark() {
    try {
      return window.matchMedia("(prefers-color-scheme: dark)").matches;
    } catch (_) {
      return false;
    }
  }

  function build() {
    if (document.getElementById(OVERLAY_ID)) return document.getElementById(OVERLAY_ID);

    var dark = prefersDark();
    var root = document.createElement("div");
    root.id = OVERLAY_ID;
    root.setAttribute("role", "status");
    root.setAttribute("aria-live", "polite");
    root.style.cssText = [
      "position:fixed",
      "inset:0",
      "display:flex",
      "flex-direction:column",
      "align-items:center",
      "justify-content:center",
      "gap:14px",
      "margin:0",
      "padding:0",
      "z-index:2147483647",
      "background:" + (dark ? "#1e1f20" : "#ffffff"),
      "font:13px/1.4 -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
      "color:" + (dark ? "#9d9da2" : "#6e6e73"),
      "-webkit-user-select:none",
      "user-select:none",
      "opacity:1",
      "transition:opacity .25s ease-out",
    ].join(";");

    var spinner = document.createElement("div");
    spinner.style.cssText = [
      "width:22px",
      "height:22px",
      "border-radius:50%",
      "border:2px solid " + (dark ? "rgba(255,255,255,.16)" : "rgba(0,0,0,.12)"),
      "border-top-color:" + (dark ? "#5c9aff" : "#3478f6"),
      "box-sizing:border-box",
    ].join(";");

    var label = document.createElement("div");
    label.textContent = "Loading…";

    root.appendChild(spinner);
    root.appendChild(label);

    // A CSS animation needs a stylesheet; the Web Animations API does not.
    try {
      spinner.animate(
        [{ transform: "rotate(0deg)" }, { transform: "rotate(360deg)" }],
        { duration: 750, iterations: Infinity, easing: "linear" },
      );
    } catch (_) {
      // Older WebKit without WAAPI: the spinner simply stays still.
    }

    if (document.documentElement) {
      document.documentElement.appendChild(root);
    }

    slowTimer = setTimeout(function () {
      if (!settled) label.textContent = "Still loading…";
    }, SLOW_HINT_MS);

    return root;
  }

  var root = build();

  function mount() {
    // Never resurrect a dismissed overlay: a single-page app rewrites <html>'s
    // children on route and theme changes, and re-attaching the full-screen
    // cover afterwards cleared the page's text selection, which made Cmd+C copy
    // nothing (a user's report, 2026-11-23: "copy works sometimes").
    if (settled) return;
    if (!root) root = build();
    if (root && !root.parentNode && document.documentElement) {
      document.documentElement.appendChild(root);
    }
  }

  // At document start the element exists but the body does not; keep the
  // overlay attached as the document is parsed. Re-attach it if a framework
  // replaces <html>'s children wholesale, which would orphan the overlay.
  var mountObserver = null;
  if (!root.parentNode) {
    document.addEventListener("readystatechange", mount);
    if (document.documentElement) {
      mountObserver = new MutationObserver(mount);
      mountObserver.observe(document.documentElement, { childList: true });
    }
  }

  function remove() {
    if (settled) return;
    settled = true;
    clearTimeout(slowTimer);
    if (mountObserver) {
      mountObserver.disconnect();
      mountObserver = null;
    }
    var el = document.getElementById(OVERLAY_ID) || root;
    if (!el) return;
    var wait = Math.max(0, MIN_VISIBLE_MS - (Date.now() - startedAt));
    setTimeout(function () {
      el.style.opacity = "0";
      setTimeout(function () {
        if (el.parentNode) el.parentNode.removeChild(el);
      }, 300);
    }, wait);
  }

  // A chat app is a single-page app: DOMContentLoaded and even `load` arrive
  // long before React paints anything, so removing the overlay then left a
  // white window for several seconds (a user's report, 2026-11-23: still white
  // for ~5s). Wait for the document to actually contain visible content.
  function hasVisibleContent() {
    var body = document.body;
    if (!body) return false;
    // Text anywhere is the cheapest honest signal that the app rendered.
    if (body.innerText && body.innerText.trim().length > 0) return true;
    // Canvas/video-only pages have no text but do have something to show.
    return !!body.querySelector("canvas, video, img, svg");
  }

  function onReady() {
    if (document.readyState === "loading") return;
    if (hasVisibleContent()) remove();
  }

  var contentPoll = setInterval(function () {
    if (hasVisibleContent()) {
      clearInterval(contentPoll);
      remove();
    }
  }, 100);

  // Nothing to show after a long wait means the page is stuck (offline,
  // blocked); drop the overlay so the user is not trapped behind it.
  setTimeout(function () {
    clearInterval(contentPoll);
    remove();
  }, 20000);

  document.addEventListener("DOMContentLoaded", onReady);
  document.addEventListener("readystatechange", onReady);
  window.addEventListener("load", onReady);
  if (document.readyState !== "loading") onReady();
})();
