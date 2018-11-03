const SchemaValidator = require('./SchemaValidator');

/**
 * @param validator
 * @return {validateDapContract}
 */
module.exports = function validateDapContractFactory(validator) {
  /**
   * @typedef validateDapContract
   * @param {DapContract} dapContract
   * @return {array}
   */
  function validateDapContract(dapContract) {
    // TODO: Use validateSchema?

    return validator.validate(SchemaValidator.SCHEMAS.META.DAP_CONTRACT, dapContract.toJSON());
  }

  return validateDapContract;
};
