const ValidationResult = require('../../../validation/ValidationResult');

/**
 * @param {validateDataContract} validateDataContract
 * @return {validateDataContractSTStructure}
 */
function validateDataContractSTStructureFactory(validateDataContract) {
  /**
   * @typedef validateDataContractSTStructure
   * @param {RawDataContractStateTransition} rawStateTransition
   * @return {ValidationResult}
   */
  function validateDataContractSTStructure(rawStateTransition) {
    const result = new ValidationResult();

    result.merge(
      validateDataContract(rawStateTransition.dataContract),
    );

    return result;
  }

  return validateDataContractSTStructure;
}

module.exports = validateDataContractSTStructureFactory;
