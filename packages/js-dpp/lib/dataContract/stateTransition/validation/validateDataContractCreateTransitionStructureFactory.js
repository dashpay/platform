const DataContractCreateTransition = require('../DataContractCreateTransition');

const InvalidDataContractEntropyError = require('../../../errors/InvalidDataContractEntropyError');

const InvalidDataContractIdError = require('../../../errors/InvalidDataContractIdError');
const encodeObjectProperties = require('../../../util/encoding/encodeObjectProperties');

const entropy = require('../../../util/entropy');

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
    // Validate state transition against JSON Schema
    const jsonStateTransition = encodeObjectProperties(
      rawStateTransition,
      DataContractCreateTransition.ENCODED_PROPERTIES,
    );

    const result = jsonSchemaValidator.validate(
      dataContractCreateTransitionSchema,
      jsonStateTransition,
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

    // Validate entropy
    if (!entropy.validate(rawStateTransition.entropy)) {
      result.addError(
        new InvalidDataContractEntropyError(rawDataContract),
      );

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
