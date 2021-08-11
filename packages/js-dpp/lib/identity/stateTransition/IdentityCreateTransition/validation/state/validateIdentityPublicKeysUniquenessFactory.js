const ValidationResult = require('../../../../../validation/ValidationResult');

const IdentityPublicKeyAlreadyExistsError = require(
  '../../../../../errors/IdentityPublicKeyAlreadyExistsError',
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

    const identityIds = await stateRepository
      .fetchIdentityIdsByPublicKeyHashes(identityPublicKeyHashes);

    identityPublicKeyHashes
      .forEach((publicKeyHash, index) => {
        if (identityIds[index]) {
          validationResult.addError(
            new IdentityPublicKeyAlreadyExistsError(publicKeyHash),
          );
        }
      });

    return validationResult;
  }

  return validateIdentityPublicKeysUniqueness;
}

module.exports = validateIdentityPublicKeysUniquenessFactory;
