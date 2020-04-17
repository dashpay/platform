const ValidationResult = require('../../../validation/ValidationResult');

const DataContractAlreadyPresentError = require('../../../errors/DataContractAlreadyPresentError');

/**
 *
 * @param {StateRepository} stateRepository
 * @return {validateDataContractCreateTransitionData}
 */
function validateDataContractCreateTransitionDataFactory(stateRepository) {
  /**
   * @typedef validateDataContractCreateTransitionData
   * @param {DataContractCreateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateDataContractCreateTransitionData(stateTransition) {
    const result = new ValidationResult();

    const dataContract = stateTransition.getDataContract();
    const dataContractId = dataContract.getId();

    // Data contract shouldn't exist
    const existingDataContract = await stateRepository.fetchDataContract(dataContractId);

    if (existingDataContract) {
      result.addError(
        new DataContractAlreadyPresentError(dataContract),
      );
    }

    return result;
  }

  return validateDataContractCreateTransitionData;
}

module.exports = validateDataContractCreateTransitionDataFactory;
