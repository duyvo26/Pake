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
    if (!root) root = build();
    if (root && !root.parentNode && document.documentElement) {
      document.documentElement.appendChild(root);
    }
  }

  // At document start the element exists but the body does not; keep the
  // overlay attached as the document is parsed.
  if (!root.parentNode) {
    document.addEventListener("readystatechange", mount);
    if (document.documentElement) {
      new MutationObserver(function () {
        mount();
        if (document.body) remove();
      }).observe(document.documentElement, { childList: true });
    }
  }

  function remove() {
    if (settled) return;
    settled = true;
    clearTimeout(slowTimer);
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

  // Hide when the document settles. `load` waits for images; a chat app keeps
  // sockets open forever, so DOM readiness is the signal worth using first.
  function onReady() {
    if (document.readyState === "loading") return;
    remove();
  }

  document.addEventListener("DOMContentLoaded", onReady);
  document.addEventListener("readystatechange", onReady);
  window.addEventListener("load", remove);
  window.addEventListener("pageshow", remove);
  // If the document never becomes ready (offline, stalled), the overlay stays
  // with the "Still loading…" hint rather than leaving a white frame.
  if (document.readyState !== "loading") remove();
})();
