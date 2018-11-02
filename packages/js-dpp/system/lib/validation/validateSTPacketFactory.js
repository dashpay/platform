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
    validator.setDapContract(dapContract);

    // TODO If contract contains wrong $schema?

    // TODO Validate objects and contract once using schema
    let errors;

    errors = validator.validate(
      SchemaValidator.SHEMAS.ST_PACKET,
      stPacket.toJSON(),
    );

    if (errors.length) {
      return errors;
    }

    for (const dapObject of stPacket.getDapObjects()) {
      errors = validateDapObject(dapObject, dapContract);

      if (errors.length) {
        errors = errors.concat(errors);
      }
    }

    const dapContractInsidePacket = stPacket.getDapContract();

    if (dapContractInsidePacket) {
      errors = errors.concat(validateDapContract(dapContractInsidePacket));
    }

    return errors;
  }

  return validateSTPacket;
};
