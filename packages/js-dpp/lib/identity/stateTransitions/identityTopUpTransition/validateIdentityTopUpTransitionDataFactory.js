const ValidationResult = require('../../../validation/ValidationResult');

/**
 * @param {validateAssetLockTransaction} validateAssetLockTransaction
 * @param {validateIdentityExistence} validateIdentityExistence
 * @return {validateIdentityTopUpTransitionData}
 */
function validateIdentityTopUpTransitionDataFactory(
  validateAssetLockTransaction,
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
   * @param {IdentityTopUpTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateIdentityTopUpTransitionData(stateTransition) {
    const result = new ValidationResult();

    result.merge(
      await validateIdentityExistence(
        stateTransition.getIdentityId(),
      ),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      await validateAssetLockTransaction(stateTransition),
    );

    return result;
  }

  return validateIdentityTopUpTransitionData;
}

module.exports = validateIdentityTopUpTransitionDataFactory;
