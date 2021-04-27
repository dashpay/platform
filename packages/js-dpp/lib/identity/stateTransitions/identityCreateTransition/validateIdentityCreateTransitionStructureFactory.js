const identityCreateTransitionSchema = require('../../../../schema/identity/stateTransition/identityCreate');

const convertBuffersToArrays = require('../../../util/convertBuffersToArrays');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validatePublicKeys} validatePublicKeys
 * @param {validateSignatureAgainstAssetLockPublicKey} validateSignatureAgainstAssetLockPublicKey
 * @param {Object.<number, Function>} proofValidationFunctionsByType
 * @return {validateIdentityCreateTransitionStructure}
 */
function validateIdentityCreateTransitionStructureFactory(
  jsonSchemaValidator,
  validatePublicKeys,
  validateSignatureAgainstAssetLockPublicKey,
  proofValidationFunctionsByType,
) {
  /**
   * @typedef validateIdentityCreateTransitionStructure
   * @param {RawIdentityCreateTransition} rawStateTransition
   * @return {Promise<ValidationResult>}
   */
  async function validateIdentityCreateTransitionStructure(rawStateTransition) {
    // Validate state transition against JSON Schema
    const result = jsonSchemaValidator.validate(
      identityCreateTransitionSchema,
      convertBuffersToArrays(rawStateTransition),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      validatePublicKeys(rawStateTransition.publicKeys),
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

    if (!result.isValid()) {
      return result;
    }

    const publicKeyHash = assetLockProofValidationResult.getData();

    result.merge(
      await validateSignatureAgainstAssetLockPublicKey(rawStateTransition, publicKeyHash),
    );

    return result;
  }

  return validateIdentityCreateTransitionStructure;
}

module.exports = validateIdentityCreateTransitionStructureFactory;
