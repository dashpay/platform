const ValidationResult = require('../../validation/ValidationResult');

const InvalidStateTransitionTypeError = require('../../errors/InvalidStateTransitionTypeError');

/**
 * @param {Object<number, Function>} validationFunctions
 * @return {validateStateTransitionState}
 */
function validateStateTransitionStateFactory(validationFunctions) {
  /**
   * @typedef {validateStateTransitionState}
   * @param {AbstractStateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateStateTransitionState(stateTransition) {
    const result = new ValidationResult();

    const validationFunction = validationFunctions[stateTransition.getType()];

    if (!validationFunction) {
      result.addError(
        new InvalidStateTransitionTypeError(stateTransition.toObject()),
      );

      return result;
    }

    return validationFunction(stateTransition);
  }

  return validateStateTransitionState;
}

module.exports = validateStateTransitionStateFactory;
