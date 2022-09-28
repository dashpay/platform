const identityCreateTransitionSchema = require('../../../../../../schema/identity/stateTransition/identityCreate.json');

const convertBuffersToArrays = require('../../../../../util/convertBuffersToArrays');

/**
 * @param {JsonSchemaValidator} jsonSchemaValidator
 * @param {validatePublicKeys} validatePublicKeys
 * @param {
 *  validateRequiredPurposeAndSecurityLevel
 * } validateRequiredPurposeAndSecurityLevel
 * @param {Object.<number, Function>} proofValidationFunctionsByType
 * @param {validateProtocolVersion} validateProtocolVersion
 * @param {validatePublicKeySignatures} validatePublicKeySignatures
 *
 * @return {validateIdentityCreateTransitionBasic}
 */
function validateIdentityCreateTransitionBasicFactory(
  jsonSchemaValidator,
  validatePublicKeys,
  validateRequiredPurposeAndSecurityLevel,
  proofValidationFunctionsByType,
  validateProtocolVersion,
  validatePublicKeySignatures,
) {
  /**
   * @typedef validateIdentityCreateTransitionBasic
   * @param {RawIdentityCreateTransition} rawStateTransition
   * @param {StateTransitionExecutionContext} executionContext
   * @return {Promise<ValidationResult>}
   */
  // eslint-disable-next-line no-unused-vars
  async function validateIdentityCreateTransitionBasic(rawStateTransition, executionContext) {
    // Validate state transition against JSON Schema
    const result = jsonSchemaValidator.validate(
      identityCreateTransitionSchema,
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

    result.merge(
      validatePublicKeys(rawStateTransition.publicKeys),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      await validatePublicKeySignatures(
        rawStateTransition,
        rawStateTransition.publicKeys,
        executionContext,
      ),
    );

    if (!result.isValid()) {
      return result;
    }

    result.merge(
      validateRequiredPurposeAndSecurityLevel(rawStateTransition.publicKeys),
    );

    if (!result.isValid()) {
      return result;
    }

    const proofValidationFunction = proofValidationFunctionsByType[
      rawStateTransition.assetLockProof.type
    ];

    const assetLockProofValidationResult = await proofValidationFunction(
      rawStateTransition.assetLockProof,
      executionContext,
    );

    result.merge(
      assetLockProofValidationResult,
    );

    return result;
  }

  return validateIdentityCreateTransitionBasic;
}

module.exports = validateIdentityCreateTransitionBasicFactory;
