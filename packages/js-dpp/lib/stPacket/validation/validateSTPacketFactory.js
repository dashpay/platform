const STPacket = require('../STPacket');

const stPacketBaseSchema = require('../../../schema/base/st-packet');
const stPacketHeaderSchema = require('../../../schema/st-packet-header');

/**
 * @param {JsonSchemaValidator} validator
 * @param {validateSTPacketDapContracts} validateSTPacketDapContracts
 * @param {validateSTPacketDapObjects} validateSTPacketDapObjects
 * @return {validateSTPacket}
 */
module.exports = function validateSTPacketFactory(
  validator,
  validateSTPacketDapContracts,
  validateSTPacketDapObjects,
) {
  /**
   * @return {Object}
   */
  function createSTPacketSchema() {
    const stPacketSchema = Object.assign({ }, stPacketBaseSchema);

    delete stPacketSchema.$id;

    stPacketSchema.properties = Object.assign(
      { },
      stPacketBaseSchema.properties,
      stPacketHeaderSchema.properties,
    );

    stPacketSchema.required = Array.from(stPacketBaseSchema.required)
      .concat(stPacketHeaderSchema.required);

    return stPacketSchema;
  }

  /**
   * @typedef validateSTPacket
   * @param {STPacket|Object} stPacket
   * @param {DapContract} dapContract
   * @return {ValidationResult}
   */
  function validateSTPacket(stPacket, dapContract) {
    const rawStPacket = (stPacket instanceof STPacket)
      ? stPacket.toJSON()
      : stPacket;

    const result = validator.validate(
      createSTPacketSchema(),
      rawStPacket,
    );

    if (!result.isValid()) {
      return result;
    }

    // TODO Validate itemsHashes and itemsMerkleRoot

    if (rawStPacket.contracts.length > 0) {
      result.merge(
        validateSTPacketDapContracts(rawStPacket.contracts, rawStPacket),
      );
    }

    if (rawStPacket.objects.length > 0) {
      result.merge(
        validateSTPacketDapObjects(rawStPacket.objects, dapContract),
      );
    }

    return result;
  }

  return validateSTPacket;
};
