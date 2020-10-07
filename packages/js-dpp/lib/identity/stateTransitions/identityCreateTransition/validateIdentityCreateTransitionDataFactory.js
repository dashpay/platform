const ValidationResult = require('../../../validation/ValidationResult');

const IdentityAlreadyExistsError = require('../../../errors/IdentityAlreadyExistsError');

/**
 * @param {StateRepository} stateRepository
 * @param {validateAssetLockTransaction} validateAssetLockTransaction
 * @param {validateIdentityPublicKeysUniqueness} validateIdentityPublicKeysUniqueness
 * @return {validateIdentityCreateTransitionData}
 */
function validateIdentityCreateTransitionDataFactory(
  stateRepository,
  validateAssetLockTransaction,
  validateIdentityPublicKeysUniqueness,
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
   * @typedef validateIdentityCreateTransitionData
   * @param {IdentityCreateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateIdentityCreateTransitionData(stateTransition) {
    const result = new ValidationResult();

    // Check if identity with such id already exists
    const identityId = stateTransition.getIdentityId();
    const identity = await stateRepository.fetchIdentity(identityId);

    if (identity) {
      result.addError(new IdentityAlreadyExistsError(stateTransition));
    }

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      await validateAssetLockTransaction(stateTransition),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      await validateIdentityPublicKeysUniqueness(
        stateTransition.getPublicKeys(),
      ),
    );

    return result;
  }

  return validateIdentityCreateTransitionData;
}

module.exports = validateIdentityCreateTransitionDataFactory;
