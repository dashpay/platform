const SchemaValidator = require('../../validation/SchemaValidator');

/**
 * @param {SchemaValidator} validator
 * @param {validateDapObject} validateDapObject
 * @param {validateDapContractStructure} validateDapContractStructure
 * @return {validateSTPacket}
 */
module.exports = function validateSTPacketFactory(
  validator,
  validateDapObject,
  validateDapContractStructure,
) {
  /**
   * @typedef validateSTPacket
   * @param {STPacket} stPacket
   * @param {DapContract} dapContract
   * @return {Object[]}
   */
  function validateSTPacket(stPacket, dapContract) {
    let errors;

    errors = validator.validate(
      SchemaValidator.SCHEMAS.ST_PACKET,
      stPacket.toJSON(),
    );

    if (errors.length) {
      return errors;
    }

    stPacket.getDapObjects().forEach((dapObject) => {
      const dapObjectErrors = validateDapObject(dapObject, dapContract);

      if (dapObjectErrors.length) {
        errors = errors.concat(dapObjectErrors);
      }
    });

    const dapContractInsidePacket = stPacket.getDapContract();

    if (dapContractInsidePacket) {
      if (stPacket.getDapContractId() !== dapContractInsidePacket.getId()) {
        return [{
          type: 'InvalidDapContractId',
        }];
      }

      const dapContractErrors = validateDapContractStructure(dapContractInsidePacket.toJSON());

      if (dapContractErrors.length) {
        errors = errors.concat(dapContractErrors);
      }
    }

    return errors;
  }

  return validateSTPacket;
};
