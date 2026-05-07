import { describe, expect, test } from 'vitest';

import { getNotificationStatus } from '../src/index.js';

const isWindows = process.platform === 'win32';
const winTest = isWindows ? test : test.skip;

// Covers AE7: on Windows without `app.setAppUserModelId()` the call returns
// the noAumid unsupported payload and never throws.
//
// The test runner / CI executes naked node, which never sets an explicit AUMID
// and is not a packaged app, so both pre-flights fail — exactly the scenario
// AE7 specifies.
describe('Windows — AE7 (no AUMID)', () => {
  winTest('missing AUMID returns unsupported(noAumid) without crashing', async () => {
    const status = await getNotificationStatus();
    expect(status.platform).toBe('win32');
    expect(status.authorization).toBe('unsupported');
    expect(status.doNotDisturb).toBe(false);
    expect(status.reason).toBe('noAumid');
  });

  winTest('ten concurrent calls all resolve identically', async () => {
    const results = await Promise.all(
      Array.from({ length: 10 }, () => getNotificationStatus()),
    );
    for (const r of results) {
      expect(r.platform).toBe('win32');
      expect(r.authorization).toBe('unsupported');
      expect(r.reason).toBe('noAumid');
      expect(r.doNotDisturb).toBe(false);
    }
  });

  winTest('the function never throws (Promise must always resolve)', async () => {
    await expect(getNotificationStatus()).resolves.toBeDefined();
  });

  winTest('50-call burst maintains payload shape and never rejects', async () => {
    const results = await Promise.all(
      Array.from({ length: 50 }, () => getNotificationStatus()),
    );
    expect(results).toHaveLength(50);
    for (const r of results) {
      expect(r.platform).toBe('win32');
      expect(r.authorization).toBe('unsupported');
      expect(r.reason).toBe('noAumid');
    }
  });
});
