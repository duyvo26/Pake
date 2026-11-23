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
//!
//! The options matter as much as the activity. This first used
//! `UserInitiated | LatencyCritical`, which also disables idle sleep and turns
//! off timer coalescing for the whole process: every subsystem stopped batching
//! its wake-ups, and the machine felt sluggish (a user's report, 2026-11-23).
//! `UserInitiatedAllowingIdleSystemSleep` alone is what this needs — it fights
//! App Nap without keeping the Mac awake or de-coalescing timers.

/// Start a user-initiated activity that survives the window being hidden.
///
/// Call once at startup. The token is deliberately leaked: the activity ends
/// when the process exits, whereas dropping it would end it immediately and
/// restore App Nap.
#[cfg(target_os = "macos")]
pub fn keep_awake_while_hidden() {
    use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};

    // Enough to keep App Nap from suspending the page's socket callbacks, and
    // no more: the Mac may still sleep, and timers keep coalescing.
    let options = NSActivityOptions::UserInitiatedAllowingIdleSystemSleep;
    let reason = NSString::from_str("Pake keeps the packaged web app running while hidden");
    let info = NSProcessInfo::processInfo();
    let activity = info.beginActivityWithOptions_reason(options, &reason);
    std::mem::forget(activity);
}

#[cfg(not(target_os = "macos"))]
pub fn keep_awake_while_hidden() {}

#[cfg(test)]
mod tests {
    /// The activity must count as user-initiated work, or App Nap still
    /// throttles the hidden webview's socket callbacks.
    ///
    /// `UserInitiatedAllowingIdleSystemSleep` is `UserInitiated` minus the idle
    /// sleep bit, so `contains(UserInitiated)` is false by construction; compare
    /// what actually matters, everything user-initiated except that bit.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_activity_is_user_initiated() {
        use objc2_foundation::NSActivityOptions;
        let options = NSActivityOptions::UserInitiatedAllowingIdleSystemSleep;
        let user_initiated_without_sleep = NSActivityOptions(
            NSActivityOptions::UserInitiated.0 & !NSActivityOptions::IdleSystemSleepDisabled.0,
        );
        assert_eq!(options, user_initiated_without_sleep);
    }

    /// The activity must NOT disable idle sleep: keeping the Mac awake for a
    /// background chat window is not what the user asked for, and the first
    /// version did exactly that.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_activity_lets_the_mac_sleep() {
        use objc2_foundation::NSActivityOptions;
        let options = NSActivityOptions::UserInitiatedAllowingIdleSystemSleep;
        assert!(!options.contains(NSActivityOptions::IdleSystemSleepDisabled));
    }

    /// Latency-critical turns off timer coalescing for the whole process, which
    /// made the machine feel slow; it must not come back.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_activity_does_not_disable_timer_coalescing() {
        use objc2_foundation::NSActivityOptions;
        let options = NSActivityOptions::UserInitiatedAllowingIdleSystemSleep;
        assert!(!options.contains(NSActivityOptions::LatencyCritical));
    }
}
