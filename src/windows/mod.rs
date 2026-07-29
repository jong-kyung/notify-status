//! Windows branch entry point.
//!
//! Sequence:
//! 1. `CreateToastNotifier().Setting()` — map AUMID-related HRESULT failures to
//!    `noAumid` and all others to `internalError`.
//! 2. `quiet_hours::read_dnd()` — best-effort, never fails.
//!
//! All steps wrapped in `panic::catch_unwind` so a Rust panic in any FFI path
//! collapses to `internalError` rather than aborting.

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::status::{NotificationStatus, Reason};

mod authorization;

#[cfg(target_os = "windows")]
mod quiet_hours;

pub fn query() -> NotificationStatus {
    let result = catch_unwind(AssertUnwindSafe(query_inner));
    match result {
        Ok(status) => status,
        Err(_panic) => NotificationStatus::unsupported("win32", Reason::InternalError),
    }
}

#[cfg(target_os = "windows")]
fn query_inner() -> NotificationStatus {
    match authorization::read_authorization() {
        Ok(auth) => NotificationStatus {
            authorization: auth,
            do_not_disturb: quiet_hours::read_dnd(),
            platform: "win32".to_string(),
            reason: None,
        },
        Err(authorization::AuthError::NoAumid) => {
            NotificationStatus::unsupported("win32", Reason::NoAumid)
        }
        Err(authorization::AuthError::Internal) => {
            NotificationStatus::unsupported("win32", Reason::InternalError)
        }
    }
}

// On non-windows builds (tests on macOS) the inner function is a stub so the
// pure mapping/classification tests in authorization.rs still compile.
#[cfg(not(target_os = "windows"))]
fn query_inner() -> NotificationStatus {
    NotificationStatus::unsupported("win32", Reason::InternalError)
}
