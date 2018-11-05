const InvalidDapObjectTypeError = require('../dapContract/errors/InvalidDapObjectTypeError');

/**
 * @param {SchemaValidator} validator
 * @param {Function} enrichDapContractWithBaseDapObject
 * @return {validateDapObject}
 */
module.exports = function validateDapObjectFactory(validator, enrichDapContractWithBaseDapObject) {
  /**
   * @typedef validateDapObject
   * @param {DapObject} dapObject
   * @param {DapContract} dapContract
   * @return {Object[]}
   */
  function validateDapObject(dapObject, dapContract) {
    let errors;

    const enrichedDapContract = enrichDapContractWithBaseDapObject(dapContract);

    try {
      errors = validator.validate(
        dapContract.getDapObjectSchemaRef(dapObject.getType()),
        dapObject.toJSON(),
        { [dapContract.getSchemaId()]: enrichedDapContract },
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
