//! macOS major version detection via Foundation's NSProcessInfo.
//!
//! We need this to gate DND reading on macOS 26 (Tahoe), where the
//! `~/Library/DoNotDisturb/DB/Assertions.json` file format changed and
//! parsing it returns garbage. Cached after first lookup; the host's macOS
//! version doesn't change at runtime.

use std::sync::OnceLock;

use objc2_foundation::NSProcessInfo;

static MACOS_MAJOR: OnceLock<u32> = OnceLock::new();

/// Returns the host's macOS major version (e.g. 14, 15, 26).
///
/// Returns 0 if the native version cannot be represented as a u32.
pub fn macos_major_version() -> u32 {
    *MACOS_MAJOR.get_or_init(|| {
        let version = NSProcessInfo::processInfo().operatingSystemVersion();
        u32::try_from(version.majorVersion).unwrap_or(0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_sw_vers_major_version() {
        let output = std::process::Command::new("/usr/bin/sw_vers")
            .arg("-productVersion")
            .output()
            .unwrap();
        assert!(output.status.success());
        let expected = std::str::from_utf8(&output.stdout)
            .unwrap()
            .trim()
            .split('.')
            .next()
            .unwrap()
            .parse::<u32>()
            .unwrap();

        let actual = objc2::rc::autoreleasepool(|_| macos_major_version());
        assert_eq!(actual, expected);
    }
}
