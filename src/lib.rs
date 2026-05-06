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
    tokio::task::spawn_blocking(macos::query)
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
    NotificationStatus::unsupported(std::env::consts::OS, Reason::InternalError)
}
