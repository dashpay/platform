const DataContractCreateTransition = require('../DataContractCreateTransition');

const InvalidDataContractIdError = require('../../../errors/InvalidDataContractIdError');

const generateDataContractId = require('../../generateDataContractId');

const dataContractCreateTransitionSchema = require('../../../../schema/dataContract/stateTransition/dataContractCreate');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validateDataContract} validateDataContract
 * @param {validateStateTransitionSignature} validateStateTransitionSignature
 * @param {validateIdentityExistence} validateIdentityExistence
 * @return {validateDataContractCreateTransitionStructure}
 */
function validateDataContractCreateTransitionStructureFactory(
  jsonSchemaValidator,
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
    const result = jsonSchemaValidator.validate(
      dataContractCreateTransitionSchema,
      rawStateTransition,
    );

    if (!result.isValid()) {
      return result;
    }

    // Validate Data Contract
    const rawDataContract = rawStateTransition.dataContract;

    result.merge(
      await validateDataContract(rawDataContract),
    );

    if (!result.isValid()) {
      return result;
    }

    // Validate Data Contract ID
    const generatedId = generateDataContractId(
      rawDataContract.ownerId, rawStateTransition.entropy,
    );

    if (!generatedId.equals(rawDataContract.$id)) {
      result.addError(
        new InvalidDataContractIdError(rawDataContract),
      );

      return result;
    }

    // Data Contract identity must exists and confirmed
    result.merge(
      await validateIdentityExistence(rawDataContract.ownerId),
    );

    if (!result.isValid()) {
      return result;
    }

    // Verify ST signature
    const stateTransition = new DataContractCreateTransition(rawStateTransition);

    result.merge(
      await validateStateTransitionSignature(stateTransition, rawDataContract.ownerId),
    );

    return result;
  }

  return validateDataContractCreateTransitionStructure;
}

module.exports = validateDataContractCreateTransitionStructureFactory;
