const convertBuffersToArrays = require('../../../../../util/convertBuffersToArrays');

const dataContractUpdateTransitionSchema = require('../../../../../../schema/dataContract/stateTransition/dataContractUpdate.json');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validateDataContract} validateDataContract
 * @param {validateProtocolVersion} validateProtocolVersion
 *
 * @return {validateDataContractUpdateTransitionBasic}
 */
function validateDataContractUpdateTransitionBasicFactory(
  jsonSchemaValidator,
  validateDataContract,
  validateProtocolVersion,
) {
  /**
   * @typedef validateDataContractUpdateTransitionBasic
   * @param {RawDataContractUpdateTransition} rawStateTransition
   * @return {ValidationResult}
   */
  async function validateDataContractUpdateTransitionBasic(rawStateTransition) {
    const result = jsonSchemaValidator.validate(
      dataContractUpdateTransitionSchema,
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

    return result;
  }

  return validateDataContractUpdateTransitionBasic;
}

module.exports = validateDataContractUpdateTransitionBasicFactory;
