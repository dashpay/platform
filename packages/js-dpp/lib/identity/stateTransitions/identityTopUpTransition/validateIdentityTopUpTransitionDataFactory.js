const ValidationResult = require('../../../validation/ValidationResult');

/**
 * @param {validateLockTransaction} validateLockTransaction
 * @param {validateIdentityExistence} validateIdentityExistence
 * @return {validateIdentityTopUpTransitionData}
 */
function validateIdentityTopUpTransitionDataFactory(
  validateLockTransaction,
  validateIdentityExistence,
) {
  /**
   *
   * For later versions:
   * 1. We need to check that outpoint exists (not now)
   * 2. Verify ownership proof signature, as it requires special transaction to be implemented
   */

  /**
   * @typedef validateIdentityTopUpTransitionData
   * @param {IdentityTopUpTransition} identityTopUpTransition
   * @return {ValidationResult}
   */
  async function validateIdentityTopUpTransitionData(identityTopUpTransition) {
    const result = new ValidationResult();

    result.merge(
      await validateIdentityExistence(
        identityTopUpTransition.getIdentityId(),
      ),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      await validateLockTransaction(identityTopUpTransition),
    );

    return result;
  }

  return validateIdentityTopUpTransitionData;
}

module.exports = validateIdentityTopUpTransitionDataFactory;
