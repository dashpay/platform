const SchemaValidator = require('../validation/SchemaValidator');

/**
 * @param validator
 * @return {validateDapContractStructure}
 */
module.exports = function validateDapContractFactory(validator) {
  /**
   * @typedef validateDapContract
   * @param {Object} rawDapContract
   * @return {Object[]}
   */
  function validateDapContract(rawDapContract) {
    // TODO: Use validateSchema?

    return validator.validate(
      SchemaValidator.SCHEMAS.META.DAP_CONTRACT,
      rawDapContract,
    );
  }

  return validateDapContract;
};
