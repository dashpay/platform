const mergeWith = require('lodash.mergewith');

const AbstractStateTransition = require('../AbstractStateTransition');

const ValidationResult = require('../../validation/ValidationResult');

const MissingStateTransitionTypeError = require('../../errors/MissingStateTransitionTypeError');
const InvalidStateTransitionTypeError = require('../../errors/InvalidStateTransitionTypeError');
const StateTransitionMaxSizeExceededError = require('../../errors/StateTransitionMaxSizeExceededError');
const MaxEncodedBytesReachedError = require('../../util/errors/MaxEncodedBytesReachedError');

const baseSchema = require('../../../schema/stateTransition/stateTransitionBase');

/**
 * @param {JsonSchemaValidator} validator
 * @param {Object.<number, {validationFunction: Function, schema: Object}>} typeExtensions
 * @param {createStateTransition} createStateTransition
 * @return {validateStateTransitionStructure}
 */
function validateStateTransitionStructureFactory(validator, typeExtensions, createStateTransition) {
  /**
   * @typedef validateStateTransitionStructure
   * @param {
   * RawDataContractCreateTransition
   * |DataContractCreateTransition
   * |RawDocumentsBatchTransition|
   * DocumentsBatchTransition} stateTransition
   */
  async function validateStateTransitionStructure(stateTransition) {
    let rawStateTransition;
    let stateTransitionModel;

    if (stateTransition instanceof AbstractStateTransition) {
      rawStateTransition = stateTransition.toJSON();
      stateTransitionModel = stateTransition;
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

    if (!result.isValid()) {
      return result;
    }

    if (!stateTransitionModel) {
      stateTransitionModel = createStateTransition(rawStateTransition);
    }

    try {
      stateTransitionModel.serialize();
    } catch (e) {
      if (e instanceof MaxEncodedBytesReachedError) {
        result.addError(
          new StateTransitionMaxSizeExceededError(rawStateTransition, e.getMaxSizeKBytes()),
        );
      } else {
        throw e;
      }
    }

    return result;
  }

  return validateStateTransitionStructure;
}

module.exports = validateStateTransitionStructureFactory;
