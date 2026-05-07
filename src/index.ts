import { getNotificationStatus as bindingGetNotificationStatus } from '../binding.js';
import type { NotificationStatus } from '../binding.js';

export type {
  Authorization,
  Reason,
  NotificationStatus,
} from '../binding.js';

/**
 * Read-only query for the host's notification authorization and DND state.
 *
 * The returned Promise NEVER rejects. Environmental failures collapse to
 * `{ authorization: 'unsupported', reason: 'noBundleId' | 'noAumid' | 'unsupportedPlatform' }`,
 * and library/runtime failures (panics, JoinError, unmapped HRESULTs, parse failures)
 * collapse to `reason: 'internalError'`.
 */
export const getNotificationStatus: () => Promise<NotificationStatus> =
  bindingGetNotificationStatus;

/**
 * Returns `true` iff `status.authorization === 'granted'` and `status.doNotDisturb === false`.
 *
 * Pure JS helper — does not call into Rust.
 */
export function isEffectivelyEnabled(
  status: NotificationStatus | null | undefined,
): boolean {
  return (
    status != null &&
    status.authorization === 'granted' &&
    status.doNotDisturb === false
  );
}
