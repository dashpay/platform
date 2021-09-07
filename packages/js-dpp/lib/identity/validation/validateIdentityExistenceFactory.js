const ValidationResult = require('../../validation/ValidationResult');
const IdentityNotFoundError = require('../../errors/consensus/signature/IdentityNotFoundError');

/**
 * @param {StateRepository} stateRepository
 * @return {validateIdentityExistence}
 */
function validateIdentityExistenceFactory(stateRepository) {
  /**
   * @typedef {validateIdentityExistence}
   * @param {Identifier} identityId
   * @return {Promise<ValidationResult>}
   */
  async function validateIdentityExistence(identityId) {
    const result = new ValidationResult();

    const identity = await stateRepository.fetchIdentity(identityId);

    if (!identity) {
      result.addError(new IdentityNotFoundError(identityId.toBuffer()));
    }

    result.setData(identity);

    return result;
  }

  return validateIdentityExistence;
}

module.exports = validateIdentityExistenceFactory;
