const InvalidDapObjectTypeError = require('../errors/InvalidDapObjectTypeError');

const DapObject = require('./DapObject');

const ValidationResult = require('../validation/ValidationResult');

const MissingDapObjectTypeError = require('../errors/MissingDapObjectTypeError');

/**
 * @param {JsonSchemaValidator} validator
 * @param {Function} enrichDapContractWithBaseDapObject
 * @return {validateDapObject}
 */
module.exports = function validateDapObjectFactory(validator, enrichDapContractWithBaseDapObject) {
  /**
   * @typedef validateDapObject
   * @param {Object|DapObject} dapObject
   * @param {DapContract} dapContract
   * @return {ValidationResult}
   */
  function validateDapObject(dapObject, dapContract) {
    const rawDapObject = (dapObject instanceof DapObject) ? dapObject.toJSON() : dapObject;

    if (!Object.prototype.hasOwnProperty.call(rawDapObject, '$type')) {
      return new ValidationResult([
        new MissingDapObjectTypeError(),
      ]);
    }

    const enrichedDapContract = enrichDapContractWithBaseDapObject(dapContract);

    try {
      return validator.validate(
        dapContract.getDapObjectSchemaRef(rawDapObject.$type),
        rawDapObject,
        { [dapContract.getSchemaId()]: enrichedDapContract },
      );
    } catch (e) {
      if (e instanceof InvalidDapObjectTypeError) {
        return new ValidationResult([e]);
      }

      throw e;
    }

    // TODO: Validate scope and scopeId
  }

  return validateDapObject;
};
