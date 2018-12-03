const JsonSchemaValidator = require('../validation/JsonSchemaValidator');

const DapContract = require('./DapContract');

/**
 * @param validator
 * @return {validateDapContract}
 */
module.exports = function validateDapContractFactory(validator) {
  /**
   * @typedef validateDapContract
   * @param {DapContract|Object} dapContract
   * @return {ValidationResult}
   */
  function validateDapContract(dapContract) {
    const rawDapContract = (dapContract instanceof DapContract)
      ? dapContract.toJSON()
      : dapContract;

    // TODO: Use validateSchema
    //  https://github.com/epoberezkin/ajv#validateschemaobject-schema---boolean

    return validator.validate(
      JsonSchemaValidator.SCHEMAS.META.DAP_CONTRACT,
      rawDapContract,
    );
  }

  return validateDapContract;
};
