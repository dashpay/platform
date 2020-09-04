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
function validateStateTransitionStructureFactory(
  validator,
  typeExtensions,
  createStateTransition,
) {
  /**
   * @typedef validateStateTransitionStructure
   * @param {
   * RawDataContractCreateTransition
   * |DataContractCreateTransition
   * |RawDocumentsBatchTransition|
   * DocumentsBatchTransition} stateTransition
   */
  async function validateStateTransitionStructure(stateTransition) {
    let stateTransitionJson;
    let stateTransitionModel;

    if (stateTransition instanceof AbstractStateTransition) {
      stateTransitionJson = stateTransition.toJSON();
      stateTransitionModel = stateTransition;
    } else {
      stateTransitionJson = stateTransition;
    }

    const result = new ValidationResult();

    if (!Object.prototype.hasOwnProperty.call(stateTransitionJson, 'type')) {
      result.addError(
        new MissingStateTransitionTypeError(stateTransitionJson),
      );

      return result;
    }

    if (!typeExtensions[stateTransitionJson.type]) {
      result.addError(
        new InvalidStateTransitionTypeError(stateTransitionJson),
      );

      return result;
    }

    const { validationFunction, schema } = typeExtensions[stateTransitionJson.type];

    const extendedSchema = mergeWith({}, baseSchema, schema, (objValue, srcValue) => (
      Array.isArray(objValue) ? objValue.concat(srcValue) : undefined
    ));

    result.merge(
      validator.validate(
        extendedSchema,
        stateTransitionJson,
      ),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      await validationFunction(stateTransitionJson),
    );

    if (!result.isValid()) {
      return result;
    }

    if (!stateTransitionModel) {
      stateTransitionModel = await createStateTransition(stateTransitionJson, {
        fromJSON: true,
      });
    }

    try {
      stateTransitionModel.serialize();
    } catch (e) {
      if (e instanceof MaxEncodedBytesReachedError) {
        result.addError(
          new StateTransitionMaxSizeExceededError(stateTransitionJson, e.getMaxSizeKBytes()),
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
