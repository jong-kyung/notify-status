import { describe, expect, test } from 'vite-plus/test';

import * as lib from '../src/index.js';
import { getNotificationStatus, isEffectivelyEnabled } from '../src/index.js';
import type { NotificationStatus } from '../src/index.js';

const ALLOWED_KEYS = new Set(['authorization', 'doNotDisturb', 'platform', 'reason']);
const VALID_AUTHORIZATIONS = new Set(['granted', 'denied', 'notDetermined', 'unsupported']);
const VALID_REASONS = new Set(['noBundleId', 'noAumid', 'unsupportedPlatform', 'internalError']);

describe('exports', () => {
  test('getNotificationStatus is a function', () => {
    expect(typeof getNotificationStatus).toBe('function');
  });

  test('isEffectivelyEnabled is a function', () => {
    expect(typeof isEffectivelyEnabled).toBe('function');
  });

  // Covers AE8 — no mutation/prompt API leaks.
  test('does not export any mutation/prompt function', () => {
    const exported = Object.keys(lib);
    for (const banned of [
      'requestAuthorization',
      'requestPermission',
      'requestAccess',
      'setAuthorization',
      'enableNotifications',
      'disableNotifications',
    ]) {
      expect(exported, `unexpected export: ${banned}`).not.toContain(banned);
    }
  });

  // R1 — read-only library: no side-effect functions.
  test('exports exactly { getNotificationStatus, isEffectivelyEnabled }', () => {
    const exported = Object.keys(lib).sort();
    expect(exported).toEqual(['getNotificationStatus', 'isEffectivelyEnabled']);
  });
});

describe('isEffectivelyEnabled (Covers AE3, R3)', () => {
  test('granted + DND off → true', () => {
    expect(
      isEffectivelyEnabled({
        authorization: 'granted',
        doNotDisturb: false,
        platform: 'darwin',
      } as NotificationStatus),
    ).toBe(true);
  });

  test('granted + DND on → false', () => {
    expect(
      isEffectivelyEnabled({
        authorization: 'granted',
        doNotDisturb: true,
        platform: 'darwin',
      } as NotificationStatus),
    ).toBe(false);
  });

  test('denied + DND off → false', () => {
    expect(
      isEffectivelyEnabled({
        authorization: 'denied',
        doNotDisturb: false,
        platform: 'win32',
      } as NotificationStatus),
    ).toBe(false);
  });

  test('notDetermined → false regardless of DND', () => {
    expect(
      isEffectivelyEnabled({
        authorization: 'notDetermined',
        doNotDisturb: false,
        platform: 'darwin',
      } as NotificationStatus),
    ).toBe(false);
    expect(
      isEffectivelyEnabled({
        authorization: 'notDetermined',
        doNotDisturb: true,
        platform: 'darwin',
      } as NotificationStatus),
    ).toBe(false);
  });

  test('unsupported → false regardless of reason', () => {
    for (const reason of [
      'noBundleId',
      'noAumid',
      'unsupportedPlatform',
      'internalError',
    ] as const) {
      expect(
        isEffectivelyEnabled({
          authorization: 'unsupported',
          doNotDisturb: false,
          platform: 'linux',
          reason,
        } as NotificationStatus),
      ).toBe(false);
    }
  });

  test('null / undefined → false (defensive)', () => {
    expect(isEffectivelyEnabled(null)).toBe(false);
    expect(isEffectivelyEnabled(undefined)).toBe(false);
  });

  test('does not coerce truthy non-boolean DND', () => {
    // doNotDisturb must be the boolean false literal — defensive against
    // upstream API breaking the contract (R2 requires boolean).
    expect(
      isEffectivelyEnabled({
        authorization: 'granted',
        // @ts-expect-error intentionally violating the contract for defensive coverage
        doNotDisturb: 0,
        platform: 'darwin',
      }),
    ).toBe(false);
  });

  test('treats unknown authorization as not enabled', () => {
    expect(
      isEffectivelyEnabled({
        // @ts-expect-error intentionally violating the contract
        authorization: 'GRANTED',
        doNotDisturb: false,
        platform: 'darwin',
      }),
    ).toBe(false);
  });
});

describe('getNotificationStatus (Covers R1, R2, R8, R11)', () => {
  test('resolves to a non-null object with only allowed keys', async () => {
    const result = await getNotificationStatus();
    expect(typeof result).toBe('object');
    expect(result).not.toBeNull();

    for (const key of Object.keys(result)) {
      expect(ALLOWED_KEYS.has(key), `unexpected key: ${key}`).toBe(true);
    }
  });

  test('authorization is one of the documented variants', async () => {
    const result = await getNotificationStatus();
    expect(VALID_AUTHORIZATIONS.has(result.authorization)).toBe(true);
  });

  test('reason is present iff authorization === unsupported', async () => {
    const result = await getNotificationStatus();
    if (result.authorization === 'unsupported') {
      expect(result.reason).toBeDefined();
      expect(VALID_REASONS.has(result.reason as string)).toBe(true);
    } else {
      expect(result.reason).toBeUndefined();
    }
  });

  test('doNotDisturb is always a boolean', async () => {
    const result = await getNotificationStatus();
    expect(typeof result.doNotDisturb).toBe('boolean');
  });

  test('platform is a non-empty string', async () => {
    const result = await getNotificationStatus();
    expect(typeof result.platform).toBe('string');
    expect(result.platform.length).toBeGreaterThan(0);
  });

  test('platform matches process.platform', async () => {
    const result = await getNotificationStatus();
    expect(result.platform).toBe(process.platform);
  });

  // R11 / R8 — never throws, never rejects.
  test('the Promise always resolves (never rejects)', async () => {
    await expect(getNotificationStatus()).resolves.toBeDefined();
  });

  test('20 sequential calls all resolve to identically-shaped payloads', async () => {
    const first = await getNotificationStatus();
    for (let i = 0; i < 19; i++) {
      const next = await getNotificationStatus();
      expect(Object.keys(next).sort()).toEqual(Object.keys(first).sort());
      expect(next.authorization).toBe(first.authorization);
      expect(next.platform).toBe(first.platform);
    }
  });
});
