const identityCreateTransitionSchema = require('../../../../../../schema/identity/stateTransition/identityCreate.json');

const convertBuffersToArrays = require('../../../../../util/convertBuffersToArrays');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validatePublicKeys} validatePublicKeys
 * @param {Object.<number, Function>} proofValidationFunctionsByType
 * @return {validateIdentityCreateTransitionBasic}
 */
function validateIdentityCreateTransitionBasicFactory(
  jsonSchemaValidator,
  validatePublicKeys,
  proofValidationFunctionsByType,
) {
  /**
   * @typedef validateIdentityCreateTransitionBasic
   * @param {RawIdentityCreateTransition} rawStateTransition
   * @return {Promise<ValidationResult>}
   */
  async function validateIdentityCreateTransitionBasic(rawStateTransition) {
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

    return result;
  }

  return validateIdentityCreateTransitionBasic;
}

module.exports = validateIdentityCreateTransitionBasicFactory;
