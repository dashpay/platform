const ValidationResult = require('../../validation/ValidationResult');

const InvalidStateTransitionTypeError = require('../../errors/InvalidStateTransitionTypeError');

/**
 * @param {Object<number, Function>} validationFunctions
 * @return {validateStateTransitionData}
 */
function validateStateTransitionDataFactory(validationFunctions) {
  /**
   * @typedef validateStateTransitionData
   * @param {AbstractStateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateStateTransitionData(stateTransition) {
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

  return validateStateTransitionData;
}

module.exports = validateStateTransitionDataFactory;
