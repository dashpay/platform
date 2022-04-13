const ValidationResult = require('../../validation/ValidationResult');

const MissingMasterPublicKeyError = require('../../errors/consensus/basic/identity/MissingMasterPublicKeyError');

const IdentityPublicKey = require('../IdentityPublicKey');

const MASTER_PURPOSE = IdentityPublicKey.PURPOSES.AUTHENTICATION;
const MASTER_SECURITY_LEVEL = IdentityPublicKey.SECURITY_LEVELS.MASTER;

/**
 * Validate public keys for the identity create ST (factory)
 *
 * @return {validatePublicKeysInIdentityCreateTransition}
 */
function validatePublicKeysInIdentityCreateTransitionFactory() {
  /**
   * Validate public keys for a create identity transaction
   *
   * @typedef validatePublicKeysInIdentityCreateTransition
   *
   * @param {RawIdentityPublicKey[]} rawPublicKeys
   *
   * @return {ValidationResult}
   */
  function validatePublicKeysInIdentityCreateTransition(rawPublicKeys) {
    const result = new ValidationResult();

    // Count how many purpose/security key combinations are here
    const keyPurposesAndLevelsCount = {};
    Object.entries(IdentityPublicKey.PURPOSES).forEach(([, purpose]) => {
      keyPurposesAndLevelsCount[purpose] = {};
      Object.entries(IdentityPublicKey.SECURITY_LEVELS).forEach(([, securityLevel]) => {
        keyPurposesAndLevelsCount[purpose][securityLevel] = 0;
      });
    });

    rawPublicKeys.forEach((rawPublicKey) => {
      keyPurposesAndLevelsCount[rawPublicKey.purpose][rawPublicKey.securityLevel] += 1;
    });

    if (keyPurposesAndLevelsCount[MASTER_PURPOSE][MASTER_SECURITY_LEVEL] === 0) {
      result.addError(new MissingMasterPublicKeyError());
    }

    return result;
  }

  return validatePublicKeysInIdentityCreateTransition;
}

module.exports = validatePublicKeysInIdentityCreateTransitionFactory;
