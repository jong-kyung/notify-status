import { expectAssignable, expectNotAssignable, expectType } from 'tsd';

import { getNotificationStatus, isEffectivelyEnabled } from '../dist/index.mjs';
import type { Authorization, NotificationStatus, Reason } from '../dist/index.mjs';

// R1 / R2 — getNotificationStatus signature.
expectType<() => Promise<NotificationStatus>>(getNotificationStatus);

declare const status: NotificationStatus;

// R3 — helper signature.
expectType<boolean>(isEffectivelyEnabled(status));
expectType<boolean>(isEffectivelyEnabled(null));
expectType<boolean>(isEffectivelyEnabled(undefined));

// R2 — Authorization union members.
expectAssignable<Authorization>('granted');
expectAssignable<Authorization>('denied');
expectAssignable<Authorization>('notDetermined');
expectAssignable<Authorization>('unsupported');

// R2 — Authorization rejects unknown members (case-sensitive, no aliases).
expectNotAssignable<Authorization>('GRANTED');
expectNotAssignable<Authorization>('Granted');
expectNotAssignable<Authorization>('not_determined');
expectNotAssignable<Authorization>('default');

// R2 — Reason union members.
expectAssignable<Reason>('noBundleId');
expectAssignable<Reason>('noAumid');
expectAssignable<Reason>('unsupportedPlatform');
expectAssignable<Reason>('internalError');

// R2 — Reason rejects unknown members.
expectNotAssignable<Reason>('NoBundleId');
expectNotAssignable<Reason>('unknown');

// R2 — NotificationStatus shape: required vs optional fields.
expectAssignable<NotificationStatus>({
  authorization: 'granted',
  doNotDisturb: false,
  platform: 'darwin',
});

expectAssignable<NotificationStatus>({
  authorization: 'unsupported',
  doNotDisturb: false,
  platform: 'linux',
  reason: 'unsupportedPlatform',
});

// reason is optional.
expectAssignable<NotificationStatus>({
  authorization: 'denied',
  doNotDisturb: true,
  platform: 'win32',
});

// R2 — missing required fields are rejected.
expectNotAssignable<NotificationStatus>({
  authorization: 'granted',
  doNotDisturb: false,
});

// doNotDisturb must be boolean (not number).
expectNotAssignable<NotificationStatus>({
  authorization: 'granted',
  doNotDisturb: 0,
  platform: 'darwin',
});
