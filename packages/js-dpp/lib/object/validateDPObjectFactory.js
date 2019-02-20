const DPObject = require('./DPObject');

const dpObjectBaseSchema = require('../../schema/base/dp-object');

const ValidationResult = require('../validation/ValidationResult');

const InvalidDPObjectTypeError = require('../errors/InvalidDPObjectTypeError');
const MissingDPObjectTypeError = require('../errors/MissingDPObjectTypeError');
const MissingDPObjectActionError = require('../errors/MissingDPObjectActionError');
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

    if (!Object.prototype.hasOwnProperty.call(rawDPObject, '$action')) {
      result.addError(
        new MissingDPObjectActionError(rawDPObject),
      );

      return result;
    }

    if (!dpContract.isDPObjectDefined(rawDPObject.$type)) {
      result.addError(
        new InvalidDPObjectTypeError(rawDPObject.$type, dpContract),
      );

      return result;
    }

    if (rawDPObject.$action === DPObject.ACTIONS.DELETE) {
      const schemaValidationResult = validator.validate(
        dpObjectBaseSchema,
        rawDPObject,
      );

      result.merge(schemaValidationResult);
    } else {
      const dpObjectSchemaRef = dpContract.getDPObjectSchemaRef(rawDPObject.$type);

      const enrichedDPContract = enrichDPContractWithBaseDPObject(dpContract);

      const additionalSchemas = {
        [dpContract.getJsonSchemaId()]: enrichedDPContract,
      };

      const schemaValidationResult = validator.validate(
        dpObjectSchemaRef,
        rawDPObject,
        additionalSchemas,
      );

      result.merge(schemaValidationResult);
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
