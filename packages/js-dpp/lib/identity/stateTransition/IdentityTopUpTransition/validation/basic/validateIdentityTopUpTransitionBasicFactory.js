const identityTopUpTransitionSchema = require('../../../../../../schema/identity/stateTransition/identityTopUp.json');

const convertBuffersToArrays = require('../../../../../util/convertBuffersToArrays');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {Object.<number, Function>} proofValidationFunctionsByType
 * @param {validateProtocolVersion} validateProtocolVersion
 *
 * @return {validateIdentityTopUpTransitionBasic}
 */
function validateIdentityTopUpTransitionBasicFactory(
  jsonSchemaValidator,
  proofValidationFunctionsByType,
  validateProtocolVersion,
) {
  /**
   * @typedef {validateIdentityTopUpTransitionBasic}
   * @param {RawIdentityTopUpTransition} rawStateTransition
   * @return {ValidationResult}
   */
  async function validateIdentityTopUpTransitionBasic(rawStateTransition) {
    const result = jsonSchemaValidator.validate(
      identityTopUpTransitionSchema,
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

    const proofValidationFunction = proofValidationFunctionsByType[
      rawStateTransition.assetLockProof.type
    ];

    const assetLockProofValidationResult = await proofValidationFunction(
      rawStateTransition.assetLockProof,
    );

    result.merge(
      assetLockProofValidationResult,
    );

    return result;
  }

  return validateIdentityTopUpTransitionBasic;
}

module.exports = validateIdentityTopUpTransitionBasicFactory;
