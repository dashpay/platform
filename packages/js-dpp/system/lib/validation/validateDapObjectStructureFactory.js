const SchemaValidator = require('./SchemaValidator');

/**
 * @param {SchemaValidator} validator
 * @return {validateDapObjectStructure}
 */
module.exports = function validateDapObjectStructureFactory(validator) {
  /**
   * @typedef validateDapObjectStructure
   * @param {Object} rawDapObject
   * @return {array}
   */
  function validateDapObjectStructure(rawDapObject) {
    return validator.validate(
      SchemaValidator.SHEMAS.BASE.DAP_OBJECT,
      rawDapObject,
    );
  }

  return validateDapObjectStructure;
};
