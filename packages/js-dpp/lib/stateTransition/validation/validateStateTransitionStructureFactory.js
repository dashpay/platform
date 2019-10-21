const mergeWith = require('lodash.mergewith');

const AbstractStateTransition = require('../AbstractStateTransition');

const ValidationResult = require('../../validation/ValidationResult');

const MissingStateTransitionTypeError = require('../../errors/MissingStateTransitionTypeError');
const InvalidStateTransitionTypeError = require('../../errors/InvalidStateTransitionTypeError');

const baseSchema = require('../../../schema/stateTransition/base');

/**
 * @param {JsonSchemaValidator} validator
 * @param {Object.<number, {validationFunction: Function, schema: Object}>} typeExtensions
 * @return {validateStateTransitionStructure}
 */
function validateStateTransitionStructureFactory(validator, typeExtensions) {
  /**
   * @typedef validateStateTransitionStructure
   * @param {
   * RawDataContractStateTransition
   * |DataContractStateTransition
   * |RawDocumentsStateTransition|
   * DocumentsStateTransition} stateTransition
   */
  async function validateStateTransitionStructure(stateTransition) {
    let rawStateTransition;

    if (stateTransition instanceof AbstractStateTransition) {
      rawStateTransition = stateTransition.toJSON();
    } else {
      rawStateTransition = stateTransition;
    }

    const result = new ValidationResult();

    if (!Object.prototype.hasOwnProperty.call(rawStateTransition, 'type')) {
      result.addError(
        new MissingStateTransitionTypeError(rawStateTransition),
      );

      return result;
    }

    if (!typeExtensions[rawStateTransition.type]) {
      result.addError(
        new InvalidStateTransitionTypeError(rawStateTransition),
      );

      return result;
    }

    const { validationFunction, schema } = typeExtensions[rawStateTransition.type];

    const extendedSchema = mergeWith({}, baseSchema, schema, (objValue, srcValue) => (
      Array.isArray(objValue) ? objValue.concat(srcValue) : undefined
    ));

    result.merge(
      validator.validate(
        extendedSchema,
        rawStateTransition,
      ),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      await validationFunction(rawStateTransition),
    );

    return result;
  }

  return validateStateTransitionStructure;
}

module.exports = validateStateTransitionStructureFactory;
