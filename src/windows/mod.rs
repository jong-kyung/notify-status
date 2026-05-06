//! Windows branch entry point.
//!
//! Sequence:
//! 1. `RoInitialize(MULTITHREADED)` — required before any WinRT call. Tolerate
//!    `RPC_E_CHANGED_MODE` (host already initialised STA).
//! 2. AUMID pre-flight (explicit OR package). If neither is set → `noAumid`.
//! 3. `CreateToastNotifier().Setting()` — map HRESULT failures to `noAumid`
//!    (race-window) or `internalError` (anything else).
//! 4. `quiet_hours::read_dnd()` — best-effort, never fails. Filled by U7.
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
    use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize};

    // RoInitialize: idempotent within a thread. RPC_E_CHANGED_MODE means the
    // calling thread is already STA — tolerate and proceed (ToastNotifier
    // works from STA in practice for Electron callers).
    // SAFETY: WinRT initialisation entrypoint.
    let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };

    if !authorization::has_aumid() {
        return NotificationStatus::unsupported("win32", Reason::NoAumid);
    }

    let dnd = quiet_hours::read_dnd();

    match authorization::read_authorization() {
        Ok(auth) => NotificationStatus {
            authorization: auth,
            do_not_disturb: dnd,
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
