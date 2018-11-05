const SchemaValidator = require('../validation/SchemaValidator');

/**
 * @param {SchemaValidator} validator
 * @return {validateDapObjectStructure}
 */
module.exports = function validateDapObjectStructureFactory(validator) {
  /**
   * @typedef validateDapObjectStructure
   * @param {Object} rawDapObject
   * @return {Object[]}
   */
  function validateDapObjectStructure(rawDapObject) {
    return validator.validate(
      SchemaValidator.SCHEMAS.BASE.DAP_OBJECT,
      rawDapObject,
    );
  }

  return validateDapObjectStructure;
};
