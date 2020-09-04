const ValidationResult = require('../../../validation/ValidationResult');
const DataContractCreateTransition = require('../DataContractCreateTransition');
const DataContract = require('../../DataContract');

const InvalidDataContractEntropyError = require('../../../errors/InvalidDataContractEntropyError');
const InvalidDataContractIdError = require('../../../errors/InvalidDataContractIdError');

const entropy = require('../../../util/entropy');
const generateDataContractId = require('../../generateDataContractId');

/**
 * @param {validateDataContract} validateDataContract
 * @param {validateStateTransitionSignature} validateStateTransitionSignature
 * @param {validateIdentityExistence} validateIdentityExistence
 * @return {validateDataContractCreateTransitionStructure}
 */
function validateDataContractCreateTransitionStructureFactory(
  validateDataContract,
  validateStateTransitionSignature,
  validateIdentityExistence,
) {
  /**
   * @typedef validateDataContractCreateTransitionStructure
   * @param {RawDataContractCreateTransition} stateTransitionJson
   * @return {ValidationResult}
   */
  async function validateDataContractCreateTransitionStructure(stateTransitionJson) {
    const result = new ValidationResult();

    result.merge(
      await validateDataContract(stateTransitionJson.dataContract),
    );

    if (!result.isValid()) {
      return result;
    }

    const dataContract = new DataContract(stateTransitionJson.dataContract);
    const dataContractId = dataContract.getId();

    if (!entropy.validate(stateTransitionJson.entropy)) {
      result.addError(
        new InvalidDataContractEntropyError(stateTransitionJson.dataContract),
      );
      return result;
    }

    const generatedId = generateDataContractId(
      dataContract.getOwnerId(), stateTransitionJson.entropy,
    );

    if (generatedId !== dataContractId) {
      result.addError(
        new InvalidDataContractIdError(stateTransitionJson.dataContract),
      );
      return result;
    }

    // Data Contract identity must exists and confirmed
    result.merge(
      await validateIdentityExistence(dataContract.getOwnerId()),
    );

    if (!result.isValid()) {
      return result;
    }

    // Verify ST signature
    const stateTransition = DataContractCreateTransition.fromJSON(stateTransitionJson);

    result.merge(
      await validateStateTransitionSignature(stateTransition, dataContract.getOwnerId()),
    );

    return result;
  }

  return validateDataContractCreateTransitionStructure;
}

module.exports = validateDataContractCreateTransitionStructureFactory;
