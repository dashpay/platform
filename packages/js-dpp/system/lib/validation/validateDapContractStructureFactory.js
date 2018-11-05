const SchemaValidator = require('./SchemaValidator');

/**
 * @param validator
 * @return {validateDapContractStructure}
 */
module.exports = function validateDapContractStructureFactory(validator) {
  /**
   * @typedef validateDapContractStructure
   * @param {Object} rawDapContract
   * @return {Object[]}
   */
  function validateDapContractStructure(rawDapContract) {
    // TODO: Use validateSchema?

    return validator.validate(
      SchemaValidator.SCHEMAS.META.DAP_CONTRACT,
      rawDapContract,
    );
  }

  return validateDapContractStructure;
};
