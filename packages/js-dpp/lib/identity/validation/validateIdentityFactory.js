const identitySchema = require('../../../schema/identity/identity');

const Identity = require('../Identity');

const encodeObjectProperties = require('../../util/encoding/encodeObjectProperties');

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
    const jsonIdentity = encodeObjectProperties(rawIdentity, Identity.ENCODED_PROPERTIES);

    const result = validator.validate(
      identitySchema,
      jsonIdentity,
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
