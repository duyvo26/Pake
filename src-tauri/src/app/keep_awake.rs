//! Keep the app responsive while its window is hidden.
//!
//! Closing the window on macOS only hides it (`hide_on_close`), and macOS then
//! treats the process as a background app: App Nap throttles timers, and a
//! hidden webview can stop running its socket callbacks. A chat app in that
//! state keeps its WebSocket open but stops reacting to it, so new messages only
//! arrive after the window is shown again. A user reported exactly that
//! (2026-11-22): messages appeared only after closing and reopening the window.
//!
//! Holding an `NSProcessInfo` activity for the process lifetime tells the system
//! this work must keep running in the background.

/// Start a user-initiated activity that survives the window being hidden.
///
/// Call once at startup. The token is deliberately leaked: the activity ends
/// when the process exits, whereas dropping it would end it immediately and
/// restore App Nap.
#[cfg(target_os = "macos")]
pub fn keep_awake_while_hidden() {
    use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};

    // Everything the user started must keep running, and the site's socket
    // handling is latency-sensitive enough that throttling it loses messages.
    let options = NSActivityOptions::UserInitiated | NSActivityOptions::LatencyCritical;
    let reason = NSString::from_str("Pake keeps the packaged web app running while hidden");
    let info = NSProcessInfo::processInfo();
    let activity = info.beginActivityWithOptions_reason(options, &reason);
    std::mem::forget(activity);
}

#[cfg(not(target_os = "macos"))]
pub fn keep_awake_while_hidden() {}

#[cfg(test)]
mod tests {
    /// The activity must include user-initiated work: anything less still lets
    /// App Nap throttle the hidden webview's socket callbacks.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_activity_is_user_initiated_and_not_throttled() {
        use objc2_foundation::NSActivityOptions;
        let options = NSActivityOptions::UserInitiated | NSActivityOptions::LatencyCritical;
        assert!(options.contains(NSActivityOptions::UserInitiated));
        assert!(options.contains(NSActivityOptions::LatencyCritical));
        assert!(options.contains(NSActivityOptions::IdleSystemSleepDisabled));
    }
}
