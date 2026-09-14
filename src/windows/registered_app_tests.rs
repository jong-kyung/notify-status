//! Run explicitly in a fresh process because setting an AUMID is process-wide.

use std::path::{Path, PathBuf};

use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize, IPersistFile, STGM_READ,
};
use windows::Win32::UI::Shell::PropertiesSystem::{
    IPropertyStore, PSCoerceToCanonicalValue, PSGetPropertyKeyFromName,
};
use windows::Win32::UI::Shell::{
    FOLDERID_Programs, IShellLinkW, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
    SetCurrentProcessExplicitAppUserModelID, ShellLink,
};
use windows::core::{HSTRING, Interface, w};

use crate::Authorization;

struct Shortcut(PathBuf);

impl Drop for Shortcut {
    fn drop(&mut self) {
        // Also clean up after a failed assertion. The success path checks removal below.
        let _ = std::fs::remove_file(&self.0);
    }
}

fn register_shortcut(path: &Path, aumid: &str) {
    struct ComGuard;
    impl Drop for ComGuard {
        fn drop(&mut self) {
            // SAFETY: Balances this function's successful CoInitializeEx on this thread.
            unsafe { CoUninitialize() };
        }
    }

    // SAFETY: COM objects stay on this thread and drop before ComGuard.
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        let _com = ComGuard;
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
        link.SetPath(&HSTRING::from(std::env::current_exe().unwrap().as_path()))
            .unwrap();

        let mut key = Default::default();
        PSGetPropertyKeyFromName(w!("System.AppUserModel.ID"), &mut key).unwrap();
        let mut value = PROPVARIANT::from(aumid);
        PSCoerceToCanonicalValue(&key, &mut value).unwrap();
        let properties: IPropertyStore = link.cast().unwrap();
        properties.SetValue(&key, &value).unwrap();
        properties.Commit().unwrap();
        let persist: IPersistFile = link.cast().unwrap();
        persist.Save(&HSTRING::from(path), true).unwrap();

        // Verify the registered shortcut from disk, not only its in-memory property store.
        let saved: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).unwrap();
        let persisted: IPersistFile = saved.cast().unwrap();
        persisted.Load(&HSTRING::from(path), STGM_READ).unwrap();
        let saved_properties: IPropertyStore = saved.cast().unwrap();
        assert_eq!(saved_properties.GetValue(&key).unwrap().to_string(), aumid);
    }
}

#[test]
#[ignore = "registers a desktop AUMID; CI runs this alone in a separate process"]
fn registered_aumid_queries_preserve_winrt_lifecycle() {
    use std::os::windows::ffi::OsStringExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    // SAFETY: The known-folder API returns a caller-owned, null-terminated string.
    let programs =
        unsafe { SHGetKnownFolderPath(&FOLDERID_Programs, KF_FLAG_DEFAULT, None) }.unwrap();
    let directory = PathBuf::from(std::ffi::OsString::from_wide(unsafe { programs.as_wide() }));
    // SAFETY: Free the owned native buffer after copying it.
    unsafe { CoTaskMemFree(Some(programs.0.cast())) };
    std::fs::create_dir_all(&directory).unwrap();

    let aumid = format!(
        "dev.notify-status.test.{}.{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    );
    let shortcut_path = directory.join(format!("{aumid}.lnk"));
    assert!(
        !shortcut_path.exists(),
        "fixture must not overwrite a shortcut"
    );
    let shortcut = Shortcut(shortcut_path);
    register_shortcut(&shortcut.0, &aumid);

    // SAFETY: This test is run in its own process, so no no-AUMID test can race it.
    unsafe { SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(&aumid)) }.unwrap();
    assert!(super::authorization::has_aumid());
    eprintln!(
        "registered desktop AUMID {aumid} on {}",
        std::env::consts::ARCH
    );

    super::tests::assert_queries_preserve_initialization(|status| {
        assert!(
            matches!(
                status.authorization,
                Authorization::Granted | Authorization::Denied
            ),
            "registered AUMID query failed: authorization={:?}, reason={:?}, worker_has_aumid={}",
            status.authorization,
            status.reason,
            super::authorization::has_aumid(),
        );
        assert_eq!(status.reason, None);
        assert_eq!(status.platform, "win32");
    });

    std::fs::remove_file(&shortcut.0).expect("remove the test's Start Menu shortcut");
}
