const identitySchema = require('../../../schema/identity/identity');

const convertBuffersToArrays = require('../../util/convertBuffersToArrays');

/**
 * @param {JsonSchemaValidator} validator
 * @param {validatePublicKeys} validatePublicKeys
 * @param {validateProtocolVersion} validateProtocolVersion
 *
 * @return {validateIdentity}
 */
function validateIdentityFactory(
  validator,
  validatePublicKeys,
  validateProtocolVersion,
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
      validateProtocolVersion(rawIdentity.protocolVersion),
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
