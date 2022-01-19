const InvalidDataContractIdError = require('../../../../../errors/consensus/basic/dataContract/InvalidDataContractIdError');

const generateDataContractId = require('../../../../generateDataContractId');

const convertBuffersToArrays = require('../../../../../util/convertBuffersToArrays');

const dataContractCreateTransitionSchema = require('../../../../../../schema/dataContract/stateTransition/dataContractCreate.json');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validateDataContract} validateDataContract
 * @param {validateProtocolVersion} validateProtocolVersion
 *
 * @return {validateDataContractCreateTransitionBasic}
 */
function validateDataContractCreateTransitionBasicFactory(
  jsonSchemaValidator,
  validateDataContract,
  validateProtocolVersion,
) {
  /**
   * @typedef validateDataContractCreateTransitionBasic
   * @param {RawDataContractCreateTransition} rawStateTransition
   * @return {ValidationResult}
   */
  async function validateDataContractCreateTransitionBasic(rawStateTransition) {
    const result = jsonSchemaValidator.validate(
      dataContractCreateTransitionSchema,
      convertBuffersToArrays(rawStateTransition),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      validateProtocolVersion(rawStateTransition.protocolVersion),
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
    const generatedId = generateDataContractId(rawDataContract.ownerId, rawStateTransition.entropy);

    if (!generatedId.equals(rawDataContract.$id)) {
      result.addError(
        new InvalidDataContractIdError(generatedId, rawDataContract.$id),
      );
    }

    return result;
  }

  return validateDataContractCreateTransitionBasic;
}

module.exports = validateDataContractCreateTransitionBasicFactory;
