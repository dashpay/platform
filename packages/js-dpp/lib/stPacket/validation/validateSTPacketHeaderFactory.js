const SchemaValidator = require('../../validation/SchemaValidator');

/**
 * @param {SchemaValidator} validator
 * @return {validateSTPacketHeader}
 */
module.exports = function validateSTPacketHeaderFactory(validator) {
  /**
   * @typedef validateSTPacketHeader
   * @param {Object} rawStPacketHeader
   * @return {Object[]}
   */
  function validateSTPacketHeader(rawStPacketHeader) {
    return validator.validate(
      SchemaValidator.SCHEMAS.ST_PACKET_HEADER,
      rawStPacketHeader,
    );
  }

  return validateSTPacketHeader;
};
