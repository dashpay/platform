const STPacketHeader = require('./STPacketHeader');

const JsonSchemaValidator = require('../validation/JsonSchemaValidator');

/**
 * @param {JsonSchemaValidator} validator
 * @return {validateSTPacketHeader}
 */
module.exports = function validateSTPacketHeaderFactory(validator) {
  /**
   * @typedef validateSTPacketHeader
   * @param {STPacketHeader|RawSTPacketHeader} stPacketHeader
   * @return {ValidationResult}
   */
  function validateSTPacketHeader(stPacketHeader) {
    const rawStPacketHeader = (stPacketHeader instanceof STPacketHeader)
      ? stPacketHeader.toJSON()
      : stPacketHeader;

    return validator.validate(
      JsonSchemaValidator.SCHEMAS.ST_PACKET_HEADER,
      rawStPacketHeader,
    );
  }

  return validateSTPacketHeader;
};
