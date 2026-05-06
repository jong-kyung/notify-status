import test from 'ava';

import * as lib from '../index.js';
import { getNotificationStatus, isEffectivelyEnabled } from '../index.js';

const ALLOWED_KEYS = new Set(['authorization', 'doNotDisturb', 'platform', 'reason']);
const VALID_AUTHORIZATIONS = new Set(['granted', 'denied', 'notDetermined', 'unsupported']);
const VALID_REASONS = new Set(['noBundleId', 'noAumid', 'unsupportedPlatform', 'internalError']);

test('exports getNotificationStatus as a function', (t) => {
  t.is(typeof getNotificationStatus, 'function');
});

test('exports isEffectivelyEnabled as a function', (t) => {
  t.is(typeof isEffectivelyEnabled, 'function');
});

// Covers AE8 — no mutation/prompt API leaks.
test('does not export any mutation/prompt function', (t) => {
  const exported = Object.keys(lib);
  for (const banned of [
    'requestAuthorization',
    'requestPermission',
    'requestAccess',
    'setAuthorization',
    'enableNotifications',
    'disableNotifications',
  ]) {
    t.false(exported.includes(banned), `unexpected export: ${banned}`);
  }
});

// Covers AE3 — isEffectivelyEnabled truth table.
test('isEffectivelyEnabled is true only for granted + doNotDisturb=false', (t) => {
  t.true(isEffectivelyEnabled({ authorization: 'granted', doNotDisturb: false, platform: 'darwin' }));
  t.false(isEffectivelyEnabled({ authorization: 'granted', doNotDisturb: true, platform: 'darwin' }));
  t.false(isEffectivelyEnabled({ authorization: 'denied', doNotDisturb: false, platform: 'win32' }));
  t.false(isEffectivelyEnabled({ authorization: 'notDetermined', doNotDisturb: false, platform: 'darwin' }));
  t.false(
    isEffectivelyEnabled({
      authorization: 'unsupported',
      doNotDisturb: false,
      platform: 'linux',
      reason: 'unsupportedPlatform',
    }),
  );
});

test('isEffectivelyEnabled handles null/undefined gracefully', (t) => {
  t.false(isEffectivelyEnabled(null));
  t.false(isEffectivelyEnabled(undefined));
});

test('returns a Promise that resolves to an object with allowed keys only', async (t) => {
  const result = await getNotificationStatus();
  t.is(typeof result, 'object');
  t.not(result, null);

  for (const key of Object.keys(result)) {
    t.true(ALLOWED_KEYS.has(key), `unexpected key: ${key}`);
  }
});

test('returned authorization is one of the documented variants', async (t) => {
  const result = await getNotificationStatus();
  t.true(VALID_AUTHORIZATIONS.has(result.authorization), `bad authorization: ${result.authorization}`);
});

test('reason is present iff authorization === unsupported', async (t) => {
  const result = await getNotificationStatus();
  if (result.authorization === 'unsupported') {
    t.true(VALID_REASONS.has(result.reason), `bad reason: ${result.reason}`);
  } else {
    t.is(result.reason, undefined, 'reason must be undefined when not unsupported');
  }
});

test('doNotDisturb is always a boolean', async (t) => {
  const result = await getNotificationStatus();
  t.is(typeof result.doNotDisturb, 'boolean');
});

test('platform is a non-empty string', async (t) => {
  const result = await getNotificationStatus();
  t.is(typeof result.platform, 'string');
  t.true(result.platform.length > 0);
});
