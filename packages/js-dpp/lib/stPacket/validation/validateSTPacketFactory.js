const SchemaValidator = require('../../validation/SchemaValidator');

const STPacket = require('../STPacket');

/**
 * @param {SchemaValidator} validator
 * @param {validateDapObject} validateDapObject
 * @param {validateDapContract} validateDapContract
 * @return {validateSTPacketHeader}
 */
module.exports = function validateSTPacketFactory(
  validator,
  validateDapObject,
  validateDapContract,
) {
  /**
   * @typedef validateSTPacket
   * @param {STPacket} stPacket
   * @param {DapContract} dapContract
   * @return {Object[]}
   */
  function validateSTPacket(stPacket, dapContract) {
    const rawStPacket = (dapContract instanceof STPacket)
      ? stPacket.toJSON()
      : stPacket;

    let errors;

    errors = validator.validate(
      SchemaValidator.SCHEMAS.ST_PACKET,
      rawStPacket,
    );

    if (errors.length) {
      return errors;
    }

    rawStPacket.objects.forEach((dapObject) => {
      const dapObjectErrors = validateDapObject(dapObject, dapContract);

      if (dapObjectErrors.length) {
        errors = errors.concat(dapObjectErrors);
      }
    });

    const [dapContractInsidePacket] = rawStPacket.contracts;

    if (dapContractInsidePacket) {
      if (stPacket.getDapContractId() !== dapContractInsidePacket.getId()) {
        return [{
          type: 'InvalidDapContractId',
        }];
      }

      const dapContractErrors = validateDapContract(dapContractInsidePacket.toJSON());

      if (dapContractErrors.length) {
        errors = errors.concat(dapContractErrors);
      }
    }

    return errors;
  }

  return validateSTPacket;
};
