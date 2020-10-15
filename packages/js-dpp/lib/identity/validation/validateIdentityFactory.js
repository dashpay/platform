const identitySchema = require('../../../schema/identity/identity');

const convertBuffersToArrays = require('../../util/convertBuffersToArrays');

/**
 * @param {JsonSchemaValidator} validator
 * @param {validatePublicKeys} validatePublicKeys
 * @return {validateIdentity}
 */
function validateIdentityFactory(
  validator,
  validatePublicKeys,
) {
  /**
   * Validates identity
   *
   * @typedef validateIdentity
   * @param {RawIdentity} rawIdentity
   * @return {ValidationResult}
   */
  function validateIdentity(rawIdentity) {
    const result = validator.validate(
      identitySchema,
      convertBuffersToArrays(rawIdentity),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      validatePublicKeys(rawIdentity.publicKeys),
    );

    return result;
  }

  return validateIdentity;
}

module.exports = validateIdentityFactory;
