const ValidationResult = require('../../../validation/ValidationResult');

const DataContractAlreadyPresentError = require('../../../errors/DataContractAlreadyPresentError');

/**
 *
 * @param {DataProvider} dataProvider
 * @param {validateBlockchainUser} validateBlockchainUser
 * @return {validateDataContractSTData}
 */
function validateDataContractSTDataFactory(dataProvider, validateBlockchainUser) {
  /**
   * @typedef validateDataContractSTData
   * @param {DataContractStateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateDataContractSTData(stateTransition) {
    const result = new ValidationResult();

    const dataContract = stateTransition.getDataContract();
    const dataContractId = dataContract.getId();

    // Data Contract identity must exists and confirmed
    result.merge(
      await validateBlockchainUser(dataContractId),
    );

    if (!result.isValid()) {
      return result;
    }

    // Data contract shouldn't exist
    const existingDataContract = await dataProvider.fetchDataContract(dataContractId);

    if (existingDataContract) {
      result.addError(
        new DataContractAlreadyPresentError(dataContract),
      );
    }

    return result;
  }

  return validateDataContractSTData;
}

module.exports = validateDataContractSTDataFactory;
