const identityTopUpTransitionSchema = require('../../../../schema/identity/stateTransition/identityTopUp.json');

const convertBuffersToArrays = require('../../../util/convertBuffersToArrays');
const Identifier = require('../../../identifier/Identifier');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validateIdentityExistence} validateIdentityExistence
 * @param {validateSignatureAgainstAssetLockPublicKey} validateSignatureAgainstAssetLockPublicKey
 * @param {Object.<number, Function>} proofValidationFunctionsByType
 * @return {validateIdentityTopUpTransitionStructure}
 */
function validateIdentityTopUpTransitionStructureFactory(
  jsonSchemaValidator,
  validateIdentityExistence,
  validateSignatureAgainstAssetLockPublicKey,
  proofValidationFunctionsByType,
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

  return validateIdentityTopUpTransitionStructure;
}

module.exports = validateIdentityTopUpTransitionStructureFactory;
