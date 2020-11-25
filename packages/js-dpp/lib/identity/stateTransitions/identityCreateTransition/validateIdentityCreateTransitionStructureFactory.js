const identityCreateTransitionSchema = require('../../../../schema/identity/stateTransition/identityCreate');

const convertBuffersToArrays = require('../../../util/convertBuffersToArrays');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validatePublicKeys} validatePublicKeys
 * @param {validateAssetLockStructure} validateAssetLockStructure
 * @param {validateSignatureAgainstAssetLockPublicKey} validateSignatureAgainstAssetLockPublicKey
 * @return {validateIdentityCreateTransitionStructure}
 */
function validateIdentityCreateTransitionStructureFactory(
  jsonSchemaValidator,
  validatePublicKeys,
  validateAssetLockStructure,
  validateSignatureAgainstAssetLockPublicKey,
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

    const assetLockValidationResult = await validateAssetLockStructure(
      rawStateTransition.assetLock,
    );

    result.merge(assetLockValidationResult);

    if (!result.isValid()) {
      return result;
    }

    const publicKeyHash = assetLockValidationResult.getData();

    result.merge(
      await validateSignatureAgainstAssetLockPublicKey(rawStateTransition, publicKeyHash),
    );

    return result;
  }

  return validateIdentityCreateTransitionStructure;
}

module.exports = validateIdentityCreateTransitionStructureFactory;
