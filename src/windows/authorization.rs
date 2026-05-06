//! Windows authorization read via ToastNotificationManager.
//!
//! Two pre-flights cover the AUMID surface:
//! 1. Explicit AUMID (set via `SetCurrentProcessExplicitAppUserModelID`) — what
//!    Electron / packaged Squirrel installers use. Read via
//!    `GetCurrentProcessExplicitAppUserModelID` (shell32).
//! 2. Package AUMID — what MSIX/UWP apps have. Read via
//!    `GetCurrentApplicationUserModelId` (kernel32).
//!
//! If neither pre-flight resolves an AUMID, we return `noAumid` without calling
//! into WinRT. If at least one resolves, we proceed to `CreateToastNotifier` /
//! `Setting()` and map any HRESULT failure based on its code (race-window
//! ERROR_NOT_FOUND / E_INVALIDARG → `noAumid`; anything else → `internalError`).

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
pub fn has_aumid() -> bool {
    explicit_aumid_set() || package_aumid_set()
}

#[cfg(target_os = "windows")]
fn explicit_aumid_set() -> bool {
    use windows::Win32::UI::Shell::GetCurrentProcessExplicitAppUserModelID;

    // SAFETY: shell32 export, returns Ok(PWSTR) iff explicit AUMID has been set
    // via SetCurrentProcessExplicitAppUserModelID at any point in this process.
    let result = unsafe { GetCurrentProcessExplicitAppUserModelID() };
    if let Ok(ptr) = result {
        if !ptr.is_null() {
            // The caller owns the buffer and must free it with CoTaskMemFree.
            unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(ptr.0 as _)) };
            return true;
        }
    }
    false
}

#[cfg(target_os = "windows")]
fn package_aumid_set() -> bool {
    use windows::Win32::Storage::Packaging::Appx::GetCurrentApplicationUserModelId;

    let mut len: u32 = 0;
    // First call learns the buffer size; we don't actually need the bytes.
    // SAFETY: kernel32 export. WIN32_ERROR is returned by value.
    let rc = unsafe { GetCurrentApplicationUserModelId(&mut len, None) };

    // ERROR_INSUFFICIENT_BUFFER (122) means an AUMID exists; ERROR_SUCCESS is
    // unexpected for a length-probe call but would also signal "exists".
    // APPMODEL_ERROR_NO_PACKAGE / NO_APPLICATION are the failure modes.
    matches!(rc.0, 0 | 122)
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
