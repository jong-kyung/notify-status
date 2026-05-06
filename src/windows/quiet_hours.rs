//! Windows Quiet Hours / Focus Assist read.
//!
//! There is no public API to query global Focus Assist state from a
//! non-packaged Win32 process. (`UserNotificationListener` requires a
//! manifest capability not available to Electron-style hosts.) The path that
//! every shipping detector ends up using is an undocumented WNF state name
//! queried via `ntdll!NtQueryWnfStateData`. We dynamically load the symbol
//! rather than statically link it, and any failure (load, status, schema)
//! collapses to `false` rather than propagating.
//!
//! Two intentional escape valves:
//! - The `NOTIFY_STATUS_DISABLE_WNF` env var, when set, skips the WNF call
//!   entirely. Lets ops disable the undocumented path post-deploy if Microsoft
//!   ships a Windows update that breaks the state name or DWORD interpretation.
//! - The DWORD interpretation (`0` = Off, non-zero = active) is wrapped in a
//!   pure helper so the assumption is unit-tested and easy to revisit.

/// `WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED` — composed from riverar's gist.
#[allow(dead_code)]
const WNF_QUIETHOURS_STATE_NAME: u64 = 0x0D83063EA3BF1C75;

/// Pure interpretation: any non-zero DWORD means a Quiet Hours / Focus Assist
/// profile is active (1 = Priority only, 2 = Alarms only, etc).
pub fn dword_means_dnd_active(dword: u32) -> bool {
    dword != 0
}

pub fn read_dnd() -> bool {
    if std::env::var_os("NOTIFY_STATUS_DISABLE_WNF").is_some() {
        return false;
    }
    read_dnd_via_wnf()
}

#[cfg(target_os = "windows")]
fn read_dnd_via_wnf() -> bool {
    use std::ffi::c_void;

    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
    use windows::core::s;

    type NtQueryWnfStateDataFn = unsafe extern "system" fn(
        state_name: *const u64,
        type_id: *const c_void,
        explicit_scope: *const c_void,
        change_stamp: *mut u32,
        buffer: *mut c_void,
        buffer_size: *mut u32,
    ) -> i32;

    // SAFETY: LoadLibraryA on a system library is safe; HMODULE is process-lifetime
    // for ntdll so we don't FreeLibrary it.
    let module = match unsafe { LoadLibraryA(s!("ntdll.dll")) } {
        Ok(h) => h,
        Err(_) => return false,
    };

    // SAFETY: GetProcAddress on a loaded module; symbol may not exist on
    // pre-Windows-10 hosts (we don't ship there but be defensive).
    let raw_proc = unsafe { GetProcAddress(module, s!("NtQueryWnfStateData")) };
    let raw_proc = match raw_proc {
        Some(p) => p,
        None => return false,
    };

    // SAFETY: We're transmuting a function pointer to its documented (in
    // riverar's gist; Microsoft docs the surface implicitly via WNF helpers)
    // signature. The state name is a value-by-pointer so we must keep `state`
    // alive across the call.
    let func: NtQueryWnfStateDataFn = unsafe { std::mem::transmute(raw_proc) };

    let state: u64 = WNF_QUIETHOURS_STATE_NAME;
    let mut change_stamp: u32 = 0;
    let mut buffer: u32 = 0;
    let mut buffer_size: u32 = 4;

    // SAFETY: All pointers point to live local stack variables; buffer is a
    // u32 with the matching buffer_size of 4. NTSTATUS == 0 means success.
    let status = unsafe {
        func(
            &state,
            std::ptr::null(),
            std::ptr::null(),
            &mut change_stamp,
            &mut buffer as *mut u32 as *mut c_void,
            &mut buffer_size,
        )
    };

    if status != 0 {
        return false;
    }

    dword_means_dnd_active(buffer)
}

#[cfg(not(target_os = "windows"))]
fn read_dnd_via_wnf() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dword_zero_means_dnd_inactive() {
        assert!(!dword_means_dnd_active(0));
    }

    #[test]
    fn nonzero_dword_means_dnd_active() {
        assert!(dword_means_dnd_active(1)); // Priority only
        assert!(dword_means_dnd_active(2)); // Alarms only
        assert!(dword_means_dnd_active(3));
        assert!(dword_means_dnd_active(255));
        assert!(dword_means_dnd_active(u32::MAX));
    }

    #[test]
    fn env_kill_switch_short_circuits_to_false() {
        // Single-threaded test mutates env; restore after.
        let saved = std::env::var_os("NOTIFY_STATUS_DISABLE_WNF");
        // SAFETY: single-threaded test.
        unsafe { std::env::set_var("NOTIFY_STATUS_DISABLE_WNF", "1") };

        assert!(!read_dnd(), "kill-switch must short-circuit to false");

        // SAFETY: restore.
        unsafe {
            match saved {
                Some(v) => std::env::set_var("NOTIFY_STATUS_DISABLE_WNF", v),
                None => std::env::remove_var("NOTIFY_STATUS_DISABLE_WNF"),
            }
        }
    }
}
