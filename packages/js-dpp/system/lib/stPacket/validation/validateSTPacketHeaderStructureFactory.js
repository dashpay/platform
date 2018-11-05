const SchemaValidator = require('../../validation/SchemaValidator');

/**
 * @param {SchemaValidator} validator
 * @return {validateSTPacketHeaderStructure}
 */
module.exports = function validateSTPacketHeaderStructureFactory(validator) {
  /**
   * @typedef validateSTPacketHeaderStructure
   * @param {Object} rawStPacketHeader
   * @return {Object[]}
   */
  function validateSTPacketHeaderStructure(rawStPacketHeader) {
    return validator.validate(
      SchemaValidator.SCHEMAS.ST_PACKET_HEADER,
      rawStPacketHeader,
    );
  }

  return validateSTPacketHeaderStructure;
};
