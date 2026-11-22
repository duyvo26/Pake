//! Dedicated account manager: a small local page served over the
//! `pakeaccounts://` custom protocol in its own window.
//!
//! The page ships inside the binary (`include_bytes!`), so CLI builds need no
//! extra asset step, and its window label `pake-accounts` matches the
//! `pake-*` glob in `capabilities/default.json`, so the page may call the
//! account commands below. Custom-protocol URLs count as local for IPC, so no
//! remote grant is involved.

use crate::app::config::AccountConfig;
use crate::app::window::MultiWindowState;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::http::{Request, Response};
use tauri::{AppHandle, Manager, Url, WebviewWindow, Wry};

pub const MANAGER_WINDOW_LABEL: &str = "pake-accounts";
pub const SETTINGS_WINDOW_LABEL: &str = "pake-settings";
pub const MANAGER_PROTOCOL: &str = "pakeaccounts";
/// Fallback reveal delay, mirroring the multi-window clones in `window.rs`;
/// normally the global `on_page_load` reveal fires first for `pake-*` labels.
const REVEAL_FALLBACK_MS: u64 = 1_000;

static NEXT_ACCOUNT_ID: AtomicU64 = AtomicU64::new(1);

/// Serve the embedded local pages for the `pakeaccounts://` scheme. The path
/// picks which page: `/settings.html` for Settings, anything else for the
/// account list. One scheme keeps the capability grant to a single host.
pub fn serve(request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let path = request.uri().path();
    let body: Vec<u8> = if path.contains("settings") {
        include_bytes!("settings.html").to_vec()
    } else {
        include_bytes!("account_manager.html").to_vec()
    };
    match Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(body)
    {
        Ok(response) => response,
        Err(_) => {
            // Static headers and body cannot fail; this arm is unreachable but
            // must still produce a valid response without panicking.
            let mut response = Response::new(Vec::new());
            *response.status_mut() = tauri::http::StatusCode::INTERNAL_SERVER_ERROR;
            response
        }
    }
}

/// Open (or focus) the Settings window.
#[cfg(target_os = "macos")]
pub fn open_settings(app: &AppHandle<Wry>) -> tauri::Result<WebviewWindow> {
    if let Some(existing) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        let _ = existing.unminimize();
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(existing);
    }

    let url =
        Url::parse(&format!("{MANAGER_PROTOCOL}://localhost/settings.html")).map_err(|error| {
            tauri::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid settings URL: {error}"),
            ))
        })?;
    let window = crate::app::window::open_internal_page_window(app, SETTINGS_WINDOW_LABEL, url)?;
    let _ = window.set_title("Settings");
    let _ = window.set_resizable(false);

    let fallback = window.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(REVEAL_FALLBACK_MS)).await;
        crate::app::window::reveal_built_window(&fallback);
    });
    let _ = window.set_focus();
    Ok(window)
}

/// Open (or focus) the manager window. Built hidden through the shared
/// `build_window` path like every other Pake window, so it picks up the same
/// per-webview installs (history gestures, script dialogs, downloads); the
/// global `on_page_load` reveal shows it once the local page finishes loading.
#[cfg(target_os = "macos")]
pub fn open_manager(app: &AppHandle<Wry>) -> tauri::Result<WebviewWindow> {
    if let Some(existing) = app.get_webview_window(MANAGER_WINDOW_LABEL) {
        let _ = existing.unminimize();
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(existing);
    }

    let url =
        Url::parse(&format!("{MANAGER_PROTOCOL}://localhost/accounts.html")).map_err(|error| {
            tauri::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid manager URL: {error}"),
            ))
        })?;
    let window = crate::app::window::open_internal_page_window(app, MANAGER_WINDOW_LABEL, url)?;
    let _ = window.set_title("Accounts");

    let fallback = window.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(REVEAL_FALLBACK_MS)).await;
        crate::app::window::reveal_built_window(&fallback);
    });

    let _ = window.set_focus();
    Ok(window)
}

/// The shape `list_accounts` returns to the manager page.
#[derive(serde::Serialize)]
pub struct AccountList {
    pub accounts: Vec<AccountConfig>,
    pub active: Option<String>,
}

fn state_snapshot(app: &AppHandle<Wry>) -> Result<(Vec<AccountConfig>, Option<String>), String> {
    let state = app
        .try_state::<MultiWindowState>()
        .ok_or("Missing app state")?;
    Ok((state.accounts_snapshot(), state.active_account()))
}

/// Persist the current account list and refresh the macOS Accounts menu.
fn persist_accounts(app: &AppHandle<Wry>, snapshot: &[AccountConfig]) -> Result<(), String> {
    let state = app
        .try_state::<MultiWindowState>()
        .ok_or("Missing app state")?;
    if let Ok(path) = state.accounts_path.lock() {
        if let Some(path) = path.as_ref() {
            crate::util::write_accounts(path, snapshot).map_err(|error| error.to_string())?;
        }
    }
    let app_handle = app.clone();
    let enable_find = state
        .pake_config
        .windows
        .first()
        .map(|window| window.enable_find)
        .unwrap_or(false);
    let allow_multi_window = state.pake_config.multi_window;
    // Menu construction is main-thread work; commands run on worker threads.
    #[cfg(target_os = "macos")]
    let _ = app.run_on_main_thread(move || {
        let _ = crate::app::menu::set_app_menu(&app_handle, allow_multi_window, enable_find);
    });
    #[cfg(not(target_os = "macos"))]
    let _ = (app_handle, enable_find, allow_multi_window);
    Ok(())
}

/// Accounts must point at real web origins: the manager navigates the main
/// window there, so anything but a literal http(s) URL is rejected.
fn validate_account_url(raw: &str) -> Result<String, String> {
    let url = Url::parse(raw.trim()).map_err(|error| format!("Invalid URL: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("URL must start with http:// or https://".into());
    }
    if url.host_str().is_none() {
        return Err("URL must include a host".into());
    }
    Ok(url.to_string())
}

fn new_account_id() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis())
        .unwrap_or(0);
    let sequence = NEXT_ACCOUNT_ID.fetch_add(1, Ordering::Relaxed);
    format!("acc_{millis}_{sequence}")
}

#[tauri::command]
pub fn list_accounts(app: AppHandle<Wry>) -> Result<AccountList, String> {
    let (accounts, active) = state_snapshot(&app)?;
    Ok(AccountList { accounts, active })
}

#[tauri::command]
pub fn add_account(
    app: AppHandle<Wry>,
    label: String,
    url: String,
) -> Result<AccountConfig, String> {
    let label = label.trim().to_string();
    if label.is_empty() {
        return Err("Account name cannot be empty".into());
    }
    let url = validate_account_url(&url)?;
    let id = new_account_id();
    // A new account gets its own store so its first sign-in cannot land in a
    // jar another account already uses.
    let store_id =
        crate::app::config::format_uuid(&crate::app::window::data_store_id_for_account(&id));
    let account = AccountConfig {
        id,
        label,
        url,
        incognito: false,
        store_id,
    };
    let state = app
        .try_state::<MultiWindowState>()
        .ok_or("Missing app state")?;
    let snapshot = {
        let mut accounts = state
            .accounts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        accounts.push(account.clone());
        accounts.clone()
    };
    persist_accounts(&app, &snapshot)?;
    Ok(account)
}

#[tauri::command]
pub fn update_account(
    app: AppHandle<Wry>,
    id: String,
    label: Option<String>,
    url: Option<String>,
) -> Result<(), String> {
    let label = label.map(|value| value.trim().to_string());
    if let Some(label) = &label {
        if label.is_empty() {
            return Err("Account name cannot be empty".into());
        }
    }
    let url = match &url {
        Some(raw) if !raw.trim().is_empty() => Some(validate_account_url(raw)?),
        Some(_) => None,
        None => None,
    };
    let state = app
        .try_state::<MultiWindowState>()
        .ok_or("Missing app state")?;
    let snapshot = {
        let mut accounts = state
            .accounts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let account = accounts
            .iter_mut()
            .find(|account| account.id == id)
            .ok_or("Account not found")?;
        if let Some(label) = &label {
            account.label = label.clone();
        }
        if let Some(url) = &url {
            account.url = url.clone();
        }
        accounts.clone()
    };
    persist_accounts(&app, &snapshot)
}

#[tauri::command]
pub fn remove_account(app: AppHandle<Wry>, id: String) -> Result<(), String> {
    let state = app
        .try_state::<MultiWindowState>()
        .ok_or("Missing app state")?;
    let (snapshot, removed_store) = {
        let mut accounts = state
            .accounts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(removed) = accounts.iter().find(|account| account.id == id).cloned() else {
            return Err("Account not found".into());
        };
        accounts.retain(|account| account.id != id);
        (accounts.clone(), removed.data_store_id())
    };
    if state.active_account().as_deref() == Some(id.as_str()) {
        if let Ok(mut active) = state.active_account_id.lock() {
            *active = None;
        }
        if let Ok(path) = state.active_account_path.lock() {
            if let Some(path) = path.as_ref() {
                let _ = std::fs::remove_file(path);
            }
        }
    }
    // Drop the account's cookie jar, but never the store the running webview is
    // using: deleting that would sign the app out of the session it is showing.
    #[cfg(target_os = "macos")]
    {
        let store = removed_store;
        let live = state.session_store_id();
        if live != Some(store) {
            let cleanup_app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = cleanup_app.remove_data_store(store).await {
                    eprintln!("[Pake] Could not remove the account data store: {error}");
                }
            });
        }
    }
    persist_accounts(&app, &snapshot)
}

/// Switch to a saved account and mark it active.
///
/// On macOS this rebuilds the home webview with the account's own data store,
/// because a webview's cookies are fixed when it is created; navigating alone
/// would keep the previous login. The manager window closes itself afterwards.
#[tauri::command]
pub fn switch_account_to(
    app: AppHandle<Wry>,
    window: WebviewWindow,
    id: String,
) -> Result<(), String> {
    let (accounts, _) = state_snapshot(&app)?;
    let account = accounts
        .iter()
        .find(|account| account.id == id)
        .ok_or("Account not found")?;
    let url = Url::parse(&account.url).map_err(|error| format!("Invalid saved URL: {error}"))?;
    let account_id = account.id.clone();

    // Record the new active account before rebuilding: the replacement webview
    // reads it to pick its data store.
    let state = app
        .try_state::<MultiWindowState>()
        .ok_or("Missing app state")?;
    if let Ok(mut active) = state.active_account_id.lock() {
        *active = Some(account_id.clone());
    }
    if let Ok(path) = state.active_account_path.lock() {
        if let Some(path) = path.as_ref() {
            let _ = crate::util::write_active_account(path, &account_id);
        }
    }
    persist_accounts(&app, &accounts)?;

    // A cookie jar belongs to the webview that owns it. Navigating the existing
    // webview kept the previous login (a user's report, 2026-11-22), so rebuild
    // the home webview with the store saved for this account.
    #[cfg(target_os = "macos")]
    crate::app::window::reopen_home_window_for_account(&app, url, Some(account.data_store_id()))
        .map_err(|error| format!("Failed to switch account: {error}"))?;

    #[cfg(not(target_os = "macos"))]
    {
        let main = app
            .get_webview_window("pake")
            .ok_or("Main window not found")?;
        main.navigate(url).map_err(|error| error.to_string())?;
    }

    if window.label() == MANAGER_WINDOW_LABEL {
        let _ = window.close();
    }
    Ok(())
}

/// What the Settings page shows on open.
#[derive(serde::Serialize)]
pub struct SettingsState {
    #[serde(rename = "downloadDir")]
    pub download_dir: String,
    #[serde(rename = "fromPicker")]
    pub from_picker: bool,
    #[serde(rename = "appName")]
    pub app_name: String,
    pub version: String,
}

#[tauri::command]
pub fn settings_state(app: AppHandle<Wry>) -> Result<SettingsState, String> {
    let state = app
        .try_state::<MultiWindowState>()
        .ok_or("Missing app state")?;
    let from_picker = state
        .download_dir_override
        .lock()
        .map(|slot| slot.is_some())
        .unwrap_or(false);
    let download_dir = crate::util::get_download_dir(&app)?;
    let package = app.package_info();
    Ok(SettingsState {
        download_dir: download_dir.display().to_string(),
        from_picker,
        app_name: package.name.clone(),
        version: package.version.to_string(),
    })
}

/// Ask for a new download folder and remember it. Returns the chosen path, or
/// `None` when the user cancels.
///
/// The picker is main-thread only, and Tauri may run this command on either
/// thread, so `pick_folder_blocking` detects where it is and dispatches only
/// when it has to.
#[tauri::command]
pub fn choose_download_folder(app: AppHandle<Wry>) -> Result<Option<String>, String> {
    let state = app
        .try_state::<MultiWindowState>()
        .ok_or("Missing app state")?;
    let current = crate::util::get_download_dir(&app).ok();
    let Some(directory) = crate::app::download_folder::pick_folder_blocking(&app, current) else {
        return Ok(None);
    };

    if let Ok(mut slot) = state.download_dir_override.lock() {
        *slot = Some(directory.clone());
    }
    if let Ok(data_dir) = app.path().app_data_dir() {
        let path = crate::util::download_dir_file_path(&data_dir);
        if let Err(error) = crate::util::write_download_dir(&path, &directory) {
            eprintln!("[Pake] Could not remember the download folder: {error}");
        }
    }
    Ok(Some(directory.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::{new_account_id, validate_account_url};

    #[test]
    fn account_urls_must_be_web_origins() {
        assert_eq!(
            validate_account_url(" https://chat.zalo.me/ ").unwrap(),
            "https://chat.zalo.me/"
        );
        assert_eq!(
            validate_account_url("http://example.com/login?next=/home").unwrap(),
            "http://example.com/login?next=/home"
        );
        for bad in [
            "",
            "   ",
            "chat.zalo.me",
            "ftp://example.com",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "about:blank",
            "data:text/html,hi",
            "https://",
        ] {
            assert!(validate_account_url(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn account_ids_never_repeat_within_a_process() {
        let first = new_account_id();
        let second = new_account_id();
        assert_ne!(first, second);
        assert!(first.starts_with("acc_"));
    }
}
