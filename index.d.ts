export type {
  Authorization,
  Reason,
  NotificationStatus,
} from './binding';
export { getNotificationStatus } from './binding';

import type { NotificationStatus } from './binding';

/**
 * Returns `true` iff `status.authorization === 'granted'` and `status.doNotDisturb === false`.
 *
 * Pure JS helper — does not call into Rust.
 */
export declare function isEffectivelyEnabled(status: NotificationStatus): boolean;
