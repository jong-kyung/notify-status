//! Windows Quiet Hours / Focus Assist read.
//!
//! There is no public API to query global Focus Assist state from a
//! non-packaged Win32 process. (`UserNotificationListener` requires a
//! manifest capability not available to Electron-style hosts.) The path that
//! every shipping detector ends up using is an undocumented WNF state name
//! queried via `ntdll!NtQueryWnfStateData`. We resolve the symbol once from the
//! process-resident module, and any failure (lookup, status, schema)
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
    use std::sync::OnceLock;

    use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows::core::s;

    type NtQueryWnfStateDataFn = unsafe extern "system" fn(
        state_name: *const u64,
        type_id: *const c_void,
        explicit_scope: *const c_void,
        change_stamp: *mut u32,
        buffer: *mut c_void,
        buffer_size: *mut u32,
    ) -> i32;

    static QUERY_FN: OnceLock<Option<NtQueryWnfStateDataFn>> = OnceLock::new();

    // Cache only the export, including its absence, never the changing DND state.
    let Some(func) = QUERY_FN.get_or_init(|| {
        #[cfg(test)]
        tests::SYMBOL_RESOLUTIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // SAFETY: ntdll is resident for the process lifetime. GetModuleHandleA
        // borrows its handle without increasing the reference count; do not free it.
        let module = unsafe { GetModuleHandleA(s!("ntdll.dll")) }.ok()?;

        // SAFETY: The module remains loaded. A missing export is a cached fallback.
        let raw_proc = unsafe { GetProcAddress(module, s!("NtQueryWnfStateData")) }?;

        // SAFETY: Use the existing NtQueryWnfStateData ABI from riverar's gist.
        let func: NtQueryWnfStateDataFn = unsafe { std::mem::transmute(raw_proc) };
        Some(func)
    }) else {
        return false;
    };

    let state: u64 = WNF_QUIETHOURS_STATE_NAME;
    let mut change_stamp: u32 = 0;
    let mut buffer: u32 = 0;
    let mut buffer_size: u32 = 4;

    #[cfg(test)]
    tests::WNF_QUERIES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

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
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    pub(super) static SYMBOL_RESOLUTIONS: AtomicUsize = AtomicUsize::new(0);
    pub(super) static WNF_QUERIES: AtomicUsize = AtomicUsize::new(0);

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
    fn wnf_queries_cache_only_the_symbol_and_honor_the_kill_switch() {
        // Single-threaded test mutates env; restore after.
        let saved = std::env::var_os("NOTIFY_STATUS_DISABLE_WNF");
        // SAFETY: single-threaded test.
        unsafe { std::env::set_var("NOTIFY_STATUS_DISABLE_WNF", "1") };

        let queries_before = WNF_QUERIES.load(Ordering::Relaxed);
        let resolutions_before = SYMBOL_RESOLUTIONS.load(Ordering::Relaxed);
        assert!(!read_dnd(), "kill-switch must short-circuit to false");
        assert_eq!(WNF_QUERIES.load(Ordering::Relaxed), queries_before);
        assert_eq!(
            SYMBOL_RESOLUTIONS.load(Ordering::Relaxed),
            resolutions_before
        );

        // Bypass the env gate to exercise the WNF path without an AUMID.
        // These workers do not read or mutate the environment.
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| {
                    for _ in 0..25 {
                        read_dnd_via_wnf();
                    }
                });
            }
        });

        assert_eq!(SYMBOL_RESOLUTIONS.load(Ordering::Relaxed), 1);
        assert_eq!(WNF_QUERIES.load(Ordering::Relaxed) - queries_before, 100);
        assert!(!read_dnd(), "kill-switch must still work after lookup");
        assert_eq!(WNF_QUERIES.load(Ordering::Relaxed) - queries_before, 100);

        // SAFETY: restore.
        unsafe {
            match saved {
                Some(v) => std::env::set_var("NOTIFY_STATUS_DISABLE_WNF", v),
                None => std::env::remove_var("NOTIFY_STATUS_DISABLE_WNF"),
            }
        }
    }
}
