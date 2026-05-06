//! Linux (and any non-darwin/win32 platform) branch.
//!
//! There is no per-app notification permission concept on Linux desktops
//! (D-Bus org.freedesktop.Notifications has no equivalent of macOS authorization
//! status or Windows ToastNotificationManager.Setting), so we honestly report
//! `unsupported` rather than fabricate a signal.

use crate::status::{NotificationStatus, Reason};

pub fn query() -> NotificationStatus {
    NotificationStatus::unsupported(std::env::consts::OS, Reason::UnsupportedPlatform)
}
