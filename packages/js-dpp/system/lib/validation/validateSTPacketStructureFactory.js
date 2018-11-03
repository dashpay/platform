const SchemaValidator = require('./SchemaValidator');

/**
 * @param {SchemaValidator} validator
 * @return {validateSTPacket}
 */
module.exports = function validateStPacketStructureFactory(validator) {
  /**
   * @typedef validateSTPacketStructure
   * @param {Object} rawStPacket
   * @return {array}
   */
  function validateSTPacketStructure(rawStPacket) {
    return validator.validate(
      SchemaValidator.SCHEMAS.ST_PACKET,
      rawStPacket,
    );
  }

  return validateSTPacketStructure;
};
