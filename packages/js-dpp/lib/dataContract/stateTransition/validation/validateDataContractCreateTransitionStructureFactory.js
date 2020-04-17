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
   * @param {RawDataContractCreateTransition} rawStateTransition
   * @return {ValidationResult}
   */
  async function validateDataContractCreateTransitionStructure(rawStateTransition) {
    const result = new ValidationResult();

    result.merge(
      await validateDataContract(rawStateTransition.dataContract),
    );

    if (!result.isValid()) {
      return result;
    }

    const dataContract = new DataContract(rawStateTransition.dataContract);
    const dataContractId = dataContract.getId();

    if (!entropy.validate(rawStateTransition.entropy)) {
      result.addError(
        new InvalidDataContractEntropyError(rawStateTransition.dataContract),
      );
      return result;
    }

    const generatedId = generateDataContractId(
      dataContract.getOwnerId(), rawStateTransition.entropy,
    );

    if (generatedId !== dataContractId) {
      result.addError(
        new InvalidDataContractIdError(rawStateTransition.dataContract),
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
    const stateTransition = new DataContractCreateTransition(rawStateTransition);

    result.merge(
      await validateStateTransitionSignature(stateTransition, dataContract.getOwnerId()),
    );

    return result;
  }

  return validateDataContractCreateTransitionStructure;
}

module.exports = validateDataContractCreateTransitionStructureFactory;
