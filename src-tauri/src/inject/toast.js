// Lightweight in-page toast used by Rust `show_toast` (download status, etc).
// Kept tiny and always loaded so the Rust side can rely on `window.pakeToast`.
(function () {
  // `action` names a Pake command a click on the toast runs (showing a finished
  // download in the file manager); such a toast stays twice as long.
  function show(msg, action) {
    if (!document.body) {
      document.addEventListener("DOMContentLoaded", function () {
        show(msg, action);
      });
      return;
    }
    const m = document.createElement("div");
    m.innerHTML = msg;
    m.style.cssText =
      "max-width:60%;min-width: 80px;padding:0 12px;height: 32px;color: rgb(255, 255, 255);line-height: 32px;text-align: center;border-radius: 8px;position: fixed; bottom:24px;right: 28px;z-index: 2147483647;background: rgba(0, 0, 0,.8);font-size: 13px;pointer-events:auto;";
    if (action) {
      m.style.cursor = "pointer";
      m.addEventListener("click", function () {
        const invoke = window.__TAURI__?.core?.invoke;
        if (invoke) invoke(action).catch(function () {});
      });
    }
    document.body.appendChild(m);
    setTimeout(
      function () {
        const d = 0.5;
        m.style.transition =
          "transform " + d + "s ease-in, opacity " + d + "s ease-in";
        m.style.opacity = "0";
        setTimeout(function () {
          if (m.parentNode) m.parentNode.removeChild(m);
        }, d * 1000);
      },
      action ? 6000 : 3000,
    );
  }

  window.pakeToast = show;
})();
