//! Windows authorization read via ToastNotificationManager.
//!
//! `CreateToastNotifier` / `Setting()` report a missing or invalid AUMID in
//! their HRESULT. We map those failures directly instead of duplicating the
//! check with lower-level Win32 calls.

use crate::status::Authorization;

/// `HRESULT_FROM_WIN32(ERROR_NOT_FOUND)` — `CreateToastNotifier` returns this
/// when AUMID is missing or unrecognised by the toast subsystem.
const HRESULT_ERROR_NOT_FOUND: i32 = 0x80070490_u32 as i32;
/// `E_INVALIDARG` — also seen when AUMID is malformed.
const HRESULT_E_INVALIDARG: i32 = 0x80070057_u32 as i32;

#[derive(Debug, PartialEq, Eq)]
pub enum AuthError {
    NoAumid,
    Internal,
}

/// Pure mapping from `NotificationSetting` raw value (`i32`) to `Authorization`.
///
/// Per Microsoft, only 5 cases exist (`Enabled` plus four `Disabled*` variants).
/// Out-of-range values default to `Denied` — conservative: if Microsoft adds a
/// new "disabled" case in a future Windows version, denying-by-default is the
/// safer interpretation than granting.
pub fn map_notification_setting(raw: i32) -> Authorization {
    match raw {
        0 => Authorization::Granted,    // Enabled
        1..=4 => Authorization::Denied, // DisabledFor* / DisabledBy*
        _ => Authorization::Denied,
    }
}

/// Maps a Windows HRESULT to the appropriate `Reason` bucket.
///
/// `ERROR_NOT_FOUND` and `E_INVALIDARG` both indicate "AUMID present but
/// unusable" — closest to the user-actionable "noAumid" condition. Anything
/// else is library/runtime trouble and lands in `internalError` so consumers
/// can spot regressions in telemetry.
pub fn classify_hresult(hresult_raw: i32) -> AuthError {
    if hresult_raw == HRESULT_ERROR_NOT_FOUND || hresult_raw == HRESULT_E_INVALIDARG {
        AuthError::NoAumid
    } else {
        AuthError::Internal
    }
}

#[cfg(target_os = "windows")]
pub fn read_authorization() -> Result<Authorization, AuthError> {
    use windows::UI::Notifications::ToastNotificationManager;

    let notifier = ToastNotificationManager::CreateToastNotifier()
        .map_err(|err| classify_hresult(err.code().0))?;

    let setting = notifier
        .Setting()
        .map_err(|err| classify_hresult(err.code().0))?;

    Ok(map_notification_setting(setting.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_documented_notification_setting_values() {
        assert_eq!(map_notification_setting(0), Authorization::Granted); // Enabled
        assert_eq!(map_notification_setting(1), Authorization::Denied); // DisabledForApplication
        assert_eq!(map_notification_setting(2), Authorization::Denied); // DisabledForUser
        assert_eq!(map_notification_setting(3), Authorization::Denied); // DisabledByGroupPolicy
        assert_eq!(map_notification_setting(4), Authorization::Denied); // DisabledByManifest
    }

    #[test]
    fn unknown_setting_values_default_to_denied() {
        assert_eq!(map_notification_setting(-1), Authorization::Denied);
        assert_eq!(map_notification_setting(99), Authorization::Denied);
        assert_eq!(map_notification_setting(i32::MAX), Authorization::Denied);
    }

    #[test]
    fn classifies_no_aumid_hresults_correctly() {
        assert_eq!(
            classify_hresult(HRESULT_ERROR_NOT_FOUND),
            AuthError::NoAumid
        );
        assert_eq!(classify_hresult(HRESULT_E_INVALIDARG), AuthError::NoAumid);
    }

    #[test]
    fn classifies_unmapped_hresults_as_internal() {
        // E_FAIL = 0x80004005 — generic failure, neither AUMID-related.
        assert_eq!(classify_hresult(0x80004005_u32 as i32), AuthError::Internal);
        // E_OUTOFMEMORY = 0x8007000E — runtime trouble.
        assert_eq!(classify_hresult(0x8007000E_u32 as i32), AuthError::Internal);
        // RPC_E_CHANGED_MODE — should not happen here but test the default.
        assert_eq!(classify_hresult(0x80010106_u32 as i32), AuthError::Internal);
    }
}
