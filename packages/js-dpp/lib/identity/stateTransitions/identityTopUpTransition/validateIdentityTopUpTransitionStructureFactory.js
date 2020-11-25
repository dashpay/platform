const identityTopUpTransitionSchema = require('../../../../schema/identity/stateTransition/identityTopUp.json');

const convertBuffersToArrays = require('../../../util/convertBuffersToArrays');
const Identifier = require('../../../identifier/Identifier');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validateIdentityExistence} validateIdentityExistence
 * @param {validateAssetLockStructure} validateAssetLockStructure
 * @param {validateSignatureAgainstAssetLockPublicKey} validateSignatureAgainstAssetLockPublicKey
 * @return {validateIdentityTopUpTransitionStructure}
 */
function validateIdentityTopUpTransitionStructureFactory(
  jsonSchemaValidator,
  validateIdentityExistence,
  validateAssetLockStructure,
  validateSignatureAgainstAssetLockPublicKey,
) {
  /**
   * @typedef {validateIdentityTopUpTransitionStructure}
   * @param {RawIdentityTopUpTransition} rawStateTransition
   * @return {ValidationResult}
   */
  async function validateIdentityTopUpTransitionStructure(rawStateTransition) {
    const result = jsonSchemaValidator.validate(
      identityTopUpTransitionSchema,
      convertBuffersToArrays(rawStateTransition),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      await validateIdentityExistence(
        new Identifier(rawStateTransition.identityId),
      ),
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

  return validateIdentityTopUpTransitionStructure;
}

module.exports = validateIdentityTopUpTransitionStructureFactory;
