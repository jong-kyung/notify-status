#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

mod status;

pub use status::{Authorization, NotificationStatus, Reason};

/// Read-only query for the host's notification authorization and DND state.
///
/// The returned Promise NEVER rejects. Environmental failures collapse to
/// `{ authorization: 'unsupported', reason: 'noBundleId' | 'noAumid' | 'unsupportedPlatform' }`,
/// and library/runtime failures collapse to `reason: 'internalError'`.
#[napi]
pub async fn get_notification_status() -> napi::Result<NotificationStatus> {
    // Skeleton — replaced platform-by-platform in U3 / U4 / U6.
    Ok(NotificationStatus::unsupported(
        std::env::consts::OS.to_string(),
        Reason::UnsupportedPlatform,
    ))
}
