//! macOS major version detection via `sw_vers -productVersion`.
//!
//! We need this to gate DND reading on macOS 26 (Tahoe), where the
//! `~/Library/DoNotDisturb/DB/Assertions.json` file format changed and
//! parsing it returns garbage. Cached after first lookup; the host's macOS
//! version doesn't change at runtime.

use std::process::Command;
use std::sync::OnceLock;

static MACOS_MAJOR: OnceLock<u32> = OnceLock::new();

/// Returns the host's macOS major version (e.g. 14, 15, 26).
///
/// Returns 0 if `sw_vers` is unavailable or its output is unparseable.
/// Callers should treat 0 as "unknown" and fall back to safe defaults.
pub fn macos_major_version() -> u32 {
    *MACOS_MAJOR.get_or_init(query_macos_major)
}

fn query_macos_major() -> u32 {
    let output = Command::new("/usr/bin/sw_vers")
        .arg("-productVersion")
        .output();

    let stdout = match output {
        Ok(out) if out.status.success() => out.stdout,
        _ => return 0,
    };

    let s = match std::str::from_utf8(&stdout) {
        Ok(s) => s.trim(),
        Err(_) => return 0,
    };

    parse_macos_major(s)
}

/// Parses the major version out of a `sw_vers -productVersion` string.
///
/// Examples:
/// - `"15.4.1"` → 15
/// - `"26.0"`   → 26
/// - `""` or `"abc"` → 0
fn parse_macos_major(version: &str) -> u32 {
    version
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typical_macos_versions() {
        assert_eq!(parse_macos_major("15.4.1"), 15);
        assert_eq!(parse_macos_major("26.0"), 26);
        assert_eq!(parse_macos_major("14"), 14);
        assert_eq!(parse_macos_major("12.7.6"), 12);
    }

    #[test]
    fn handles_malformed_input() {
        assert_eq!(parse_macos_major(""), 0);
        assert_eq!(parse_macos_major("abc"), 0);
        assert_eq!(parse_macos_major("xx.yy"), 0);
    }
}
