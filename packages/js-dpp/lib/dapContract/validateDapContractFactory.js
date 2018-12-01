const SchemaValidator = require('../validation/SchemaValidator');

const DapContract = require('./DapContract');

/**
 * @param validator
 * @return {validateDapContract}
 */
module.exports = function validateDapContractFactory(validator) {
  /**
   * @typedef validateDapContract
   * @param {Object|DapContract} dapContract
   * @return {Object[]}
   */
  function validateDapContract(dapContract) {
    // TODO: Use validateSchema?

    const rawDapContract = (dapContract instanceof DapContract)
      ? dapContract.toJSON()
      : dapContract;

    return validator.validate(
      SchemaValidator.SCHEMAS.META.DAP_CONTRACT,
      rawDapContract,
    );
  }

  return validateDapContract;
};
