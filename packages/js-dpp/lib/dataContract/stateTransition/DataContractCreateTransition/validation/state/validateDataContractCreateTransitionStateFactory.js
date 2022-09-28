const ValidationResult = require('../../../../../validation/ValidationResult');

const DataContractAlreadyPresentError = require('../../../../../errors/consensus/state/dataContract/DataContractAlreadyPresentError');

/**
 *
 * @param {StateRepository} stateRepository
 * @return {validateDataContractCreateTransitionState}
 */
function validateDataContractCreateTransitionStateFactory(stateRepository) {
  /**
   * @typedef validateDataContractCreateTransitionState
   * @param {DataContractCreateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateDataContractCreateTransitionState(stateTransition) {
    const result = new ValidationResult();

    const executionContext = stateTransition.getExecutionContext();
    const dataContract = stateTransition.getDataContract();
    const dataContractId = dataContract.getId();

    // Data contract shouldn't exist
    const existingDataContract = await stateRepository.fetchDataContract(
      dataContractId,
      executionContext,
    );

    if (executionContext.isDryRun()) {
      return result;
    }

    if (existingDataContract) {
      result.addError(
        new DataContractAlreadyPresentError(dataContractId.toBuffer()),
      );
    }

    return result;
  }

  return validateDataContractCreateTransitionState;
}

module.exports = validateDataContractCreateTransitionStateFactory;
