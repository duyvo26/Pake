use serde::{Deserialize, Serialize};
use tauri::ipc::RuntimeCapability;
use tauri::utils::acl::capability::{Capability, CapabilityFile, CapabilityRemote};
use tauri::utils::acl::RemoteUrlPattern;
use tauri::Url;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowConfig {
    pub url: String,
    pub hide_title_bar: bool,
    /// macOS only, set by the runtime builder rather than a CLI flag: a native
    /// transparent title bar above the page, painted with the page's top color.
    #[serde(default)]
    pub transparent_title_bar: bool,
    #[serde(default)]
    pub hide_window_decorations: bool,
    pub fullscreen: bool,
    pub maximize: bool,
    pub width: f64,
    pub height: f64,
    pub resizable: bool,
    pub url_type: String,
    pub always_on_top: bool,
    pub dark_mode: bool,
    pub disabled_web_shortcuts: bool,
    pub activation_shortcut: String,
    pub hide_on_close: bool,
    pub incognito: bool,
    #[serde(default = "default_restore_last_url")]
    pub restore_last_url: bool,
    #[serde(default)]
    pub password_autosave: bool,
    pub title: Option<String>,
    pub enable_wasm: bool,
    #[serde(default)]
    pub new_window: bool,
    pub start_to_tray: bool,
    #[serde(default)]
    pub force_internal_navigation: bool,
    #[serde(default)]
    pub internal_url_regex: String,
    #[serde(default)]
    pub enable_find: bool,
    /// macOS only: Back and Forward buttons in the title bar, beside the
    /// traffic lights. Off unless the app asks for them.
    #[serde(default)]
    pub navigation_buttons: bool,
    #[serde(default = "default_zoom")]
    pub zoom: u32,
    #[serde(default)]
    pub min_width: f64,
    #[serde(default)]
    pub min_height: f64,
    #[serde(default)]
    pub ignore_certificate_errors: bool,
    /// Saved web accounts for the Switch Account menu. Empty by default.
    #[serde(default)]
    pub accounts: Vec<AccountConfig>,
}

fn default_zoom() -> u32 {
    100
}

fn default_restore_last_url() -> bool {
    true
}

#[cfg(any(windows, test))]
impl WindowConfig {
    pub fn password_autosave_enabled(&self) -> bool {
        self.password_autosave && !self.incognito
    }
}

impl WindowConfig {
    pub fn remembers_last_url(&self) -> bool {
        self.restore_last_url && self.url_type == "web" && !self.incognito
    }

    /// The overlay title bar (`hide_title_bar`) keeps precedence, so existing
    /// configs behave exactly as before.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub fn title_bar_follows_page(&self) -> bool {
        self.transparent_title_bar && !self.hide_title_bar
    }
}

/// A saved web account the user can switch back to from the menu.
///
/// `store_id` names the `WKWebsiteDataStore` this account's logins live in, as a
/// UUID string. It belongs to the account, not to the account id: an account
/// saved from a session that already had its own store keeps that store, so
/// reopening the account returns to the exact same cookies.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AccountConfig {
    pub id: String,
    pub label: String,
    pub url: String,
    #[serde(default)]
    pub incognito: bool,
    /// UUID of the account's data store. Empty on configs saved before stores
    /// existed; those fall back to a store derived from `id`.
    #[serde(default)]
    pub store_id: String,
}

impl AccountConfig {
    /// The UUID this account's webview must use. Malformed or missing values
    /// fall back to the id-derived store so an account always opens somewhere
    /// deterministic instead of sharing the default store.
    pub fn data_store_id(&self) -> [u8; 16] {
        parse_uuid(&self.store_id)
            .unwrap_or_else(|| crate::app::window::data_store_id_for_account(&self.id))
    }
}

/// Parse a canonical `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` UUID.
pub fn parse_uuid(value: &str) -> Option<[u8; 16]> {
    let hex: String = value.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = [0u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

/// Canonical lowercase UUID text for a 16-byte identifier.
pub fn format_uuid(bytes: &[u8; 16]) -> String {
    let hex: Vec<String> = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        hex[0..4].concat(),
        hex[4..6].concat(),
        hex[6..8].concat(),
        hex[8..10].concat(),
        hex[10..16].concat()
    )
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlatformSpecific<T> {
    pub macos: T,
    pub linux: T,
    pub windows: T,
}

impl<T> PlatformSpecific<T> {
    pub const fn get(&self) -> &T {
        #[cfg(target_os = "macos")]
        let platform = &self.macos;
        #[cfg(target_os = "linux")]
        let platform = &self.linux;
        #[cfg(target_os = "windows")]
        let platform = &self.windows;

        platform
    }
}

impl<T> PlatformSpecific<T>
where
    T: Copy,
{
    pub const fn copied(&self) -> T {
        *self.get()
    }
}

pub type UserAgent = PlatformSpecific<String>;
pub type FunctionON = PlatformSpecific<bool>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PakeConfig {
    pub windows: Vec<WindowConfig>,
    pub user_agent: UserAgent,
    pub system_tray: FunctionON,
    pub system_tray_path: String,
    pub proxy_url: String,
    #[serde(default)]
    pub download_dir: String,
    /// Prompt for HTTP Basic credentials at runtime on macOS. WKWebView does
    /// not provide its own 401 login dialog, while Windows and Linux WebViews
    /// handle this flow natively.
    #[serde(default)]
    pub basic_auth: bool,
    #[serde(default)]
    pub multi_instance: bool,
    #[serde(default)]
    pub multi_window: bool,
    /// Open at login. Read only from a runtime override: the app then keeps its own
    /// login item in step at every start (see `autostart.rs`).
    #[serde(default)]
    pub launch_at_login: bool,
    /// Built-in filter lists to block with: `ads`, `trackers`, `cookies`,
    /// `china`. Only builds with the `content-blocking` feature read it.
    #[serde(default)]
    pub content_blocking: Vec<String>,
    /// Replaces the compiled custom.js when a runtime override supplies one.
    #[serde(skip)]
    pub runtime_custom_js: Option<String>,
    /// Set when a runtime override supplied the app identity, so per-app
    /// directories follow the identifier rather than the product name.
    #[serde(skip)]
    pub runtime_app: bool,
}

impl PakeConfig {
    pub fn show_system_tray(&self) -> bool {
        self.system_tray.copied()
    }

    pub fn remote_capability(
        &self,
    ) -> Result<Option<RemoteCapability>, Box<dyn std::error::Error>> {
        let mut origins = Vec::new();
        for window in self
            .windows
            .iter()
            .filter(|window| window.url_type == "web")
        {
            let url = Url::parse(&window.url)?;
            let host = url.host_str().ok_or("Remote IPC requires a hostname")?;
            // URLPattern metacharacters in a host must never turn a configured
            // literal origin into a wildcard or regular expression grant.
            if !matches!(url.scheme(), "http" | "https")
                || host.contains(['*', '?', '(', ')', '{', '}', '\\'])
            {
                return Err("Remote IPC requires a literal HTTP or HTTPS origin".into());
            }
            // Colons inside an IPv6 literal are URLPattern name delimiters
            // unless escaped. Keep the actual port delimiter unescaped.
            let port = url
                .port()
                .map(|port| format!(":{port}"))
                .unwrap_or_default();
            let origin = format!("{}://{}{}/*", url.scheme(), host.replace(':', "\\:"), port);
            origin.parse::<RemoteUrlPattern>()?;
            if !origins.contains(&origin) {
                origins.push(origin);
            }
        }
        if origins.is_empty() {
            return Ok(None);
        }

        // Reuse the local permission set, including scopes, without keeping a
        // second list that can drift or widening grants after navigation.
        let mut capability: Capability =
            serde_json::from_str(include_str!("../../capabilities/default.json"))?;
        capability.identifier = "pake-configured-origins".into();
        capability.local = false;
        capability.remote = Some(CapabilityRemote { urls: origins });
        Ok(Some(RemoteCapability(capability)))
    }
}

pub struct RemoteCapability(Capability);

impl RuntimeCapability for RemoteCapability {
    fn build(self) -> CapabilityFile {
        CapabilityFile::Capability(self.0)
    }
}

#[cfg(test)]
mod account_store_tests {
    use super::{format_uuid, parse_uuid, AccountConfig};

    fn account(id: &str, store_id: &str) -> AccountConfig {
        AccountConfig {
            id: id.into(),
            label: id.into(),
            url: "https://chat.zalo.me/".into(),
            incognito: false,
            store_id: store_id.into(),
        }
    }

    #[test]
    fn uuids_round_trip() {
        let bytes = crate::app::window::data_store_id_for_account("acc_1");
        let text = format_uuid(&bytes);
        assert_eq!(text.len(), 36);
        assert_eq!(text.chars().filter(|c| *c == '-').count(), 4);
        assert_eq!(parse_uuid(&text), Some(bytes));
    }

    #[test]
    fn malformed_uuids_are_rejected() {
        for bad in [
            "",
            "not-a-uuid",
            "d2f94cc9-48db-48eb-b0cf",
            "d2f94cc9-48db-48eb-b0cf-1c479c6425f",
            "zzf94cc9-48db-48eb-b0cf-1c479c6425fb",
        ] {
            assert_eq!(parse_uuid(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn an_account_uses_its_saved_store() {
        let saved = crate::app::window::data_store_id_for_account("pinned");
        let pinned = account("acc_1", &format_uuid(&saved));
        assert_eq!(pinned.data_store_id(), saved);
    }

    #[test]
    fn accounts_without_a_saved_store_stay_distinct() {
        // Configs written before stores existed carry an empty store_id. Two
        // such accounts must still resolve to different jars, or switching
        // between them would show one account's cookies under the other.
        let first = account("acc_1", "").data_store_id();
        let second = account("acc_2", "").data_store_id();
        assert_ne!(first, second);
    }

    #[test]
    fn a_corrupt_store_falls_back_to_the_id_store() {
        let broken = account("acc_1", "garbage");
        assert_eq!(
            broken.data_store_id(),
            crate::app::window::data_store_id_for_account("acc_1")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tauri::utils::acl::{manifest::Manifest, resolved::Resolved, ExecutionContext};

    fn config_for(url: &str) -> PakeConfig {
        let mut config: PakeConfig = serde_json::from_str(include_str!("../../pake.json")).unwrap();
        config.windows[0].url = url.into();
        config.windows[0].url_type = "web".into();
        config
    }

    #[test]
    fn last_url_restoration_defaults_on_and_preserves_private_and_local_exclusions() {
        let mut window = config_for("https://example.com").windows.remove(0);
        let mut legacy = serde_json::to_value(&window).unwrap();
        legacy.as_object_mut().unwrap().remove("restore_last_url");
        let legacy: WindowConfig = serde_json::from_value(legacy.clone()).unwrap();
        assert!(legacy.restore_last_url);
        assert!(legacy.remembers_last_url());

        for enabled in [false, true] {
            for incognito in [false, true] {
                for url_type in ["web", "local"] {
                    window.restore_last_url = enabled;
                    window.incognito = incognito;
                    window.url_type = url_type.into();
                    assert_eq!(
                        window.remembers_last_url(),
                        enabled && !incognito && url_type == "web"
                    );
                }
            }
        }

        let mut disabled = serde_json::to_value(legacy).unwrap();
        disabled["restore_last_url"] = false.into();
        let disabled: WindowConfig = serde_json::from_value(disabled).unwrap();
        assert!(!disabled.remembers_last_url());
        assert!(!disabled.incognito);
    }

    #[test]
    fn password_autosave_is_opt_in_and_disabled_in_private_windows() {
        let mut window = config_for("https://example.com").windows.remove(0);
        for enabled in [false, true] {
            for incognito in [false, true] {
                window.password_autosave = enabled;
                window.incognito = incognito;
                assert_eq!(window.password_autosave_enabled(), enabled && !incognito);
            }
        }
        let mut legacy = serde_json::to_value(window).unwrap();
        legacy.as_object_mut().unwrap().remove("password_autosave");
        let legacy: WindowConfig = serde_json::from_value(legacy).unwrap();
        assert!(!legacy.password_autosave);
        assert!(!legacy.password_autosave_enabled());
    }

    #[test]
    fn navigation_buttons_are_opt_in() {
        let window = config_for("https://example.com").windows.remove(0);
        let mut window = serde_json::to_value(window).unwrap();
        window.as_object_mut().unwrap().remove("navigation_buttons");
        let legacy: WindowConfig = serde_json::from_value(window.clone()).unwrap();
        assert!(!legacy.navigation_buttons);

        window["navigation_buttons"] = true.into();
        let asked: WindowConfig = serde_json::from_value(window).unwrap();
        assert!(asked.navigation_buttons);
    }

    #[test]
    fn transparent_title_bar_is_opt_in_and_yields_to_the_overlay() {
        let mut window = config_for("https://example.com").windows.remove(0);
        let mut legacy = serde_json::to_value(&window).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("transparent_title_bar");
        let legacy: WindowConfig = serde_json::from_value(legacy).unwrap();
        assert!(!legacy.transparent_title_bar);
        assert!(!legacy.title_bar_follows_page());

        for transparent in [false, true] {
            for hidden in [false, true] {
                window.transparent_title_bar = transparent;
                window.hide_title_bar = hidden;
                assert_eq!(window.title_bar_follows_page(), transparent && !hidden);
            }
        }
    }

    fn resolve(config: &PakeConfig) -> Resolved {
        let manifest: BTreeMap<String, Manifest> = serde_json::from_str(include_str!(concat!(
            env!("OUT_DIR"),
            "/acl-manifests.json"
        )))
        .unwrap();
        let local: Capability =
            serde_json::from_str(include_str!("../../capabilities/default.json")).unwrap();
        let mut capabilities = BTreeMap::from([(local.identifier.clone(), local)]);
        if let Some(remote) = config.remote_capability().unwrap() {
            capabilities.insert(remote.0.identifier.clone(), remote.0);
        }
        Resolved::resolve(
            &manifest,
            capabilities,
            tauri::utils::platform::Target::current(),
        )
        .unwrap()
    }

    fn allows(acl: &Resolved, command: &str, label: &str, url: Option<&str>) -> bool {
        acl.allowed_commands.get(command).is_some_and(|grants| {
            grants.iter().any(|grant| {
                let context_matches = match (&grant.context, url) {
                    (ExecutionContext::Local, None) => true,
                    (ExecutionContext::Remote { url: pattern }, Some(url)) => {
                        pattern.test(&Url::parse(url).unwrap())
                    }
                    _ => false,
                };
                context_matches && grant.webviews.iter().any(|pattern| pattern.matches(label))
            })
        })
    }

    #[test]
    fn remote_ipc_requires_the_configured_scheme_host_and_port() {
        let acl = resolve(&config_for("https://example.com/app?next=https://evil.com"));
        assert!(acl.has_app_acl);
        for command in [
            "download_file",
            "send_notification",
            "close_notification",
            "set_zoom",
            "plugin:window|is_fullscreen",
        ] {
            for label in ["pake", "pake-1"] {
                assert!(allows(
                    &acl,
                    command,
                    label,
                    Some("https://example.com/other?q=1#x")
                ));
                for url in [
                    "https://evil.com/",
                    "https://sub.example.com/",
                    "https://example.com.evil.com/",
                    "http://example.com/",
                    "https://example.com:8443/",
                    "https://evil.com/?next=https://example.com/",
                ] {
                    assert!(
                        !allows(&acl, command, label, Some(url)),
                        "allowed {command} on {url}"
                    );
                }
            }
            assert!(!allows(
                &acl,
                command,
                "unrelated",
                Some("https://example.com/")
            ));
        }
    }

    #[test]
    fn local_pages_keep_permissions_without_any_remote_grant() {
        let mut config = config_for("index.html");
        config.windows[0].url_type = "local".into();
        assert!(config.remote_capability().unwrap().is_none());
        let acl = resolve(&config);
        assert!(acl.has_app_acl);
        assert!(allows(&acl, "download_file", "pake", None));
        assert!(!allows(
            &acl,
            "download_file",
            "pake",
            Some("https://example.com/")
        ));
    }

    #[test]
    fn remote_ipc_handles_loopback_ipv6_and_international_domains_literally() {
        for url in [
            "http://127.0.0.1:8123/app",
            "http://[::1]:8123/app",
            "https://例子.测试/app",
        ] {
            let acl = resolve(&config_for(url));
            assert!(
                allows(&acl, "download_file", "pake", Some(url)),
                "denied {url}"
            );
            assert!(!allows(
                &acl,
                "download_file",
                "pake",
                Some("https://evil.com/")
            ));
        }
    }

    #[test]
    fn remote_ipc_deduplicates_origins_and_ignores_local_entries() {
        let mut config = config_for("https://example.com/one");
        let mut second = config.windows[0].clone();
        second.url = "https://example.com/two".into();
        config.windows.push(second);
        let mut local = config.windows[0].clone();
        local.url = "index.html".into();
        local.url_type = "local".into();
        config.windows.push(local);
        let remote = config.remote_capability().unwrap().unwrap();
        assert_eq!(remote.0.remote.unwrap().urls, vec!["https://example.com/*"]);
        assert!(!remote.0.local);
    }

    #[test]
    fn remote_ipc_rejects_pattern_hosts_and_non_web_schemes() {
        for url in [
            "https://*.example.com/",
            "https://(evil).com/",
            "file:///tmp/index.html",
            "data:text/html,hello",
        ] {
            assert!(
                config_for(url).remote_capability().is_err(),
                "accepted {url}"
            );
        }
    }

    #[test]
    fn window_print_reaches_the_webview_only_from_trusted_contexts() {
        // On macOS Tauri replaces `window.print` with an invoke of
        // `plugin:webview|print`, so the page's own print button depends on it.
        let command = "plugin:webview|print";
        let mut local = config_for("index.html");
        local.windows[0].url_type = "local".into();
        let local = resolve(&local);
        let remote = resolve(&config_for("https://example.com/app"));
        for label in ["pake", "pake-1"] {
            assert!(allows(&local, command, label, None), "local {label}");
            assert!(allows(&remote, command, label, None), "bundled {label}");
            assert!(
                allows(&remote, command, label, Some("https://example.com/doc")),
                "configured origin {label}"
            );
            for url in [
                "https://evil.com/",
                "https://sub.example.com/",
                "http://example.com/",
            ] {
                assert!(!allows(&local, command, label, Some(url)), "{url}");
                assert!(!allows(&remote, command, label, Some(url)), "{url}");
            }
        }
        assert!(!allows(&remote, command, "unrelated", None));
        assert!(!allows(
            &remote,
            command,
            "unrelated",
            Some("https://example.com/")
        ));
    }

    #[test]
    fn every_registered_app_command_is_gated_and_remains_available_on_the_entry_origin() {
        let acl = resolve(&config_for("https://example.com/"));
        assert!(acl.has_app_acl);
        let source = include_str!("../lib.rs");
        let registered = source
            .split("tauri::generate_handler![")
            .nth(1)
            .unwrap()
            .split(']')
            .next()
            .unwrap();
        for command in registered
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            assert!(
                allows(&acl, command, "pake", None),
                "missing local permission for {command}"
            );
            assert!(
                allows(&acl, command, "pake", Some("https://example.com/")),
                "missing remote permission for {command}"
            );
            assert!(
                !allows(&acl, command, "pake", Some("https://evil.com/")),
                "ungated {command}"
            );
        }
        assert!(!allows(
            &acl,
            "unregistered_command",
            "pake",
            Some("https://example.com/")
        ));
    }
}
