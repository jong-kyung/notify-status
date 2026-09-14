#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

mod status;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod linux;

pub use status::{Authorization, NotificationStatus, Reason};

/// Read-only query for the host's notification authorization and DND state.
///
/// The returned Promise NEVER rejects. Environmental failures collapse to
/// `{ authorization: 'unsupported', reason: 'noBundleId' | 'noAumid' | 'unsupportedPlatform' }`,
/// and library/runtime failures (panics, JoinError, unmapped HRESULTs, parse failures)
/// collapse to `reason: 'internalError'`.
#[napi]
pub async fn get_notification_status() -> napi::Result<NotificationStatus> {
    Ok(run_platform_query().await.unwrap_or_else(internal_error))
}

#[cfg(target_os = "macos")]
async fn run_platform_query() -> Result<NotificationStatus, ()> {
    // Keep the thread-local pool on the worker, including the no-bundle preflight.
    tokio::task::spawn_blocking(|| objc2::rc::autoreleasepool(|_| macos::query()))
        .await
        .map_err(|_join_err| ())
}

#[cfg(target_os = "windows")]
async fn run_platform_query() -> Result<NotificationStatus, ()> {
    tokio::task::spawn_blocking(windows::query)
        .await
        .map_err(|_join_err| ())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
async fn run_platform_query() -> Result<NotificationStatus, ()> {
    Ok(linux::query())
}

fn internal_error(_: ()) -> NotificationStatus {
    let platform = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        platform => platform,
    };
    NotificationStatus::unsupported(platform, Reason::InternalError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_error_preserves_platform_and_failure_fields() {
        let result = internal_error(());

        #[cfg(target_os = "macos")]
        assert_eq!(result.platform, "darwin");
        #[cfg(target_os = "windows")]
        assert_eq!(result.platform, "win32");
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        assert_eq!(result.platform, std::env::consts::OS);

        assert_eq!(result.authorization, Authorization::Unsupported);
        assert_eq!(result.reason, Some(Reason::InternalError));
        assert!(!result.do_not_disturb);
    }
}
