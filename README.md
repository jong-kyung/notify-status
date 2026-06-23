# notify-status

[![npm version](https://img.shields.io/npm/v/notify-status.svg)](https://www.npmjs.com/package/notify-status)
[![npm downloads](https://img.shields.io/npm/dm/notify-status.svg)](https://www.npmjs.com/package/notify-status)
[![license](https://img.shields.io/npm/l/notify-status.svg)](LICENSE)

Cross-platform notification authorization and Do Not Disturb status for Node /
Electron, distributed as NAPI-RS prebuilt binaries.

```sh
pnpm add notify-status
# or: npm install notify-status / yarn add notify-status
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

## API

| Export                  | Kind     | Signature / Shape                                                                            |
| ----------------------- | -------- | -------------------------------------------------------------------------------------------- |
| `getNotificationStatus` | function | `() => Promise<NotificationStatus>`                                                          |
| `isEffectivelyEnabled`  | function | `(status: NotificationStatus \| null \| undefined) => boolean`                               |
| `NotificationStatus`    | type     | `{ authorization: Authorization; doNotDisturb: boolean; platform: string; reason?: Reason }` |
| `Authorization`         | type     | `'granted' \| 'denied' \| 'notDetermined' \| 'unsupported'`                                  |
| `Reason`                | type     | `'noBundleId' \| 'noAumid' \| 'unsupportedPlatform' \| 'internalError'`                      |

## Platform support

| Platform                                           | Authorization                                | Do Not Disturb                                              | Notes                                                                                  |
| -------------------------------------------------- | -------------------------------------------- | ----------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| macOS 12 – 15 (`darwin-arm64`, `darwin-x64`)       | full (`UNUserNotificationCenter`)            | Focus state via `~/Library/DoNotDisturb/DB/Assertions.json` | best-effort DND                                                                        |
| macOS 26 (Tahoe)                                   | full                                         | **stub: always `false`**                                    | the Assertions.json format moved/changed; v1.x will add a Tahoe path                   |
| Windows 10 1607+ / 11 (`win32-x64`, `win32-arm64`) | full (`ToastNotificationManager.Setting`)    | Focus Assist / Quiet Hours via `ntdll!NtQueryWnfStateData`  | undocumented WNF path; opt-out via `NOTIFY_STATUS_DISABLE_WNF=1`                       |
| Linux & everything else                            | always `unsupported` (`unsupportedPlatform`) | always `false`                                              | no per-app permission concept on D-Bus; honest `unsupported` instead of fake "granted" |

## What `unsupported` means

The library reports `authorization: 'unsupported'` when it could not determine
the host's notification permission state. The `reason` field discriminates:

| `reason`                | Meaning                                                                                                                               | What the consumer should do                                                                                                                                            |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `'noBundleId'`          | macOS only. The host process has no `Bundle.bundleIdentifier` (naked `node`, unbundled scripts, some Electron dev environments).      | Ensure the app is launched via a bundled `.app` with `CFBundleIdentifier` set in `Info.plist`.                                                                         |
| `'noAumid'`             | Windows only. The process has no Application User Model ID.                                                                           | Call `app.setAppUserModelId('com.example.YourApp')` early in your Electron main process, or install via a packaged installer (Squirrel, MSIX) that registers an AUMID. |
| `'unsupportedPlatform'` | Linux / BSD / unknown.                                                                                                                | No remediation; treat notifications as best-effort and fall back to in-app indicators.                                                                                 |
| `'internalError'`       | A library/runtime failure occurred (unmapped HRESULT, panic, JoinError, parse failure, caught NSException not from a missing bundle). | This is the signal you want in your telemetry to spot regressions — distinct from environmental issues above.                                                          |

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

## More examples

### Plain Node.js / CLI

For sysadmin scripts, CI smoke tests, or any headless-Node usage — Electron is
not required.

```js
// check-notifications.mjs
import { getNotificationStatus, isEffectivelyEnabled } from 'notify-status';

const status = await getNotificationStatus();
console.log(JSON.stringify(status, null, 2));
process.exit(isEffectivelyEnabled(status) ? 0 : 1);
```

```sh
node check-notifications.mjs && echo ready || echo blocked
```

Running a bare `node` script on macOS returns
`{ authorization: 'unsupported', reason: 'noBundleId' }` — that is the correct
answer for an unbundled host process. Run the same script from inside a
packaged `.app` (or from a packaged Electron / Tauri host) to read the host's
real permission state.

### TypeScript — exhaustive narrowing

`Authorization` and `Reason` are exported as string-literal unions, so a
`switch` on `status.authorization` narrows exhaustively. With `strict` (or
`noImplicitReturns`), adding a new variant to either union without a matching
`case` surfaces as a type error.

```ts
import {
  getNotificationStatus,
  type NotificationStatus,
  type Reason,
} from 'notify-status';

function explain(status: NotificationStatus): string {
  switch (status.authorization) {
    case 'granted':
      return status.doNotDisturb ? 'focus_active' : 'ready';
    case 'denied':
      return 'permission_denied';
    case 'notDetermined':
      return 'will_prompt_on_first_use';
    case 'unsupported':
      return remediate(status.reason);
  }
}

function remediate(reason?: Reason): string {
  switch (reason) {
    case 'noBundleId':          return 'unsupported:add_bundle_id_to_app';
    case 'noAumid':             return 'unsupported:set_aumid_on_startup';
    case 'unsupportedPlatform': return 'unsupported:platform';
    case 'internalError':       return 'unsupported:internal_error';
    case undefined:             return 'unsupported:unknown';
  }
}

const message: string = explain(await getNotificationStatus());
```

## Build from source

You normally do not need to build; `npm install` pulls a prebuilt binary for
your triple. To build locally with [Vite+](https://viteplus.dev):

```sh
vp install
vp run build         # release (napi build + vp pack)
vp run build:debug   # debug
vp check             # fmt + lint + type-check
vp test              # vitest
```

Plain `pnpm` works too if `vp` is not installed — every script delegates
through `pnpm run` under the hood. Cargo unit tests:

```sh
cargo test --lib
```

## License

MIT — see [`LICENSE`](LICENSE).
