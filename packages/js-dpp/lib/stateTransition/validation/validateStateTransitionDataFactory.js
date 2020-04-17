const ValidationResult = require('../../validation/ValidationResult');

const InvalidStateTransitionTypeError = require('../../errors/InvalidStateTransitionTypeError');

/**
 * @param {Object<number, Function>} validationFunctions
 * @return {validateStateTransitionData}
 */
function validateStateTransitionDataFactory(validationFunctions) {
  /**
   * @typedef validateStateTransitionData
   * @param {DataContractCreateTransition|DocumentsBatchTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateStateTransitionData(stateTransition) {
    const result = new ValidationResult();

    const validationFunction = validationFunctions[stateTransition.getType()];

    if (!validationFunction) {
      result.addError(
        new InvalidStateTransitionTypeError(stateTransition.toJSON()),
      );

      return result;
    }

    return validationFunction(stateTransition);
  }

  return validateStateTransitionData;
}

module.exports = validateStateTransitionDataFactory;
