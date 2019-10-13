const STPacket = require('../STPacket');

const stPacketBaseSchema = require('../../../schema/base/st-packet');
const stPacketHeaderSchema = require('../../../schema/st-packet-header');

const calculateItemsHash = require('../calculateItemsHash');
const calculateItemsMerkleRoot = require('../calculateItemsMerkleRoot');

const InvalidItemsHashError = require('../../errors/InvalidItemsHashError');
const InvalidItemsMerkleRootError = require('../../errors/InvalidItemsMerkleRootError');

/**
 * @param {JsonSchemaValidator} validator
 * @param {validateSTPacketContracts} validateSTPacketContracts
 * @param {validateSTPacketDocuments} validateSTPacketDocuments
 * @return {validateSTPacket}
 */
module.exports = function validateSTPacketFactory(
  validator,
  validateSTPacketContracts,
  validateSTPacketDocuments,
) {
  /**
   * @return {Object}
   */
  function createSTPacketSchema() {
    const stPacketSchema = Object.assign({}, stPacketBaseSchema);

    delete stPacketSchema.$id;

    stPacketSchema.properties = Object.assign(
      {},
      stPacketBaseSchema.properties,
      stPacketHeaderSchema.properties,
    );

    stPacketSchema.required = Array.from(stPacketBaseSchema.required)
      .concat(stPacketHeaderSchema.required);

    return stPacketSchema;
  }

  /**
   * @typedef validateSTPacket
   * @param {STPacket|RawSTPacket} stPacket
   * @param {DataContract} [dataContract]
   * @return {ValidationResult}
   */
  function validateSTPacket(stPacket, dataContract = undefined) {
    const rawSTPacket = (stPacket instanceof STPacket)
      ? stPacket.toJSON()
      : stPacket;

    const result = validator.validate(
      createSTPacketSchema(),
      rawSTPacket,
    );

    if (!result.isValid()) {
      return result;
    }

    if (rawSTPacket.contracts.length > 0) {
      result.merge(
        validateSTPacketContracts(rawSTPacket),
      );
    }

    if (rawSTPacket.documents.length > 0) {
      result.merge(
        validateSTPacketDocuments(rawSTPacket, dataContract),
      );
    }

    if (!result.isValid()) {
      return result;
    }

    if (calculateItemsMerkleRoot(rawSTPacket) !== rawSTPacket.itemsMerkleRoot) {
      result.addError(
        new InvalidItemsMerkleRootError(rawSTPacket),
      );
    }

    if (calculateItemsHash(rawSTPacket) !== rawSTPacket.itemsHash) {
      result.addError(
        new InvalidItemsHashError(rawSTPacket),
      );
    }

    return result;
  }

  return validateSTPacket;
};
