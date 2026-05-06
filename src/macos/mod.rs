//! macOS branch — UNUserNotificationCenter authorization + Focus DND.
//!
//! Filled in by U4 (authorization + crash isolation) and U5 (DND + macOS 26 gate).

use crate::status::{NotificationStatus, Reason};

pub fn query() -> NotificationStatus {
    // Placeholder — replaced by U4.
    NotificationStatus::unsupported("darwin", Reason::NoBundleId)
}
