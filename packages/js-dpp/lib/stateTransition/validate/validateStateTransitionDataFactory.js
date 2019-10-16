const types = require('../stateTransitionTypes');

const ValidationResult = require('../../validation/ValidationResult');

const InvalidStateTransitionTypeError = require('../../errors/InvalidStateTransitionTypeError');

/**
 * @param {validateDataContractSTData} validateDataContractSTData
 * @return {validateStateTransitionData}
 */
function validateStateTransitionDataFactory(validateDataContractSTData) {
  /**
   * @typedef validateStateTransitionData
   * @param {DataContractStateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateStateTransitionData(stateTransition) {
    const result = new ValidationResult();

    if (stateTransition.getType() === types.DATA_CONTRACT) {
      result.merge(
        await validateDataContractSTData(stateTransition),
      );
    } else {
      result.addError(
        new InvalidStateTransitionTypeError(stateTransition.toJSON()),
      );
    }

    return result;
  }

  return validateStateTransitionData;
}

module.exports = validateStateTransitionDataFactory;
