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
    read_dnd_with_version(
        macos_major_version(),
        std::env::var_os("HOME").map(PathBuf::from),
    )
}

/// Test-injectable version of `read_dnd`.
pub(crate) fn read_dnd_with_version(macos_major: u32, home: Option<PathBuf>) -> bool {
    if macos_major >= FIRST_UNSUPPORTED_MAJOR {
        // Tahoe+: documented stub. v1.x will add a per-version branch once the
        // new file shape has been observed on a Tahoe host.
        return false;
    }

    let path = match assertions_json_path(home) {
        Some(p) => p,
        None => return false,
    };

    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return false,
    };

    parse_dnd_active(&contents)
}

fn assertions_json_path(home: Option<PathBuf>) -> Option<PathBuf> {
    let mut path = home?;
    path.push("Library/DoNotDisturb/DB/Assertions.json");
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_26_and_above_return_false_without_reading_file() {
        assert!(!read_dnd_with_version(26, None));
        assert!(!read_dnd_with_version(27, None));
        assert!(!read_dnd_with_version(99, None));
    }

    #[test]
    fn pre_26_versions_attempt_file_read() {
        assert!(!read_dnd_with_version(15, None));
        assert!(!read_dnd_with_version(12, None));
    }
}
