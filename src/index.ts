import { createRequire } from "node:module";
import type { NotificationStatus } from "../binding.cjs";

export type { Authorization, Reason, NotificationStatus } from "../binding.cjs";

const require = createRequire(import.meta.url);
const binding: {
  getNotificationStatus: () => Promise<NotificationStatus>;
} = require("../binding.cjs");

/**
 * Read-only query for the host's notification authorization and DND state.
 *
 * The returned Promise NEVER rejects. Environmental failures collapse to
 * `{ authorization: 'unsupported', reason: 'noBundleId' | 'noAumid' | 'unsupportedPlatform' }`,
 * and library/runtime failures (panics, JoinError, unmapped HRESULTs, parse failures)
 * collapse to `reason: 'internalError'`.
 */
export const getNotificationStatus: () => Promise<NotificationStatus> =
  binding.getNotificationStatus;

/**
 * Returns `true` iff `status.authorization === 'granted'` and `status.doNotDisturb === false`.
 *
 * Pure JS helper — does not call into Rust.
 */
export function isEffectivelyEnabled(status: NotificationStatus | null | undefined): boolean {
  return status != null && status.authorization === "granted" && status.doNotDisturb === false;
}
