import test from 'ava';

import { getNotificationStatus } from '../index.js';

test('exports getNotificationStatus as a function', (t) => {
  t.is(typeof getNotificationStatus, 'function');
});

test('returns a Promise that resolves to an object', async (t) => {
  const result = await getNotificationStatus();
  t.is(typeof result, 'object');
  t.not(result, null);
});
