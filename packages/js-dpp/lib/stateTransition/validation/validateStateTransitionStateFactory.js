const InvalidStateTransitionTypeError = require('../errors/InvalidStateTransitionTypeError');

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
    const validationFunction = validationFunctions[stateTransition.getType()];

    if (!validationFunction) {
      throw new InvalidStateTransitionTypeError(stateTransition.getType());
    }

    return validationFunction(stateTransition);
  }

  return validateStateTransitionState;
}

module.exports = validateStateTransitionStateFactory;
