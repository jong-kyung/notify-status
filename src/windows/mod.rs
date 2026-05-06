//! Windows branch — ToastNotificationManager authorization + Quiet Hours.
//!
//! Filled in by U6 (authorization + AUMID guard) and U7 (Quiet Hours via WNF).

use crate::status::{NotificationStatus, Reason};

pub fn query() -> NotificationStatus {
    // Placeholder — replaced by U6.
    NotificationStatus::unsupported("win32", Reason::NoAumid)
}
