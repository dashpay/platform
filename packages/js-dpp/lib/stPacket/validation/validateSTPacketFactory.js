const JsonSchemaValidator = require('../../validation/JsonSchemaValidator');

const STPacket = require('../STPacket');

const ConsensusError = require('../../consensusErrors/ConsensusError');

/**
 * @param {JsonSchemaValidator} validator
 * @param {validateDapObject} validateDapObject
 * @param {validateDapContract} validateDapContract
 * @return {validateSTPacket}
 */
module.exports = function validateSTPacketFactory(
  validator,
  validateDapObject,
  validateDapContract,
) {
  /**
   * @typedef validateSTPacket
   * @param {STPacket|Object} stPacket
   * @param {DapContract} dapContract
   * @return {ValidationResult}
   */
  function validateSTPacket(stPacket, dapContract) {
    const rawStPacket = (dapContract instanceof STPacket)
      ? stPacket.toJSON()
      : stPacket;

    const result = validator.validate(
      JsonSchemaValidator.SCHEMAS.ST_PACKET,
      rawStPacket,
    );

    if (!result.isValid()) {
      return result;
    }

    rawStPacket.objects.forEach((dapObject) => {
      result.merge(
        validateDapObject(dapObject, dapContract),
      );
    });

    const [dapContractInsidePacket] = rawStPacket.contracts;

    if (dapContractInsidePacket) {
      if (stPacket.getDapContractId() !== dapContractInsidePacket.getId()) {
        result.addError(
          new ConsensusError('InvalidDapContractId'),
        );
      }

      result.merge(
        validateDapContract(dapContractInsidePacket.toJSON()),
      );
    }

    // TODO Validate itemsHashes and itemsMerkleRoot

    return result;
  }

  return validateSTPacket;
};
