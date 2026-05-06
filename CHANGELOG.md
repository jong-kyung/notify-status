# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.1] — 2026-05-06

### Added

- `getNotificationStatus(): Promise<NotificationStatus>` — read-only async query for notification authorization and DND state. The Promise never rejects; environmental failures collapse to `unsupported` with a `reason` field, library/runtime failures collapse to `reason: 'internalError'`.
- `isEffectivelyEnabled(status)` — pure JS helper returning `true` iff `authorization === 'granted' && doNotDisturb === false`.
- macOS branch (`darwin-arm64`, `darwin-x64`):
  - Authorization read via `UNUserNotificationCenter.getNotificationSettings`, mapping `authorized`/`provisional`/`ephemeral` → `granted`, `denied` → `denied`, `notDetermined` → `notDetermined`.
  - Bundle-ID pre-flight + `objc2::exception::catch` + `panic::catch_unwind` defang the documented `NSInternalInconsistencyException` crash from `macos-notification-state` (Electron #45570).
  - DND/Focus read via `~/Library/DoNotDisturb/DB/Assertions.json` on macOS 12 - 15. macOS 26+ (Tahoe) returns `false` as a documented stub; the Tahoe file format will be supported in a v1.x release.
- Windows branch (`win32-x64`, `win32-arm64`):
  - Authorization read via `ToastNotificationManager.CreateToastNotifier().Setting()`, mapping `Enabled` → `granted` and all four `Disabled*` cases → `denied`.
  - Two AUMID pre-flights (`GetCurrentProcessExplicitAppUserModelID` for Electron / Squirrel, `GetCurrentApplicationUserModelId` for MSIX/UWP).
  - Quiet Hours / Focus Assist read via `ntdll!NtQueryWnfStateData` (best-effort). `NOTIFY_STATUS_DISABLE_WNF` env var disables this path entirely.
- Linux: returns `{ authorization: 'unsupported', doNotDisturb: false, platform: 'linux', reason: 'unsupportedPlatform' }`. No prebuilt artifact; consumer hosts that fail to resolve a per-triple `*.node` will get `unsupportedPlatform` from the JS-level no-op.
- NAPI-RS prebuilt distribution via per-triple npm subpackages, GitHub Actions matrix build/publish, and a `windows-11-arm` smoke test that verifies the cross-compiled aarch64 binary loads on real ARM64 Windows before publish.

[0.0.1]: https://github.com/jklee/notify-status/releases/tag/v0.0.1
