const DPObject = require('./DPObject');

const ValidationResult = require('../validation/ValidationResult');

const InvalidDPObjectTypeError = require('../errors/InvalidDPObjectTypeError');
const MissingDPObjectTypeError = require('../errors/MissingDPObjectTypeError');
const InvalidDPObjectScopeIdError = require('../errors/InvalidDPObjectScopeIdError');

const entropy = require('../util/entropy');

/**
 * @param {JsonSchemaValidator} validator
 * @param {enrichDPContractWithBaseDPObject} enrichDPContractWithBaseDPObject
 * @return {validateDPObject}
 */
module.exports = function validateDPObjectFactory(
  validator,
  enrichDPContractWithBaseDPObject,
) {
  /**
   * @typedef validateDPObject
   * @param {Object|DPObject} dpObject
   * @param {DPContract} dpContract
   * @return {ValidationResult}
   */
  function validateDPObject(dpObject, dpContract) {
    const rawDPObject = (dpObject instanceof DPObject) ? dpObject.toJSON() : dpObject;

    const result = new ValidationResult();

    if (!Object.prototype.hasOwnProperty.call(rawDPObject, '$type')) {
      result.addError(
        new MissingDPObjectTypeError(rawDPObject),
      );

      return result;
    }

    const enrichedDPContract = enrichDPContractWithBaseDPObject(dpContract);

    let dpObjectSchemaRef;

    try {
      dpObjectSchemaRef = dpContract.getDPObjectSchemaRef(rawDPObject.$type);
    } catch (e) {
      if (!(e instanceof InvalidDPObjectTypeError)) {
        throw e;
      }

      result.addError(e);
    }

    if (dpObjectSchemaRef) {
      const additionalSchemas = {
        [dpContract.getJsonSchemaId()]: enrichedDPContract,
      };

      const schemaResult = validator.validate(
        dpObjectSchemaRef,
        rawDPObject,
        additionalSchemas,
      );

      result.merge(schemaResult);
    }

    if (!entropy.validate(rawDPObject.$scopeId)) {
      result.addError(
        new InvalidDPObjectScopeIdError(rawDPObject),
      );
    }

    return result;
  }

  return validateDPObject;
};
