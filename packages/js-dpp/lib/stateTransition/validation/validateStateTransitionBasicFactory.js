const ValidationResult = require('../../validation/ValidationResult');

const MissingStateTransitionTypeError = require('../../errors/consensus/basic/stateTransition/MissingStateTransitionTypeError');
const InvalidStateTransitionTypeError = require('../../errors/consensus/basic/stateTransition/InvalidStateTransitionTypeError');
const StateTransitionMaxSizeExceededError = require('../../errors/consensus/basic/stateTransition/StateTransitionMaxSizeExceededError');
const MaxEncodedBytesReachedError = require('../../util/errors/MaxEncodedBytesReachedError');
const AbstractStateTransitionIdentitySigned = require('../AbstractStateTransitionIdentitySigned');

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
   */
  async function validateStateTransitionBasic(rawStateTransition) {
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
      await validationFunction(rawStateTransition),
    );

    if (!result.isValid()) {
      return result;
    }

    const stateTransition = await createStateTransition(rawStateTransition);

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
