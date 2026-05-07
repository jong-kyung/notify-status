# notify-status

Cross-platform notification authorization and Do Not Disturb status for Node /
Electron, distributed as NAPI-RS prebuilt binaries.

```
npm install notify-status
```

```js
// ESM
import { getNotificationStatus, isEffectivelyEnabled } from 'notify-status';

// CJS
// const { getNotificationStatus, isEffectivelyEnabled } = require('notify-status');

const status = await getNotificationStatus();
// {
//   authorization: 'granted' | 'denied' | 'notDetermined' | 'unsupported',
//   doNotDisturb: boolean,
//   platform: 'darwin' | 'win32' | 'linux' | string,
//   reason?: 'noBundleId' | 'noAumid' | 'unsupportedPlatform' | 'internalError'
// }

if (isEffectivelyEnabled(status)) {
  // user has granted permission and is not in Focus / Quiet Hours
}
```

The returned Promise **never rejects**. Every error path resolves to a
structured `unsupported` payload — your code only needs the `.then` branch.

## Platform support

| Platform | Authorization | Do Not Disturb | Notes |
| --- | --- | --- | --- |
| macOS 12 – 15 (`darwin-arm64`, `darwin-x64`) | full (`UNUserNotificationCenter`) | Focus state via `~/Library/DoNotDisturb/DB/Assertions.json` | best-effort DND |
| macOS 26 (Tahoe) | full | **stub: always `false`** | the Assertions.json format moved/changed; v1.x will add a Tahoe path |
| Windows 10 1607+ / 11 (`win32-x64`, `win32-arm64`) | full (`ToastNotificationManager.Setting`) | Focus Assist / Quiet Hours via `ntdll!NtQueryWnfStateData` | undocumented WNF path; opt-out via `NOTIFY_STATUS_DISABLE_WNF=1` |
| Linux & everything else | always `unsupported` (`unsupportedPlatform`) | always `false` | no per-app permission concept on D-Bus; honest `unsupported` instead of fake "granted" |

## What `unsupported` means

The library reports `authorization: 'unsupported'` when it could not determine
the host's notification permission state. The `reason` field discriminates:

| `reason` | Meaning | What the consumer should do |
| --- | --- | --- |
| `'noBundleId'` | macOS only. The host process has no `Bundle.bundleIdentifier` (naked `node`, unbundled scripts, some Electron dev environments). | Ensure the app is launched via a bundled `.app` with `CFBundleIdentifier` set in `Info.plist`. |
| `'noAumid'` | Windows only. The process has no Application User Model ID. | Call `app.setAppUserModelId('com.example.YourApp')` early in your Electron main process, or install via a packaged installer (Squirrel, MSIX) that registers an AUMID. |
| `'unsupportedPlatform'` | Linux / BSD / unknown. | No remediation; treat notifications as best-effort and fall back to in-app indicators. |
| `'internalError'` | A library/runtime failure occurred (unmapped HRESULT, panic, JoinError, parse failure, caught NSException not from a missing bundle). | This is the signal you want in your telemetry to spot regressions — distinct from environmental issues above. |

## Recommended integration pattern

Call `getNotificationStatus()` lazily at the points where the answer matters,
**not** on a `setInterval`. Lazy calls migrate cleanly when a future version
adds an `onChange` subscription API.

```js
// Electron main process
import { app, Notification } from 'electron';
import { getNotificationStatus, isEffectivelyEnabled } from 'notify-status';

// Set AUMID early — required for Windows reporting to work.
if (process.platform === 'win32') {
  app.setAppUserModelId('com.example.YourApp');
}

ipcMain.handle('notify:check-status', async () => {
  const status = await getNotificationStatus();
  return {
    canShowToast: isEffectivelyEnabled(status),
    why: explain(status),
  };
});

function explain(s) {
  if (s.authorization === 'denied') return 'permission_denied';
  if (s.doNotDisturb) return 'focus_active';
  if (s.authorization === 'notDetermined') return 'will_prompt_on_first_use';
  if (s.authorization === 'unsupported') return `unsupported:${s.reason ?? 'unknown'}`;
  return 'ready';
}
```

When you need the user to grant permission, call Electron's `new Notification()`
directly — `notify-status` is read-only and will not prompt.

## Build from source

You normally do not need to build; `npm install` pulls a prebuilt binary for
your triple. To build locally with [Vite+](https://viteplus.dev):

```sh
vp install
vp run build         # release (napi build + vp pack)
vp run build:debug   # debug
vp check             # fmt + lint + type-check
vp test              # vitest
vp run test:types    # tsd type contract
```

Plain `pnpm` works too if `vp` is not installed — every script delegates
through `pnpm run` under the hood. Cargo unit tests:

```sh
cargo test --lib
```

## License

MIT — see [`LICENSE`](LICENSE).
