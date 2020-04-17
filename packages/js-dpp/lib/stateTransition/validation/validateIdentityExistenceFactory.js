const ValidationResult = require('../../validation/ValidationResult');
const IdentityNotFoundError = require('../../errors/IdentityNotFoundError');

/**
 * @param {StateRepository} stateRepository
 * @return {validateIdentityExistence}
 */
function validateIdentityExistenceFactory(stateRepository) {
  /**
   * @typedef validateIdentityExistence
   * @param {string} identityId
   * @return {Promise<ValidationResult>}
   */
  async function validateIdentityExistence(identityId) {
    const result = new ValidationResult();

    const rawIdentity = await stateRepository.fetchIdentity(identityId);

    if (!rawIdentity) {
      result.addError(new IdentityNotFoundError(identityId));
      return result;
    }

    return result;
  }

  return validateIdentityExistence;
}

module.exports = validateIdentityExistenceFactory;
