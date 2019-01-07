const JsonSchemaValidator = require('../validation/JsonSchemaValidator');

const DPContract = require('./DPContract');

/**
 * @param validator
 * @return {validateDPContract}
 */
module.exports = function validateDPContractFactory(validator) {
  /**
   * @typedef validateDPContract
   * @param {DPContract|Object} dpContract
   * @return {ValidationResult}
   */
  function validateDPContract(dpContract) {
    const rawDPContract = (dpContract instanceof DPContract)
      ? dpContract.toJSON()
      : dpContract;

    // TODO: Use validateSchema
    //  https://github.com/epoberezkin/ajv#validateschemaobject-schema---boolean

    return validator.validate(
      JsonSchemaValidator.SCHEMAS.META.DP_CONTRACT,
      rawDPContract,
    );
  }

  return validateDPContract;
};
