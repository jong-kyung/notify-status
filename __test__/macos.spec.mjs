import test from 'ava';
import { getNotificationStatus } from '../index.js';

const isMacOS = process.platform === 'darwin';
const macTest = isMacOS ? test : test.skip;

// Covers AE6: in macOS Electron dev (unbundled) the call must not crash and
// must return the `noBundleId` unsupported payload.
//
// `node` itself runs without an .app bundle, so `Bundle.main.bundleIdentifier`
// is nil. Running this spec from `npm test` exercises exactly the AE6 contract.
macTest('AE6: unbundled host returns unsupported(noBundleId) without crashing', async (t) => {
  const status = await getNotificationStatus();
  t.is(status.platform, 'darwin');
  t.is(status.authorization, 'unsupported');
  t.is(status.doNotDisturb, false);
  t.is(status.reason, 'noBundleId');
});

macTest('AE6: ten concurrent calls all resolve identically', async (t) => {
  const results = await Promise.all(
    Array.from({ length: 10 }, () => getNotificationStatus()),
  );
  for (const r of results) {
    t.is(r.platform, 'darwin');
    t.is(r.authorization, 'unsupported');
    t.is(r.reason, 'noBundleId');
  }
});

macTest('AE6: the function never throws (Promise must always resolve)', async (t) => {
  await t.notThrowsAsync(() => getNotificationStatus());
});
