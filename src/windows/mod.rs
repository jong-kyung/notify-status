//! Windows branch entry point.
//!
//! Sequence:
//! 1. `RoInitialize(MULTITHREADED)` — required before any WinRT call. Tolerate
//!    `RPC_E_CHANGED_MODE` (host already initialised STA). Balance every success
//!    with `RoUninitialize` and map other initialization failures to `internalError`.
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
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};

    struct RoUninitializeGuard;

    impl Drop for RoUninitializeGuard {
        fn drop(&mut self) {
            // SAFETY: Created only after successful initialization. This local
            // guard stays on the same thread throughout the synchronous query.
            unsafe { RoUninitialize() };
        }
    }

    // Both S_OK and S_FALSE require a matching RoUninitialize. A host's STA
    // returns RPC_E_CHANGED_MODE, which we tolerate without owning its cleanup.
    // SAFETY: WinRT initialization entrypoint, balanced by the local guard.
    let _winrt = match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
        Ok(()) => Some(RoUninitializeGuard),
        Err(err) if err.code() == RPC_E_CHANGED_MODE => None,
        Err(_) => return NotificationStatus::unsupported("win32", Reason::InternalError),
    };

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

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use windows::Win32::Foundation::S_FALSE;
    use windows::Win32::System::Com::{
        COINIT_APARTMENTTHREADED, COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize,
    };

    use super::*;

    #[test]
    fn repeated_queries_preserve_the_threads_initialization_state() {
        for initial_mode in [
            None,
            Some(COINIT_MULTITHREADED),
            Some(COINIT_APARTMENTTHREADED),
        ] {
            // A fresh thread starts without an explicit COM initialization.
            std::thread::spawn(move || {
                if let Some(mode) = initial_mode {
                    // SAFETY: All successful initializations are balanced on this thread.
                    unsafe { CoInitializeEx(None, mode) }.ok().unwrap();
                }

                for _ in 0..100 {
                    // The test executable has no AUMID, exercising the early return.
                    assert_eq!(query().reason, Some(Reason::NoAumid));
                }

                if let Some(mode) = initial_mode {
                    // S_FALSE proves the query did not tear down the host's apartment.
                    // SAFETY: Probe and cleanup run on the initializing thread.
                    let probe = unsafe { CoInitializeEx(None, mode) };
                    if probe.is_ok() {
                        unsafe { CoUninitialize() };
                    }
                    assert_eq!(probe, S_FALSE);
                    // SAFETY: Balance the host initialization above.
                    unsafe { CoUninitialize() };
                }

                // A leaked MTA initialization would reject this STA initialization
                // with RPC_E_CHANGED_MODE, even if another thread owns an MTA.
                // SAFETY: Probe and cleanup run on the same thread.
                let probe = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
                if probe.is_ok() {
                    unsafe { CoUninitialize() };
                }
                assert!(
                    probe.is_ok(),
                    "query leaked an MTA initialization: {probe:?}"
                );
            })
            .join()
            .unwrap();
        }
    }
}
