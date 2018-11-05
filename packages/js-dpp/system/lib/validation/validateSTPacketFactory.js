const SchemaValidator = require('./SchemaValidator');

/**
 * @param {SchemaValidator} validator
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
   * @param {STPacket} stPacket
   * @param {DapContract} dapContract
   * @return {array}
   */
  function validateSTPacket(stPacket, dapContract) {
    // TODO Validate objects and contract once using schema
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
      errors = errors.concat(validateDapContract(dapContractInsidePacket));
    }

    return errors;
  }

  return validateSTPacket;
};
