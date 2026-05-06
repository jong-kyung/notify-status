import test from 'ava';
import { getNotificationStatus } from '../index.js';

const isWindows = process.platform === 'win32';
const winTest = isWindows ? test : test.skip;

// Covers AE7: on Windows without `app.setAppUserModelId()` the call returns
// the noAumid unsupported payload and never throws.
//
// The test runner / CI executes naked node, which never sets an explicit AUMID
// and is not a packaged app, so both pre-flights fail — exactly the scenario
// AE7 specifies.
winTest('AE7: missing AUMID returns unsupported(noAumid) without crashing', async (t) => {
  const status = await getNotificationStatus();
  t.is(status.platform, 'win32');
  t.is(status.authorization, 'unsupported');
  t.is(status.doNotDisturb, false);
  t.is(status.reason, 'noAumid');
});

winTest('AE7: ten concurrent calls all resolve identically', async (t) => {
  const results = await Promise.all(
    Array.from({ length: 10 }, () => getNotificationStatus()),
  );
  for (const r of results) {
    t.is(r.platform, 'win32');
    t.is(r.authorization, 'unsupported');
    t.is(r.reason, 'noAumid');
  }
});

winTest('AE7: the function never throws (Promise must always resolve)', async (t) => {
  await t.notThrowsAsync(() => getNotificationStatus());
});
