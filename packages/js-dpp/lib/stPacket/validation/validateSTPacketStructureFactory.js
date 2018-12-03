const JsonSchemaValidator = require('../../validation/JsonSchemaValidator');

/**
 * @param {JsonSchemaValidator} validator
 * @return {validateSTPacketStructure}
 */
module.exports = function validateSTPacketStructureFactory(validator) {
  /**
   * @typedef validateSTPacketStructure
   * @param {Object} rawStPacket
   * @return {ValidationResult}
   */
  function validateSTPacketStructure(rawStPacket) {
    return validator.validate(
      JsonSchemaValidator.SCHEMAS.ST_PACKET,
      rawStPacket,
    );
  }

  return validateSTPacketStructure;
};
