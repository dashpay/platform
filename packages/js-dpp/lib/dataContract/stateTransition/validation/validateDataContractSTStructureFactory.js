const ValidationResult = require('../../../validation/ValidationResult');
const DataContractStateTransition = require('../DataContractStateTransition');
const Identity = require('../../../identity/Identity');

/**
 * @param {validateDataContract} validateDataContract
 * @param {validateStateTransitionSignature} validateStateTransitionSignature
 * @param {createDataContract} createDataContract
 * @param {validateIdentityExistenceAndType} validateIdentityExistenceAndType
 * @return {validateDataContractSTStructure}
 */
function validateDataContractSTStructureFactory(
  validateDataContract,
  validateStateTransitionSignature,
  createDataContract,
  validateIdentityExistenceAndType,
) {
  /**
   * @typedef validateDataContractSTStructure
   * @param {RawDataContractStateTransition} rawStateTransition
   * @return {ValidationResult}
   */
  async function validateDataContractSTStructure(rawStateTransition) {
    const result = new ValidationResult();

    result.merge(
      await validateDataContract(rawStateTransition.dataContract),
    );

    if (!result.isValid()) {
      return result;
    }

    const dataContract = createDataContract(rawStateTransition.dataContract);
    const dataContractId = dataContract.getId();

    // Data Contract identity must exists and confirmed
    result.merge(
      await validateIdentityExistenceAndType(dataContractId, [Identity.TYPES.APPLICATION]),
    );

    if (!result.isValid()) {
      return result;
    }

    // Verify ST signature
    const stateTransition = new DataContractStateTransition(dataContract);

    stateTransition
      .setSignature(rawStateTransition.signature)
      .setSignaturePublicKeyId(rawStateTransition.signaturePublicKeyId);

    result.merge(
      await validateStateTransitionSignature(stateTransition, dataContractId),
    );

    return result;
  }

  return validateDataContractSTStructure;
}

module.exports = validateDataContractSTStructureFactory;
