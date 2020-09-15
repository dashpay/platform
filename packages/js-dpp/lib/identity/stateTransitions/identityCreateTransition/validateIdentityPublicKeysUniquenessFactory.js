const ValidationResult = require('../../../validation/ValidationResult');

const IdentityPublicKeyAlreadyExistsError = require(
  '../../../errors/IdentityPublicKeyAlreadyExistsError',
);

/**
 * Validate that no identity bound to this key (factory)
 *
 * @param {StateRepository} stateRepository
 *
 * @returns {validateIdentityPublicKeysUniqueness}
 */
function validateIdentityPublicKeysUniquenessFactory(stateRepository) {
  /**
   * Validate that no identity bound to this key
   *
   * @typedef validateIdentityPublicKeysUniqueness
   *
   * @param {IdentityPublicKey[]} identityPublicKeys
   *
   * @returns {Promise<ValidationResult>}
   */
  async function validateIdentityPublicKeysUniqueness(identityPublicKeys) {
    const validationResult = new ValidationResult();

    const identityPublicKeyHashes = identityPublicKeys
      .map((identityPublicKey) => identityPublicKey.hash());

    const identitiesByKeys = await stateRepository
      .fetchIdentityIdsByPublicKeyHashes(identityPublicKeyHashes);

    Object.entries(identitiesByKeys)
      .filter(([, identityId]) => identityId !== null)
      .forEach(([identityPublicKeyHash]) => {
        validationResult.addError(
          new IdentityPublicKeyAlreadyExistsError(identityPublicKeyHash),
        );
      });

    return validationResult;
  }

  return validateIdentityPublicKeysUniqueness;
}

module.exports = validateIdentityPublicKeysUniquenessFactory;
