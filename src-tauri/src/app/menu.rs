// Menu functionality is only used on macOS; the module is gated in app/mod.rs.
use crate::app::navigation::{history_step, reload_window};
use crate::app::window::{open_additional_window_safe, MultiWindowState};
use std::io::Write;
use std::process::{Command, Stdio};
use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, WebviewWindow, Wry};
use tauri_plugin_opener::OpenerExt;

pub fn set_app_menu(
    app: &AppHandle<Wry>,
    allow_multi_window: bool,
    enable_find: bool,
) -> tauri::Result<()> {
    let pake_version = env!("CARGO_PKG_VERSION");
    let pake_menu_item_title = format!("Built with Pake V{}", pake_version);

    let window_submenu = window_menu(app)?;

    let menu = Menu::with_items(
        app,
        &[
            &app_menu(app)?,
            &file_menu(app, allow_multi_window)?,
            &edit_menu(app, enable_find)?,
            &view_menu(app)?,
            &navigation_menu(app)?,
            &window_submenu,
            &help_menu(app, &pake_menu_item_title)?,
        ],
    )?;

    app.set_menu(menu)?;

    // AppKit injects Move & Resize, Fill, Center, Full Screen Tile, and
    // window-cycling once the submenu is registered as the windows menu.
    window_submenu.set_as_windows_menu_for_nsapp()?;

    Ok(())
}

fn app_menu(app: &AppHandle<Wry>) -> tauri::Result<Submenu<Wry>> {
    let app_menu = Submenu::new(app, "Pake", true)?;
    let about_metadata = AboutMetadata::default();
    app_menu.append(&PredefinedMenuItem::about(
        app,
        Some("Pake"),
        Some(about_metadata),
    )?)?;
    app_menu.append(&PredefinedMenuItem::separator(app)?)?;
    app_menu.append(&PredefinedMenuItem::services(app, None)?)?;
    app_menu.append(&PredefinedMenuItem::separator(app)?)?;
    app_menu.append(&PredefinedMenuItem::hide(app, None)?)?;
    app_menu.append(&PredefinedMenuItem::hide_others(app, None)?)?;
    app_menu.append(&PredefinedMenuItem::show_all(app, None)?)?;
    app_menu.append(&PredefinedMenuItem::separator(app)?)?;
    app_menu.append(&PredefinedMenuItem::quit(app, None)?)?;
    Ok(app_menu)
}

fn file_menu(app: &AppHandle<Wry>, allow_multi_window: bool) -> tauri::Result<Submenu<Wry>> {
    let file_menu = Submenu::new(app, "File", true)?;
    if allow_multi_window {
        file_menu.append(&MenuItem::with_id(
            app,
            "new_window",
            "New Window",
            true,
            Some("CmdOrCtrl+N"),
        )?)?;
        file_menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    file_menu.append(&PredefinedMenuItem::close_window(app, None)?)?;
    file_menu.append(&PredefinedMenuItem::separator(app)?)?;
    file_menu.append(&MenuItem::with_id(
        app,
        "clear_cache_restart",
        "Clear Cache & Restart",
        true,
        Some("CmdOrCtrl+Shift+Backspace"),
    )?)?;
    Ok(file_menu)
}

fn edit_menu(app: &AppHandle<Wry>, enable_find: bool) -> tauri::Result<Submenu<Wry>> {
    let edit_menu = Submenu::new(app, "Edit", true)?;
    edit_menu.append(&PredefinedMenuItem::undo(app, None)?)?;
    edit_menu.append(&PredefinedMenuItem::redo(app, None)?)?;
    edit_menu.append(&PredefinedMenuItem::separator(app)?)?;
    edit_menu.append(&PredefinedMenuItem::cut(app, None)?)?;
    edit_menu.append(&PredefinedMenuItem::copy(app, None)?)?;
    edit_menu.append(&PredefinedMenuItem::paste(app, None)?)?;
    edit_menu.append(&MenuItem::with_id(
        app,
        "paste_and_match_style",
        "Paste and Match Style",
        true,
        Some("CmdOrCtrl+Shift+Option+V"),
    )?)?;
    edit_menu.append(&PredefinedMenuItem::select_all(app, None)?)?;
    edit_menu.append(&PredefinedMenuItem::separator(app)?)?;
    if enable_find {
        edit_menu.append(&MenuItem::with_id(
            app,
            "find",
            "Find",
            true,
            Some("CmdOrCtrl+F"),
        )?)?;
        edit_menu.append(&MenuItem::with_id(
            app,
            "find_next",
            "Find Next",
            true,
            Some("CmdOrCtrl+G"),
        )?)?;
        edit_menu.append(&MenuItem::with_id(
            app,
            "find_previous",
            "Find Previous",
            true,
            Some("CmdOrCtrl+Shift+G"),
        )?)?;
        edit_menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    edit_menu.append(&MenuItem::with_id(
        app,
        "copy_url",
        "Copy URL",
        true,
        Some("CmdOrCtrl+L"),
    )?)?;
    Ok(edit_menu)
}

fn view_menu(app: &AppHandle<Wry>) -> tauri::Result<Submenu<Wry>> {
    let view_menu = Submenu::new(app, "View", true)?;
    view_menu.append(&MenuItem::with_id(
        app,
        "reload",
        "Reload",
        true,
        Some("CmdOrCtrl+R"),
    )?)?;
    view_menu.append(&PredefinedMenuItem::separator(app)?)?;
    view_menu.append(&MenuItem::with_id(
        app,
        "zoom_in",
        "Zoom In",
        true,
        Some("CmdOrCtrl+="),
    )?)?;
    view_menu.append(&MenuItem::with_id(
        app,
        "zoom_out",
        "Zoom Out",
        true,
        Some("CmdOrCtrl+-"),
    )?)?;
    view_menu.append(&MenuItem::with_id(
        app,
        "zoom_reset",
        "Actual Size",
        true,
        Some("CmdOrCtrl+0"),
    )?)?;
    view_menu.append(&PredefinedMenuItem::separator(app)?)?;
    view_menu.append(&PredefinedMenuItem::fullscreen(app, None)?)?;
    view_menu.append(&PredefinedMenuItem::separator(app)?)?;
    view_menu.append(&MenuItem::with_id(
        app,
        "toggle_devtools",
        "Toggle Developer Tools",
        cfg!(debug_assertions),
        Some("CmdOrCtrl+Option+I"),
    )?)?;
    Ok(view_menu)
}

fn navigation_menu(app: &AppHandle<Wry>) -> tauri::Result<Submenu<Wry>> {
    let navigation_menu = Submenu::new(app, "Navigation", true)?;
    navigation_menu.append(&MenuItem::with_id(
        app,
        "go_back",
        "Back",
        true,
        Some("CmdOrCtrl+["),
    )?)?;
    navigation_menu.append(&MenuItem::with_id(
        app,
        "go_forward",
        "Forward",
        true,
        Some("CmdOrCtrl+]"),
    )?)?;
    navigation_menu.append(&MenuItem::with_id(
        app,
        "go_home",
        "Go Home",
        true,
        Some("CmdOrCtrl+Shift+H"),
    )?)?;
    Ok(navigation_menu)
}

fn window_menu(app: &AppHandle<Wry>) -> tauri::Result<Submenu<Wry>> {
    let window_menu = Submenu::new(app, "Window", true)?;
    window_menu.append(&PredefinedMenuItem::minimize(app, None)?)?;
    window_menu.append(&PredefinedMenuItem::maximize(app, None)?)?;

    let accounts_menu = Submenu::new(app, "Accounts", true)?;
    let mut has_accounts = false;
    let mut has_active_account = false;
    if let Some(state) = app.try_state::<MultiWindowState>() {
        let accounts = state.accounts_snapshot();
        if !accounts.is_empty() {
            has_accounts = true;
            let active = state.active_account();
            has_active_account = active.is_some();
            for account in &accounts {
                let id = format!("switch_account:{}", account.id);
                let mut label = if account.label.trim().is_empty() {
                    account.id.clone()
                } else {
                    account.label.clone()
                };
                if active.as_ref() == Some(&account.id) {
                    label = format!("✓ {}", label);
                }
                accounts_menu.append(&MenuItem::with_id(app, &id, &label, true, None::<&str>)?)?;
            }
            accounts_menu.append(&PredefinedMenuItem::separator(app)?)?;
        }
    }
    accounts_menu.append(&MenuItem::with_id(
        app,
        "save_current_account",
        "Save Current Page as Account",
        true,
        None::<&str>,
    )?)?;
    if has_accounts && has_active_account {
        accounts_menu.append(&MenuItem::with_id(
            app,
            "rename_current_account",
            "Rename Current Account",
            true,
            None::<&str>,
        )?)?;
    }
    if has_accounts {
        accounts_menu.append(&MenuItem::with_id(
            app,
            "remove_current_account",
            "Remove Current Account",
            true,
            None::<&str>,
        )?)?;
    }
    accounts_menu.append(&MenuItem::with_id(
        app,
        "switch_account",
        "New Session (Clear Data)",
        true,
        None::<&str>,
    )?)?;
    window_menu.append(&accounts_menu)?;

    window_menu.append(&PredefinedMenuItem::separator(app)?)?;
    window_menu.append(&MenuItem::with_id(
        app,
        "always_on_top",
        "Toggle Always on Top",
        true,
        None::<&str>,
    )?)?;
    window_menu.append(&PredefinedMenuItem::separator(app)?)?;
    window_menu.append(&PredefinedMenuItem::close_window(app, None)?)?;
    Ok(window_menu)
}

fn help_menu(app: &AppHandle<Wry>, title: &str) -> tauri::Result<Submenu<Wry>> {
    let help_menu = Submenu::new(app, "Help", true)?;
    let github_item = MenuItem::with_id(app, "pake_github_link", title, true, None::<&str>)?;
    help_menu.append(&github_item)?;
    Ok(help_menu)
}

// Resolve the app's real home URL from its window config. Split out from
// `home_url` so the mapping can be unit-tested without an AppHandle.
fn resolve_home_url(url_type: &str, url: &str) -> Option<tauri::Url> {
    match url_type {
        // A web app's configured url is already absolute.
        "web" => tauri::Url::parse(url).ok(),
        // A local-file app's url is only a basename; Tauri serves bundled assets
        // from tauri://localhost on macOS (this menu is macOS-only). Resolving it
        // against the currently loaded remote origin (as the old eval path did)
        // would point at the wrong server.
        "local" => tauri::Url::parse(&format!("tauri://localhost/{url}")).ok(),
        _ => None,
    }
}

fn home_url(app: &AppHandle) -> Option<tauri::Url> {
    let state = app.try_state::<MultiWindowState>()?;
    let window_config = state.pake_config.windows.first()?;
    resolve_home_url(&window_config.url_type, &window_config.url)
}

fn focused_webview_window(app_handle: &AppHandle) -> Option<WebviewWindow> {
    let windows = app_handle.webview_windows();
    windows
        .values()
        .find(|window| window.is_focused().unwrap_or(false))
        .cloned()
        .or_else(|| windows.get("pake").cloned())
}

/// Copy text via pbcopy so it works when the page has no JS context (error
/// shells) and when the Clipboard API is blocked by the site CSP.
fn copy_text_to_pasteboard(text: &str) {
    let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() else {
        return;
    };
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(text.as_bytes());
    }
    let _ = child.wait();
}

/// Prompt for an account name with a native macOS dialog. Returns None if the
/// user cancels or the dialog cannot be shown.
fn prompt_account_name(default: &str) -> Option<String> {
    let escaped = default
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
        .replace('\r', " ");
    let script = format!(
        "text returned of (display dialog \"Account name:\" default answer \"{escaped}\" with title \"Pake\" buttons {{\"Cancel\",\"Save\"}} default button \"Save\")"
    );
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

pub fn handle_menu_click(app_handle: &AppHandle, id: &str) {
    match id {
        "new_window" => {
            open_additional_window_safe(app_handle);
        }
        "pake_github_link" => {
            let _ = app_handle
                .opener()
                .open_url("https://github.com/tw93/Pake", None::<&str>);
        }
        "reload" => {
            if let Some(window) = focused_webview_window(app_handle) {
                // Native reload works on blank error pages where eval cannot.
                reload_window(&window);
            }
        }
        "toggle_devtools" => {
            #[cfg(debug_assertions)] // Only allow in debug builds
            if let Some(window) = focused_webview_window(app_handle) {
                if window.is_devtools_open() {
                    window.close_devtools();
                } else {
                    window.open_devtools();
                }
            }
        }
        "zoom_in" => {
            if let Some(window) = focused_webview_window(app_handle) {
                let _ = window.eval("zoomIn()");
            }
        }
        "zoom_out" => {
            if let Some(window) = focused_webview_window(app_handle) {
                let _ = window.eval("zoomOut()");
            }
        }
        "zoom_reset" => {
            if let Some(window) = focused_webview_window(app_handle) {
                let _ = window.eval("setZoom('100%')");
            }
        }
        "go_back" => {
            if let Some(window) = focused_webview_window(app_handle) {
                history_step(&window, true);
            }
        }
        "go_forward" => {
            if let Some(window) = focused_webview_window(app_handle) {
                history_step(&window, false);
            }
        }
        "go_home" => {
            if let Some(window) = focused_webview_window(app_handle) {
                // Native navigation works even from a blank error page (where
                // eval cannot run) and resolves local-file apps to the correct
                // bundled asset instead of a path on the current origin.
                match home_url(app_handle) {
                    Some(url) => {
                        let _ = window.navigate(url);
                    }
                    None => {
                        let _ = window.eval("window.location.href = window.pakeConfig.url");
                    }
                }
            }
        }
        "copy_url" => {
            if let Some(window) = focused_webview_window(app_handle) {
                // Prefer the native webview URL so copy still works on error
                // pages that have no document.location / clipboard API.
                if let Ok(url) = window.url() {
                    copy_text_to_pasteboard(url.as_str());
                }
            }
        }
        "paste_and_match_style" => {
            if let Some(window) = focused_webview_window(app_handle) {
                let _ = window.eval("triggerPasteAsPlainText()");
            }
        }
        "find" => {
            if let Some(window) = focused_webview_window(app_handle) {
                let _ = window.eval("window.pakeFind?.open()");
            }
        }
        "find_next" => {
            if let Some(window) = focused_webview_window(app_handle) {
                let _ = window.eval("window.pakeFind?.next()");
            }
        }
        "find_previous" => {
            if let Some(window) = focused_webview_window(app_handle) {
                let _ = window.eval("window.pakeFind?.previous()");
            }
        }
        "clear_cache_restart" => {
            if let Some(window) = focused_webview_window(app_handle) {
                if window.clear_all_browsing_data().is_ok() {
                    app_handle.restart();
                }
            }
        }
        "always_on_top" => {
            if let Some(window) = focused_webview_window(app_handle) {
                let is_on_top = window.is_always_on_top().unwrap_or(false);
                let _ = window.set_always_on_top(!is_on_top);
            }
        }
        "switch_account" => {
            // New session: clear browsing data and return to home URL.
            if let Some(window) = focused_webview_window(app_handle) {
                crate::util::show_toast(&window, "Starting new session...");
                let _ = window.clear_all_browsing_data();
                if let Some(state) = app_handle.try_state::<MultiWindowState>() {
                    if let Ok(mut active) = state.active_account_id.lock() {
                        *active = None;
                    }
                    if let Ok(path) = state.active_account_path.lock() {
                        if let Some(path) = path.as_ref() {
                            let _ = std::fs::remove_file(path);
                        }
                    }
                    let _ = set_app_menu(
                        app_handle,
                        state.pake_config.multi_window,
                        state
                            .pake_config
                            .windows
                            .first()
                            .map(|w| w.enable_find)
                            .unwrap_or(false),
                    );
                }
                if let Some(url) = home_url(app_handle) {
                    let _ = window.navigate(url);
                }
            }
        }
        "save_current_account" => {
            if let Some(window) = focused_webview_window(app_handle) {
                if let Some(state) = app_handle.try_state::<MultiWindowState>() {
                    let current_url = window
                        .url()
                        .ok()
                        .map(|u| u.to_string())
                        .or_else(|| state.pake_config.windows.first().map(|w| w.url.clone()));
                    let Some(url) = current_url else { return };
                    let title = window.title().unwrap_or_default();
                    let suggested = if title.trim().is_empty() || title == "Pake" {
                        url.clone()
                    } else {
                        title
                    };
                    let Some(label) = prompt_account_name(&suggested) else {
                        return;
                    };
                    let id = format!(
                        "acc_{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(0)
                    );
                    let toast_label = label.clone();
                    let account = crate::app::config::AccountConfig {
                        id: id.clone(),
                        label,
                        url,
                        incognito: false,
                    };
                    let mut accounts = state.accounts.lock().unwrap_or_else(|e| e.into_inner());
                    accounts.push(account);
                    let snapshot = accounts.clone();
                    drop(accounts);
                    if let Ok(mut active) = state.active_account_id.lock() {
                        *active = Some(id.clone());
                    }
                    if let Ok(path) = state.accounts_path.lock() {
                        if let Some(path) = path.as_ref() {
                            let _ = crate::util::write_accounts(path, &snapshot);
                        }
                    }
                    if let Ok(path) = state.active_account_path.lock() {
                        if let Some(path) = path.as_ref() {
                            let _ = crate::util::write_active_account(path, &id);
                        }
                    }
                    // Rebuild menu to reflect new account list
                    let _ = set_app_menu(
                        app_handle,
                        state.pake_config.multi_window,
                        state
                            .pake_config
                            .windows
                            .first()
                            .map(|w| w.enable_find)
                            .unwrap_or(false),
                    );
                    crate::util::show_toast(&window, &format!("Saved: {}", toast_label));
                }
            }
        }
        "rename_current_account" => {
            if let Some(window) = focused_webview_window(app_handle) {
                if let Some(state) = app_handle.try_state::<MultiWindowState>() {
                    let active = state.active_account();
                    let Some(active_id) = active else { return };
                    let mut accounts = state.accounts.lock().unwrap_or_else(|e| e.into_inner());
                    let Some(account) = accounts.iter_mut().find(|a| a.id == active_id) else {
                        return;
                    };
                    let Some(new_label) = prompt_account_name(&account.label) else {
                        return;
                    };
                    account.label = new_label.clone();
                    let snapshot = accounts.clone();
                    drop(accounts);
                    if let Ok(path) = state.accounts_path.lock() {
                        if let Some(path) = path.as_ref() {
                            let _ = crate::util::write_accounts(path, &snapshot);
                        }
                    }
                    let _ = set_app_menu(
                        app_handle,
                        state.pake_config.multi_window,
                        state
                            .pake_config
                            .windows
                            .first()
                            .map(|w| w.enable_find)
                            .unwrap_or(false),
                    );
                    crate::util::show_toast(&window, &format!("Renamed: {}", new_label));
                }
            }
        }
        "remove_current_account" => {
            if let Some(window) = focused_webview_window(app_handle) {
                if let Some(state) = app_handle.try_state::<MultiWindowState>() {
                    let active = state.active_account();
                    if let Some(active_id) = active {
                        let mut accounts = state.accounts.lock().unwrap_or_else(|e| e.into_inner());
                        accounts.retain(|a| a.id != active_id);
                        let snapshot = accounts.clone();
                        drop(accounts);
                        if let Ok(mut active) = state.active_account_id.lock() {
                            *active = None;
                        }
                        if let Ok(path) = state.accounts_path.lock() {
                            if let Some(path) = path.as_ref() {
                                let _ = crate::util::write_accounts(path, &snapshot);
                            }
                        }
                        if let Ok(path) = state.active_account_path.lock() {
                            if let Some(path) = path.as_ref() {
                                let _ = std::fs::remove_file(path);
                            }
                        }
                        let _ = set_app_menu(
                            app_handle,
                            state.pake_config.multi_window,
                            state
                                .pake_config
                                .windows
                                .first()
                                .map(|w| w.enable_find)
                                .unwrap_or(false),
                        );
                        crate::util::show_toast(&window, "Account removed");
                    }
                }
            }
        }
        _ => {
            if let Some(account_id) = id.strip_prefix("switch_account:") {
                if let Some(window) = focused_webview_window(app_handle) {
                    if let Some(state) = app_handle.try_state::<MultiWindowState>() {
                        let accounts = state.accounts_snapshot();
                        if let Some(account) = accounts.iter().find(|a| a.id == account_id) {
                            if let Ok(url) = tauri::Url::parse(&account.url) {
                                let _ = window.navigate(url);
                                if let Ok(mut active) = state.active_account_id.lock() {
                                    *active = Some(account.id.clone());
                                }
                                if let Ok(path) = state.active_account_path.lock() {
                                    if let Some(path) = path.as_ref() {
                                        let _ =
                                            crate::util::write_active_account(path, &account.id);
                                    }
                                }
                                let _ = set_app_menu(
                                    app_handle,
                                    state.pake_config.multi_window,
                                    state
                                        .pake_config
                                        .windows
                                        .first()
                                        .map(|w| w.enable_find)
                                        .unwrap_or(false),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_home_url;

    #[test]
    fn web_url_passes_through_unchanged() {
        assert_eq!(
            resolve_home_url("web", "https://github.com").map(|u| u.to_string()),
            Some("https://github.com/".to_string())
        );
    }

    #[test]
    fn local_basename_resolves_to_bundled_asset_url() {
        // The fix: a local app's basename must become the bundled asset URL,
        // not a path resolved against whatever origin is currently loaded.
        assert_eq!(
            resolve_home_url("local", "launcher.html").map(|u| u.to_string()),
            Some("tauri://localhost/launcher.html".to_string())
        );
    }

    #[test]
    fn unknown_url_type_is_none() {
        assert!(resolve_home_url("bogus", "whatever").is_none());
    }
}
