import { describe, expect, test } from 'vite-plus/test';

import { getNotificationStatus } from '../src/index.js';

const isMacOS = process.platform === 'darwin';
const macTest = isMacOS ? test : test.skip;

// Covers AE6: in macOS Electron dev (unbundled) the call must not crash and
// must return the `noBundleId` unsupported payload.
//
// `node` itself runs without an .app bundle, so `Bundle.main.bundleIdentifier`
// is nil. Running this spec from the test runner exercises exactly the AE6 contract.
describe('macOS — AE6 (unbundled host)', () => {
  macTest('returns unsupported(noBundleId) without crashing', async () => {
    const status = await getNotificationStatus();
    expect(status.platform).toBe('darwin');
    expect(status.authorization).toBe('unsupported');
    expect(status.doNotDisturb).toBe(false);
    expect(status.reason).toBe('noBundleId');
  });

  macTest('ten concurrent calls all resolve identically', async () => {
    const results = await Promise.all(Array.from({ length: 10 }, () => getNotificationStatus()));
    for (const r of results) {
      expect(r.platform).toBe('darwin');
      expect(r.authorization).toBe('unsupported');
      expect(r.reason).toBe('noBundleId');
      expect(r.doNotDisturb).toBe(false);
    }
  });

  macTest('the function never throws (Promise must always resolve)', async () => {
    await expect(getNotificationStatus()).resolves.toBeDefined();
  });

  // Stress / non-flakiness: bursts of 50 calls without a crash or shape drift.
  macTest('50-call burst maintains payload shape and never rejects', async () => {
    const results = await Promise.all(Array.from({ length: 50 }, () => getNotificationStatus()));
    expect(results).toHaveLength(50);
    for (const r of results) {
      expect(r.platform).toBe('darwin');
      expect(r.authorization).toBe('unsupported');
      expect(r.reason).toBe('noBundleId');
    }
  });
});
