const Identity = require('../Identity');

const ValidationResult = require('../../validation/ValidationResult');

const UnknownIdentityTypeError = require('../../errors/UnknownIdentityTypeError');

/**
 * @typedef validateIdentityType
 * @param {number} type
 * @return ValidationResult
 */
function validateIdentityType(type) {
  const result = new ValidationResult();

  const isReservedType = type < Identity.MAX_RESERVED_TYPE;
  const isRegisteredType = Identity.TYPES_ENUM.includes(type);

  /* Check that identity type in the range that is reserved for internal usage,
  /* but is unknown for dpp */
  if (isReservedType && !isRegisteredType) {
    result.addError(new UnknownIdentityTypeError(type));
  }

  return result;
}

module.exports = validateIdentityType;
