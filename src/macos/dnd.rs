//! macOS DND/Focus state read.
//!
//! Strategy:
//! - macOS 26+ (Tahoe and later): the Assertions.json file moved/changed format.
//!   We return `false` rather than parse-fail-quietly; the README documents this
//!   stub clearly so consumers aren't misled by silent dishonesty.
//! - macOS 12 - 15: read `~/Library/DoNotDisturb/DB/Assertions.json` and check
//!   for non-empty `storeAssertionRecords`. Any failure (file missing, unexpected
//!   schema, IO error) collapses to `false`.

use std::path::PathBuf;

use crate::macos::dnd_parse::parse_dnd_active;
use crate::macos::version::macos_major_version;

/// First macOS major version where the Assertions.json approach is known to fail.
const FIRST_UNSUPPORTED_MAJOR: u32 = 26;

pub fn read_dnd() -> bool {
    read_dnd_with_version(macos_major_version())
}

/// Test-injectable version of `read_dnd` that takes the macOS major as a parameter.
pub(crate) fn read_dnd_with_version(macos_major: u32) -> bool {
    if macos_major >= FIRST_UNSUPPORTED_MAJOR {
        // Tahoe+: documented stub. v1.x will add a per-version branch once the
        // new file shape has been observed on a Tahoe host.
        return false;
    }

    let path = match assertions_json_path() {
        Some(p) => p,
        None => return false,
    };

    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return false,
    };

    parse_dnd_active(&contents)
}

fn assertions_json_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let mut path = PathBuf::from(home);
    path.push("Library/DoNotDisturb/DB/Assertions.json");
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_26_and_above_return_false_without_reading_file() {
        // We cannot easily prove the file isn't read, but we can prove the
        // version gate short-circuits on Tahoe even when no HOME is set.
        let saved_home = std::env::var_os("HOME");
        // SAFETY: single-threaded test, restore afterwards.
        unsafe { std::env::remove_var("HOME") };

        assert!(!read_dnd_with_version(26));
        assert!(!read_dnd_with_version(27));
        assert!(!read_dnd_with_version(99));

        if let Some(h) = saved_home {
            // SAFETY: restore.
            unsafe { std::env::set_var("HOME", h) };
        }
    }

    #[test]
    fn pre_26_versions_attempt_file_read() {
        // With no HOME and pre-26 version, file lookup fails → false (no panic).
        let saved_home = std::env::var_os("HOME");
        // SAFETY: single-threaded test, restore afterwards.
        unsafe { std::env::remove_var("HOME") };

        assert!(!read_dnd_with_version(15));
        assert!(!read_dnd_with_version(12));

        if let Some(h) = saved_home {
            // SAFETY: restore.
            unsafe { std::env::set_var("HOME", h) };
        }
    }
}
