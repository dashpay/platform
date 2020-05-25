const ValidationResult = require('../../../validation/ValidationResult');

const IdentityAlreadyExistsError = require('../../../errors/IdentityAlreadyExistsError');

/**
 * @param {StateRepository} stateRepository
 * @param {validateAssetLockTransaction} validateAssetLockTransaction
 * @param {validateIdentityPublicKeyUniqueness} validateIdentityPublicKeyUniqueness
 * @return {validateIdentityCreateSTData}
 */
function validateIdentityCreateSTDataFactory(
  stateRepository,
  validateAssetLockTransaction,
  validateIdentityPublicKeyUniqueness,
) {
  /**
   *
   * Do we need to check that key ids are incremental?
   *
   * For later versions:
   * 1. We need to check that outpoint exists (not now)
   * 2. Verify ownership proof signature, as it requires special transaction to be implemented
   */

  /**
   * @typedef validateIdentityCreateSTData
   * @param {IdentityCreateTransition} identityCreateTransition
   * @return {ValidationResult}
   */
  async function validateIdentityCreateSTData(identityCreateTransition) {
    const result = new ValidationResult();

    // Check if identity with such id already exists
    const identityId = identityCreateTransition.getIdentityId();
    const identity = await stateRepository.fetchIdentity(identityId);

    if (identity) {
      result.addError(new IdentityAlreadyExistsError(identityCreateTransition));
    }

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      await validateAssetLockTransaction(identityCreateTransition),
    );

    if (!result.isValid()) {
      return result;
    }

    const [firstPublicKey] = identityCreateTransition.getPublicKeys();

    result.merge(
      await validateIdentityPublicKeyUniqueness(
        firstPublicKey,
      ),
    );

    return result;
  }

  return validateIdentityCreateSTData;
}

module.exports = validateIdentityCreateSTDataFactory;
