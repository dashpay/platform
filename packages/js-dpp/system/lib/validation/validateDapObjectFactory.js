const SchemaValidator = require('./SchemaValidator');
const InvalidDapObjectTypeError = require('../errors/InvalidDapObjectTypeError');

/**
 * @param {SchemaValidator} validator
 * @return {validateDapObject}
 */
module.exports = function validateDapObjectFactory(validator) {
  /**
   * @typedef validateDapObject
   * @param {DapObject} dapObject
   * @param {DapContract} dapContract
   * @return {array}
   */
  function validateDapObject(dapObject, dapContract) {
    let errors;

    validator.setDapContract(dapContract);

    // TODO validate once
    errors = validator.validate(
      SchemaValidator.SHEMAS.BASE.DAP_OBJECT,
      dapObject.toJSON(),
    );

    if (errors.length) {
      return errors;
    }

    try {
      errors = validator.validate(
        dapContract.getDapObjectSchema(dapObject.getType()),
        dapObject.toJSON(),
      );
    } catch (e) {
      if (e instanceof InvalidDapObjectTypeError) {
        return [e];
      }

      throw e;
    }

    return errors;
  }

  return validateDapObject;
};
