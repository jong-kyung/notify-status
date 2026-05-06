//! macOS branch entry point.
//!
//! Crash isolation strategy (load-bearing for R9 / AE6):
//!
//! 1. Pre-flight: if `Bundle.main.bundleIdentifier` is nil we skip the UN call
//!    entirely — calling `UNUserNotificationCenter.currentNotificationCenter()`
//!    without a bundle identifier raises `NSInternalInconsistencyException`
//!    ("Invalid parameter not satisfying: bundleIdentifier != nil") which
//!    aborts the process. The pre-flight catches the dominant failure mode
//!    (naked node, unbundled scripts).
//!
//! 2. Defense-in-depth: the FFI block is wrapped in nested catchers:
//!    `panic::catch_unwind(AssertUnwindSafe(|| objc2::exception::catch(...)))`
//!    The outer catch_unwind defangs Rust panics (including the panic that
//!    `objc2::exception::catch` rethrows when its closure panics). The inner
//!    `exception::catch` translates ObjC NSExceptions into a `Result::Err`
//!    rather than aborting. This is what carries the helper-process case
//!    (bundleId is non-nil but UN context is invalid).
//!
//! Failure mapping into `Reason`:
//! - Pre-flight nil bundle → `noBundleId` (positive identification of cause)
//! - Caught NSException OR Rust panic OR auth read failure → `internalError`
//!   (we don't pretend we know the cause was a missing bundle ID)

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::status::{NotificationStatus, Reason};

mod authorization;
mod dnd;
mod dnd_parse;
mod version;

#[cfg(test)]
mod authorization_tests;

pub fn query() -> NotificationStatus {
    if !has_bundle_identifier() {
        return NotificationStatus::unsupported("darwin", Reason::NoBundleId);
    }

    // DND read is best-effort and never fails. Wrap it in catch_unwind too —
    // a serde_json panic on a malformed file should not abort the process.
    let dnd = catch_unwind(AssertUnwindSafe(dnd::read_dnd)).unwrap_or(false);

    let auth_result = catch_unwind(AssertUnwindSafe(|| {
        objc2::exception::catch(authorization::read_authorization)
    }));

    match auth_result {
        // Successful read.
        Ok(Ok(Ok(authorization))) => NotificationStatus {
            authorization,
            do_not_disturb: dnd,
            platform: "darwin".to_string(),
            reason: None,
        },
        // Inner authorization read returned Err — internal failure, not a missing bundle.
        Ok(Ok(Err(_))) => NotificationStatus::unsupported("darwin", Reason::InternalError),
        // ObjC NSException caught by exception::catch.
        Ok(Err(_)) => NotificationStatus::unsupported("darwin", Reason::InternalError),
        // Rust panic caught by catch_unwind (including a panic propagated by exception::catch).
        Err(_) => NotificationStatus::unsupported("darwin", Reason::InternalError),
    }
}

/// Returns true iff `Bundle.main.bundleIdentifier` is non-nil.
///
/// `mainBundle` is documented thread-safe and always returns a non-nil bundle.
/// `bundleIdentifier` returns Option<Retained<NSString>>; a None value here is
/// the documented signal that the calling process is unbundled (the case AE6
/// covers).
fn has_bundle_identifier() -> bool {
    use objc2_foundation::NSBundle;

    let bundle = NSBundle::mainBundle();
    bundle.bundleIdentifier().is_some()
}
