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
    read_dnd_with_version(macos_major_version(), assertions_json_path)
}

/// Test-injectable version with a lazy path lookup to preserve the version gate.
pub(crate) fn read_dnd_with_version(
    macos_major: u32,
    assertions_path: impl FnOnce() -> Option<PathBuf>,
) -> bool {
    if macos_major >= FIRST_UNSUPPORTED_MAJOR {
        // Tahoe+: documented stub. v1.x will add a per-version branch once the
        // new file shape has been observed on a Tahoe host.
        return false;
    }

    let path = match assertions_path() {
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
    fn macos_26_and_above_skip_path_lookup() {
        for major in [26, 27, 99] {
            assert!(!read_dnd_with_version(major, || {
                panic!("unsupported macOS versions must not look up the assertions path")
            }));
        }
    }

    #[test]
    fn pre_26_versions_attempt_path_lookup() {
        for major in [12, 15, 25] {
            let mut path_requested = false;
            assert!(!read_dnd_with_version(major, || {
                path_requested = true;
                // A missing HOME is represented by a path lookup returning None.
                None
            }));
            assert!(path_requested);
        }
    }
}
