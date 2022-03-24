const ValidationResult = require('../../validation/ValidationResult');
const InvalidIdentityPublicKeyDisabledError = require('../../errors/consensus/state/identity/InvalidIdentityPublicKeyDisabledError');

/**
 * Validate public keys are enabled
 *
 * @typedef validatePublicKeysAreEnabled
 *
 * @param {RawIdentityPublicKey[]} rawPublicKeys
 *
 * @return {ValidationResult}
 */
function validatePublicKeysAreEnabled(rawPublicKeys) {
  const result = new ValidationResult();

  rawPublicKeys.forEach((pk) => {
    if (pk.disabledAt) {
      result.addError(
        new InvalidIdentityPublicKeyDisabledError(pk.id),
      );
    }
  });

  return result;
}

module.exports = validatePublicKeysAreEnabled;
