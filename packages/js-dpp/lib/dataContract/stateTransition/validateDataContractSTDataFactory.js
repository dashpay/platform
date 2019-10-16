const ValidationResult = require('../../validation/ValidationResult');

const DataContractAlreadyPresentError = require('../../errors/DataContractAlreadyPresentError');

const DataContractIdentityNotFoundError = require('../../errors/DataContractIdentityNotFoundError');

const UnconfirmedUserError = require('../../errors/UnconfirmedUserError');

const MIN_CONFIRMATIONS = 6;

/**
 *
 * @param {DataProvider} dataProvider
 * @return {validateDataContractSTData}
 */
function validateDataContractSTDataFactory(dataProvider) {
  /**
   * @typedef validateDataContractSTData
   * @param {DataContractStateTransition} stateTransition
   * @return {ValidationResult}
   */
  async function validateDataContractSTData(stateTransition) {
    const result = new ValidationResult();

    const dataContract = stateTransition.getDataContract();
    const dataContractId = dataContract.getId();

    const blockchainUserTransition = await dataProvider.fetchTransaction(dataContractId);

    if (!blockchainUserTransition) {
      result.addError(
        new DataContractIdentityNotFoundError(dataContractId),
      );

      return result;
    }

    if (blockchainUserTransition.confirmations < MIN_CONFIRMATIONS) {
      result.addError(
        new UnconfirmedUserError(blockchainUserTransition),
      );

      return result;
    }

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
