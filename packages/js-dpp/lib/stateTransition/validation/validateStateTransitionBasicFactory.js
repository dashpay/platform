const ValidationResult = require('../../validation/ValidationResult');

const MissingStateTransitionTypeError = require('../../errors/consensus/basic/stateTransition/MissingStateTransitionTypeError');
const InvalidStateTransitionTypeError = require('../../errors/consensus/basic/stateTransition/InvalidStateTransitionTypeError');
const StateTransitionMaxSizeExceededError = require('../../errors/consensus/basic/stateTransition/StateTransitionMaxSizeExceededError');
const MaxEncodedBytesReachedError = require('../../util/errors/MaxEncodedBytesReachedError');

/**
 * @param {Object.<number, Function>} validationFunctionsByType
 * @param {createStateTransition} createStateTransition
 * @return {validateStateTransitionBasic}
 */
function validateStateTransitionBasicFactory(
  validationFunctionsByType,
  createStateTransition,
) {
  /**
   * @typedef validateStateTransitionBasic
   * @param {RawStateTransition} rawStateTransition
   * @param {StateTransitionExecutionContext} executionContext
   */
  async function validateStateTransitionBasic(rawStateTransition, executionContext) {
    const result = new ValidationResult();

    if (!Object.prototype.hasOwnProperty.call(rawStateTransition, 'type')) {
      result.addError(
        new MissingStateTransitionTypeError(),
      );

      return result;
    }

    if (!validationFunctionsByType[rawStateTransition.type]) {
      result.addError(
        new InvalidStateTransitionTypeError(rawStateTransition.type),
      );

      return result;
    }

    const validationFunction = validationFunctionsByType[rawStateTransition.type];

    result.merge(
      await validationFunction(rawStateTransition, executionContext),
    );

    if (!result.isValid()) {
      return result;
    }

    const stateTransition = await createStateTransition(rawStateTransition, executionContext);

    try {
      stateTransition.toBuffer();
    } catch (e) {
      if (e instanceof MaxEncodedBytesReachedError) {
        result.addError(
          new StateTransitionMaxSizeExceededError(
            Math.floor(e.getPayload().length / 1024),
            e.getMaxSizeKBytes(),
          ),
        );
      } else {
        throw e;
      }
    }

    return result;
  }

  return validateStateTransitionBasic;
}

module.exports = validateStateTransitionBasicFactory;
