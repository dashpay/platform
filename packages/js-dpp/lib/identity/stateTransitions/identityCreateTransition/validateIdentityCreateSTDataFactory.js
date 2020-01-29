const ValidationResult = require('../../../validation/ValidationResult');

const IdentityAlreadyExistsError = require('../../../errors/IdentityAlreadyExistsError');

/**
 * @param {DataProvider} dataProvider
 * @return {validateIdentityCreateSTData}
 */
function validateIdentityCreateSTDataFactory(dataProvider) {
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
    const identity = await dataProvider.fetchIdentity(identityId);

    if (identity) {
      result.addError(new IdentityAlreadyExistsError(identityCreateTransition));
    }

    // TODO: Here we need to fetch lock transaction, extract pubkey from it and verify signature

    return result;
  }

  return validateIdentityCreateSTData;
}

module.exports = validateIdentityCreateSTDataFactory;
