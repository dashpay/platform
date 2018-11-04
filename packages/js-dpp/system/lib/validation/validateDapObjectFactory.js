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

    // TODO validate once
    errors = validator.validate(
      SchemaValidator.SCHEMAS.BASE.DAP_OBJECT,
      dapObject.toJSON(),
    );

    if (errors.length) {
      return errors;
    }

    try {
      errors = validator.validate(
        dapContract.getDapObjectSchemaRef(dapObject.getType()),
        dapObject.toJSON(),
        { 'dap-contract': dapContract.toJSON() },
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
