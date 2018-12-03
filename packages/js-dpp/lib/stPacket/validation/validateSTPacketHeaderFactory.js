const JsonSchemaValidator = require('../../validation/JsonSchemaValidator');

/**
 * @param {JsonSchemaValidator} validator
 * @return {validateSTPacketHeader}
 */
module.exports = function validateSTPacketHeaderFactory(validator) {
  /**
   * @typedef validateSTPacketHeader
   * @param {Object} rawStPacketHeader
   * @return {ValidationResult}
   */
  function validateSTPacketHeader(rawStPacketHeader) {
    return validator.validate(
      JsonSchemaValidator.SCHEMAS.ST_PACKET_HEADER,
      rawStPacketHeader,
    );
  }

  return validateSTPacketHeader;
};
