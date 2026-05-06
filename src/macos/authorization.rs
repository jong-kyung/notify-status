//! macOS authorization read via UNUserNotificationCenter.
//!
//! - Maps `UNAuthorizationStatus` raw values to our `Authorization` enum.
//! - Awaits the async `getNotificationSettings` callback via a `RcBlock` + sync_channel
//!   with a hard timeout (a callback that never fires must not hang the worker thread).
//! - Returns `Err(AuthError)` for any internal failure; the caller decides how to map
//!   that to `unsupported(...)`.

use std::ptr::NonNull;
use std::sync::mpsc::{RecvTimeoutError, sync_channel};
use std::time::Duration;

use crate::status::Authorization;

/// Hard cap on how long we wait for the UN callback to fire.
///
/// Tuning rationale: `getNotificationSettings` is documented as fast (typical
/// completion < 100ms). 2s gives ~20x headroom for slow/contended hosts while
/// keeping a stuck notification daemon from parking the spawn_blocking pool.
/// If the callback fires after the timeout, the late send hits a closed channel
/// and is swallowed by `try_send` (the block never panics).
pub const CALLBACK_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub enum AuthError {
    /// The UN callback did not fire within `CALLBACK_TIMEOUT`.
    Timeout,
    /// The callback fired but the sending half was already dropped.
    ChannelClosed,
}

/// Pure mapping from `UNAuthorizationStatus.0` (NSInteger) to our enum.
///
/// `2 | 3 | 4` (Authorized / Provisional / Ephemeral) all map to `Granted` —
/// for the consumer's purposes "the user has not actively denied" is granted.
/// Out-of-range values are treated as `NotDetermined` (forward-compatible).
pub fn map_authorization_status(raw: i64) -> Authorization {
    match raw {
        0 => Authorization::NotDetermined,
        1 => Authorization::Denied,
        2 | 3 | 4 => Authorization::Granted,
        _ => Authorization::NotDetermined,
    }
}

#[cfg(target_os = "macos")]
pub fn read_authorization() -> Result<Authorization, AuthError> {
    use block2::RcBlock;
    use objc2_user_notifications::{UNNotificationSettings, UNUserNotificationCenter};

    let (tx, rx) = sync_channel::<i64>(1);
    let tx_block = tx.clone();

    // RcBlock (heap-allocated, ref-counted) is required because the framework
    // may invoke the callback on its own queue after the calling code has
    // returned. StackBlock would be a use-after-free here.
    let block = RcBlock::new(move |settings: NonNull<UNNotificationSettings>| {
        // SAFETY: The framework guarantees `settings` is a valid pointer for
        // the duration of the callback invocation.
        let raw = unsafe { settings.as_ref().authorizationStatus().0 };
        let _ = tx_block.try_send(raw as i64); // late fire on closed channel: swallow
    });

    let center = UNUserNotificationCenter::currentNotificationCenter();
    center.getNotificationSettingsWithCompletionHandler(&block);

    drop(tx); // drop our retained sender so a late-fire actually closes the channel

    match rx.recv_timeout(CALLBACK_TIMEOUT) {
        Ok(raw) => Ok(map_authorization_status(raw)),
        Err(RecvTimeoutError::Timeout) => Err(AuthError::Timeout),
        Err(RecvTimeoutError::Disconnected) => Err(AuthError::ChannelClosed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_documented_un_authorization_status_values() {
        assert_eq!(map_authorization_status(0), Authorization::NotDetermined);
        assert_eq!(map_authorization_status(1), Authorization::Denied);
        assert_eq!(map_authorization_status(2), Authorization::Granted); // Authorized
        assert_eq!(map_authorization_status(3), Authorization::Granted); // Provisional
        assert_eq!(map_authorization_status(4), Authorization::Granted); // Ephemeral
    }

    #[test]
    fn out_of_range_values_default_to_not_determined() {
        assert_eq!(map_authorization_status(-1), Authorization::NotDetermined);
        assert_eq!(map_authorization_status(99), Authorization::NotDetermined);
        assert_eq!(map_authorization_status(i64::MAX), Authorization::NotDetermined);
    }
}
