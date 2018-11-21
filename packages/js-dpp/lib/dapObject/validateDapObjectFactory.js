const InvalidDapObjectTypeError = require('../dapContract/errors/InvalidDapObjectTypeError');

/**
 * @param {SchemaValidator} validator
 * @param {Function} enrichDapContractWithBaseDapObject
 * @return {validateDapObject}
 */
module.exports = function validateDapObjectFactory(validator, enrichDapContractWithBaseDapObject) {
  /**
   * @typedef validateDapObject
   * @param {Object} rawDapObject
   * @param {DapContract} dapContract
   * @return {Object[]}
   */
  function validateDapObject(rawDapObject, dapContract) {
    let errors;

    // TODO consensus error + test
    if (Object.prototype.hasOwnProperty.call(rawDapObject, '$type')) {
      return [new Error('$type is not present')];
    }

    const enrichedDapContract = enrichDapContractWithBaseDapObject(dapContract);

    try {
      errors = validator.validate(
        dapContract.getDapObjectSchemaRef(rawDapObject.$type),
        rawDapObject,
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
