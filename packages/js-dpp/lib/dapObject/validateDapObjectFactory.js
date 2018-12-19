const DapObject = require('./DapObject');

const ValidationResult = require('../validation/ValidationResult');

const InvalidDapObjectTypeError = require('../errors/InvalidDapObjectTypeError');
const MissingDapObjectTypeError = require('../errors/MissingDapObjectTypeError');
const InvalidDapObjectScopeIdError = require('../errors/InvalidDapObjectScopeIdError');

const entropy = require('../util/entropy');

/**
 * @param {JsonSchemaValidator} validator
 * @param {Function} enrichDapContractWithBaseDapObject
 * @return {validateDapObject}
 */
module.exports = function validateDapObjectFactory(
  validator,
  enrichDapContractWithBaseDapObject,
) {
  /**
   * @typedef validateDapObject
   * @param {Object|DapObject} dapObject
   * @param {DapContract} dapContract
   * @return {ValidationResult}
   */
  function validateDapObject(dapObject, dapContract) {
    const rawDapObject = (dapObject instanceof DapObject) ? dapObject.toJSON() : dapObject;

    const result = new ValidationResult();

    if (!Object.prototype.hasOwnProperty.call(rawDapObject, '$type')) {
      result.addError(
        new MissingDapObjectTypeError(rawDapObject),
      );

      return result;
    }

    const enrichedDapContract = enrichDapContractWithBaseDapObject(dapContract);

    let dapObjectSchemaRef;

    try {
      dapObjectSchemaRef = dapContract.getDapObjectSchemaRef(rawDapObject.$type);
    } catch (e) {
      if (!(e instanceof InvalidDapObjectTypeError)) {
        throw e;
      }

      result.addError(e);
    }

    if (dapObjectSchemaRef) {
      const additionalSchemas = {
        [dapContract.getSchemaId()]: enrichedDapContract,
      };

      const schemaResult = validator.validate(
        dapObjectSchemaRef,
        rawDapObject,
        additionalSchemas,
      );

      result.merge(schemaResult);
    }

    if (!entropy.validate(rawDapObject.$scopeId)) {
      result.addError(
        new InvalidDapObjectScopeIdError(rawDapObject),
      );
    }

    return result;
  }

  return validateDapObject;
};
