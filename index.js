'use strict';

const binding = require('./binding.js');

module.exports.getNotificationStatus = binding.getNotificationStatus;

/**
 * Returns true iff `status.authorization === 'granted'` and `status.doNotDisturb === false`.
 *
 * Pure JS helper — does not call into Rust.
 *
 * @param {{ authorization: string, doNotDisturb: boolean }} status
 * @returns {boolean}
 */
module.exports.isEffectivelyEnabled = function isEffectivelyEnabled(status) {
  return (
    status != null &&
    status.authorization === 'granted' &&
    status.doNotDisturb === false
  );
};
