const identitySchema = require('../../../schema/identity/identity');

const Identity = require('../Identity');

/**
 * @param {JsonSchemaValidator} validator
 * @param {validateIdentityType} validateIdentityType
 * @param {validatePublicKeys} validatePublicKeys
 * @return {validateIdentity}
 */
function validateIdentityFactory(
  validator,
  validateIdentityType,
  validatePublicKeys,
) {
  /**
   * Validates identity
   *
   * @typedef validateIdentity
   * @param {Identity|RawIdentity} identity
   * @return {ValidationResult}
   */
  function validateIdentity(identity) {
    let rawIdentity = identity;

    if (identity instanceof Identity) {
      rawIdentity = identity.toJSON();
    }

    const result = validator.validate(
      identitySchema,
      rawIdentity,
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      validateIdentityType(rawIdentity.type),
    );

    result.merge(
      validatePublicKeys(rawIdentity.publicKeys),
    );

    return result;
  }

  return validateIdentity;
}

module.exports = validateIdentityFactory;
