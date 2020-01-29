const ValidationResult = require('../../validation/ValidationResult');
const UnexpectedIdentityTypeError = require('../../errors/UnexpectedIdentityTypeError');
const IdentityNotFoundError = require('../../errors/IdentityNotFoundError');

/**
 * @param {DataProvider} dataProvider
 * @return {validateIdentityExistenceAndType}
 */
function validateIdentityExistenceAndTypeFactory(dataProvider) {
  /**
   * @typedef validateIdentityExistenceAndType
   * @param {string} identityId
   * @param {number[]} expectedIdentityTypes
   * @return {Promise<ValidationResult>}
   */
  async function validateIdentityExistenceAndType(identityId, expectedIdentityTypes) {
    const result = new ValidationResult();

    const rawIdentity = await dataProvider.fetchIdentity(identityId);

    if (!rawIdentity) {
      result.addError(new IdentityNotFoundError(identityId));
      return result;
    }

    if (!expectedIdentityTypes.includes(rawIdentity.type)) {
      result.addError(new UnexpectedIdentityTypeError(rawIdentity, expectedIdentityTypes));
    }

    return result;
  }

  return validateIdentityExistenceAndType;
}

module.exports = validateIdentityExistenceAndTypeFactory;
